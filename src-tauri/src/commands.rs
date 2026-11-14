use crate::state::AppState;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;
use zibra_core::{AppSettings, CatalogSortMode, Chapter, DownloadStatus, Manga, MangaPage, ReadingProgress, SourceTag};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub base_url: String,
    pub icon_url: String,
    pub priority_rank: u32,
    pub tags: Vec<String>,
    pub is_nsfw: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MangaPageResultDto {
    pub items: Vec<Manga>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItemDto {
    pub chapter_id: String,
    pub manga_title: String,
    pub chapter_title: String,
    pub status: String,
    pub progress: f32,
    pub total_pages: usize,
    pub downloaded_pages: usize,
}

#[tauri::command]
pub async fn get_history(state: State<'_, AppState>) -> Result<Vec<Manga>, String> {
    Ok(state.library.list_history())
}

#[tauri::command]
pub async fn get_favorites(state: State<'_, AppState>) -> Result<Vec<Manga>, String> {
    Ok(state.library.list_favorites())
}

#[tauri::command]
pub async fn toggle_favorite(state: State<'_, AppState>, manga: Manga) -> Result<bool, String> {
    Ok(state.library.toggle_favorite(&manga))
}

#[tauri::command]
pub async fn is_favorite(state: State<'_, AppState>, manga_id: String) -> Result<bool, String> {
    Ok(state.library.is_favorite(&manga_id))
}

#[tauri::command]
pub async fn remove_history(state: State<'_, AppState>, manga_id: String) -> Result<(), String> {
    state.library.remove_history(&manga_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_history(state: State<'_, AppState>, manga: Manga) -> Result<(), String> {
    state.library.record_history(&manga);
    Ok(())
}

#[tauri::command]
pub async fn get_reading_progress(state: State<'_, AppState>, manga_id: String) -> Result<Option<ReadingProgress>, String> {
    Ok(state.library.latest_progress(&manga_id))
}

#[tauri::command]
pub async fn save_reading_progress(state: State<'_, AppState>, progress: ReadingProgress) -> Result<(), String> {
    state.library.save_progress(&progress);
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings_store.load())
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    state.settings_store.save(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_sources(
    state: State<'_, AppState>,
    lang_filter: Option<String>,
    query: Option<String>,
    nsfw_enabled: Option<bool>,
) -> Result<Vec<SourceInfo>, String> {
    let settings = state.settings_store.load();
    let nsfw = nsfw_enabled.unwrap_or(settings.nsfw_enabled);
    let lang = lang_filter.unwrap_or_else(|| "ALL".to_string());
    let q = query.unwrap_or_default();

    let pinned: HashSet<String> = settings.pinned_source_ids.into_iter().collect();
    let tags: HashSet<SourceTag> = HashSet::new();

    let sources = state.registry.ordered_catalog(
        CatalogSortMode::RecommendedOrder,
        &pinned,
        &tags,
        &lang,
        &q,
        nsfw,
    );

    let list = sources
        .into_iter()
        .map(|s| SourceInfo {
            id: s.id().to_string(),
            name: s.name().to_string(),
            language: s.language().to_string(),
            base_url: s.base_url().to_string(),
            icon_url: s.icon_url(),
            priority_rank: s.priority_rank(),
            tags: s.tags().iter().map(|t| t.label().to_string()).collect(),
            is_nsfw: s.is_nsfw(),
            latency_ms: state.registry.latency_for(s.id()),
        })
        .collect();

    Ok(list)
}

#[tauri::command]
pub async fn fetch_source_page(
    state: State<'_, AppState>,
    source_id: String,
    page: u32,
    latest: bool,
    query: Option<String>,
    nsfw_enabled: Option<bool>,
) -> Result<MangaPageResultDto, String> {
    let settings = state.settings_store.load();
    let nsfw = nsfw_enabled.unwrap_or(settings.nsfw_enabled);

    let result = state
        .registry
        .fetch_from_source(&source_id, page, latest, query.as_deref(), nsfw)
        .await
        .map_err(|e| e.to_string())?;

    Ok(MangaPageResultDto {
        items: result.items,
        has_next_page: result.has_next_page,
    })
}

#[tauri::command]
pub async fn search_global(
    state: State<'_, AppState>,
    query: String,
    nsfw_enabled: Option<bool>,
) -> Result<Vec<Manga>, String> {
    let settings = state.settings_store.load();
    let nsfw = nsfw_enabled.unwrap_or(settings.nsfw_enabled);
    let pinned: HashSet<String> = settings.pinned_source_ids.into_iter().collect();

    let results = state.registry.search_global(&query, &pinned, nsfw).await;
    Ok(results)
}

#[tauri::command]
pub async fn get_manga_details(
    state: State<'_, AppState>,
    source_id: String,
    manga: Manga,
) -> Result<Manga, String> {
    let src = state
        .registry
        .get(&source_id)
        .ok_or_else(|| format!("Source '{}' not found", source_id))?;

    let mut details = src.details(&manga).await.map_err(|e| e.to_string())?;
    details.enrich_manhwa_tags();
    Ok(details)
}

#[tauri::command]
pub async fn get_chapters(
    state: State<'_, AppState>,
    source_id: String,
    manga: Manga,
) -> Result<Vec<Chapter>, String> {
    let src = state
        .registry
        .get(&source_id)
        .ok_or_else(|| format!("Source '{}' not found", source_id))?;

    src.chapters(&manga).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pages(
    state: State<'_, AppState>,
    source_id: String,
    chapter: Chapter,
) -> Result<Vec<MangaPage>, String> {
    let src = state
        .registry
        .get(&source_id)
        .ok_or_else(|| format!("Source '{}' not found", source_id))?;

    src.pages(&chapter).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_image_base64(
    state: State<'_, AppState>,
    url: String,
    referer: Option<String>,
) -> Result<String, String> {
    let mut req = state.http_client.get(&url);
    if let Some(ref r) = referer {
        req = req.header("Referer", r);
    }
    req = req.header("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8");

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", content_type, encoded))
}

#[tauri::command]
pub async fn get_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadItemDto>, String> {
    let jobs = state.downloader.jobs();
    let dtos = jobs
        .into_iter()
        .map(|item| {
            let (status_str, progress) = match &item.status {
                DownloadStatus::Queued => ("Queued".to_string(), 0.0),
                DownloadStatus::Downloading { completed_pages, total_pages } => {
                    let prog = if *total_pages > 0 {
                        *completed_pages as f32 / *total_pages as f32
                    } else {
                        0.0
                    };
                    ("Downloading".to_string(), prog)
                }
                DownloadStatus::Completed { .. } => ("Completed".to_string(), 1.0),
                DownloadStatus::Failed { error } => (format!("Failed: {}", error), 0.0),
            };
            DownloadItemDto {
                chapter_id: item.chapter.id.clone(),
                manga_title: item.manga.title.clone(),
                chapter_title: item.chapter.title.clone(),
                status: status_str,
                progress,
                total_pages: 0,
                downloaded_pages: 0,
            }
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
pub async fn queue_download(
    state: State<'_, AppState>,
    manga: Manga,
    chapter: Chapter,
) -> Result<(), String> {
    let src = state
        .registry
        .get(&chapter.source_id)
        .ok_or_else(|| format!("Source '{}' not found", chapter.source_id))?;

    let pages = src.pages(&chapter).await.map_err(|e| e.to_string())?;
    let dl = state.downloader.clone();
    tokio::spawn(async move {
        let _ = dl.download_chapter(manga, chapter, pages).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_download(_state: State<'_, AppState>, _chapter_id: String) -> Result<(), String> {
    Ok(())
}
