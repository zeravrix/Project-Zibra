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
pub async fn set_manga_status(state: State<'_, AppState>, manga_id: String, status: String) -> Result<(), String> {
    state.library.set_manga_status(&manga_id, &status).map_err(|e| e.to_string())
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
    pinned_ids: Option<Vec<String>>,
    nsfw_enabled: Option<bool>,
) -> Result<Vec<Manga>, String> {
    let settings = state.settings_store.load();
    let nsfw = nsfw_enabled.unwrap_or(settings.nsfw_enabled);
    let pinned: HashSet<String> = if let Some(p) = pinned_ids {
        p.into_iter().collect()
    } else {
        settings.pinned_source_ids.into_iter().collect()
    };

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
    // Preserve original cover if details returns empty or a generic site logo/icon
    if let Some(ref orig_cov) = manga.cover_url {
        if !orig_cov.is_empty() {
            let is_valid_new_cover = details.cover_url.as_ref().map_or(false, |c| {
                !c.is_empty() 
                    && !c.to_lowercase().contains("logo") 
                    && !c.to_lowercase().contains("favicon")
                    && !c.to_lowercase().contains("header")
            });
            if !is_valid_new_cover {
                details.cover_url = Some(orig_cov.clone());
            }
        }
    }
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

pub fn save_source_cookies_inner(
    state: &AppState,
    source_id: &str,
    cookie: &str,
    base_url: Option<&str>,
) -> Result<(), String> {
    let mut settings = state.settings_store.load();
    let trimmed_cookie = cookie.trim();
    if trimmed_cookie.is_empty() {
        settings.custom_source_cookies.remove(source_id);
    } else {
        settings.custom_source_cookies.insert(source_id.to_string(), trimmed_cookie.to_string());
    }
    state.settings_store.save(&settings).map_err(|e| e.to_string())?;

    let target_url = base_url
        .and_then(|u| u.parse::<url::Url>().ok())
        .or_else(|| {
            state.registry.get(source_id).and_then(|s| s.base_url().parse::<url::Url>().ok())
        });

    if let Some(parsed) = target_url {
        for part in trimmed_cookie.split(';') {
            let item = part.trim();
            if !item.is_empty() {
                state.cookie_jar.add_cookie_str(item, &parsed);
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn open_cloudflare_solver(
    app: tauri::AppHandle,
    url: String,
    title: Option<String>,
    source_id: Option<String>,
) -> Result<(), String> {
    use tauri::Manager;
    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    if let Some(window) = app.get_webview_window("cloudflare-solver") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.navigate(parsed_url);
        return Ok(());
    }

    let win_title = title.unwrap_or_else(|| "Cloudflare Captcha Solver — Zibrax".to_string());
    let app_handle = app.clone();
    let src_id = source_id.unwrap_or_else(|| {
        parsed_url.host_str().unwrap_or("unknown").to_string()
    });
    let base_url_str = url.clone();

    let init_script = r#"
        (function() {
            function initZibraPill() {
                if (document.getElementById('zibra-cf-pill')) return;
                
                const pill = document.createElement('div');
                pill.id = 'zibra-cf-pill';
                pill.style.cssText = 'position:fixed;bottom:20px;right:20px;z-index:2147483647;background:rgba(35,33,44,0.94);backdrop-filter:blur(14px);-webkit-backdrop-filter:blur(14px);color:#f4f3f7;border:1px solid rgba(255,255,255,0.15);border-radius:9999px;display:flex;align-items:center;gap:10px;padding:8px 16px;box-shadow:0 12px 32px rgba(0,0,0,0.65);font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;user-select:none;font-size:12px;';

                const label = document.createElement('span');
                label.innerText = 'Cloudflare Verification';
                label.style.cssText = 'font-weight:600;color:#c4b5fd;';

                const reloadBtn = document.createElement('button');
                reloadBtn.innerText = '↻';
                reloadBtn.title = 'Reload page';
                reloadBtn.style.cssText = 'background:rgba(255,255,255,0.08);color:#f4f3f7;border:none;border-radius:50%;width:24px;height:24px;cursor:pointer;display:flex;align-items:center;justify-content:center;font-size:13px;';
                reloadBtn.onclick = () => window.location.reload();

                const syncBtn = document.createElement('button');
                syncBtn.id = 'zibra-sync-btn';
                syncBtn.innerText = 'Done & Sync';
                syncBtn.style.cssText = 'background:#4f46e5;color:#ffffff;border:none;border-radius:9999px;padding:5px 14px;font-size:11px;font-weight:600;cursor:pointer;transition:all 0.2s;';

                let synced = false;
                function doSync(auto) {
                    if (synced) return;
                    synced = true;
                    syncBtn.innerText = auto ? '✓ Solved!' : 'Syncing...';
                    syncBtn.style.background = '#10b981';
                    const c = document.cookie || '';
                    const ua = navigator.userAgent || '';
                    setTimeout(() => {
                        window.location.href = 'https://zibra.local/sync-cf?c=' + encodeURIComponent(c) + '&ua=' + encodeURIComponent(ua);
                    }, 400);
                }

                syncBtn.onclick = () => doSync(false);

                pill.appendChild(label);
                pill.appendChild(reloadBtn);
                pill.appendChild(syncBtn);

                if (document.body) {
                    document.body.appendChild(pill);
                } else if (document.documentElement) {
                    document.documentElement.appendChild(pill);
                }

                const checkInterval = setInterval(() => {
                    if (synced) {
                        clearInterval(checkInterval);
                        return;
                    }
                    const c = document.cookie || '';
                    if (c.includes('cf_clearance')) {
                        clearInterval(checkInterval);
                        doSync(true);
                    }
                }, 600);
            }

            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', initZibraPill);
            } else {
                initZibraPill();
            }
            setTimeout(initZibraPill, 1000);
            setTimeout(initZibraPill, 2500);
        })();
    "#;

    let _ = tauri::WebviewWindowBuilder::new(
        &app,
        "cloudflare-solver",
        tauri::WebviewUrl::External(parsed_url),
    )
    .title(win_title)
    .inner_size(520.0, 700.0)
    .center()
    .resizable(true)
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
    .initialization_script(init_script)
    .on_navigation(move |nav_url| {
        if nav_url.host_str() == Some("zibra.local") && nav_url.path() == "/sync-cf" {
            let pairs: std::collections::HashMap<_, _> = nav_url.query_pairs().into_owned().collect();
            let cookie_str = pairs.get("c").cloned().unwrap_or_default();
            let app_h = app_handle.clone();
            let s_id = src_id.clone();
            let b_url = base_url_str.clone();

            tauri::async_runtime::spawn(async move {
                let state = app_h.state::<AppState>();
                let _ = save_source_cookies_inner(&state, &s_id, &cookie_str, Some(&b_url));
                if let Some(win) = app_h.get_webview_window("cloudflare-solver") {
                    let _ = win.close();
                }
                use tauri::Emitter;
                let _ = app_h.emit("cloudflare-solved", serde_json::json!({
                    "source_id": s_id,
                    "cookies": cookie_str,
                }));
            });
            return false;
        }
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn save_source_cookies(
    state: State<'_, AppState>,
    source_id: String,
    cookie: String,
    base_url: Option<String>,
) -> Result<(), String> {
    save_source_cookies_inner(&state, &source_id, &cookie, base_url.as_deref())
}

#[tauri::command]
pub async fn clear_source_cookies(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<(), String> {
    save_source_cookies_inner(&state, &source_id, "", None)
}

#[tauri::command]
pub async fn get_source_cookies(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<String, String> {
    let settings = state.settings_store.load();
    Ok(settings.custom_source_cookies.get(&source_id).cloned().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_source_cookies_parsing_and_persistence() {
        let temp_dir = std::env::temp_dir().join(format!(
            "zibra_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);

        let settings_store = std::sync::Arc::new(zibra_core::AppSettingsStore::new(&temp_dir));
        let library = std::sync::Arc::new(zibra_core::LibraryStore::new(&temp_dir).unwrap());
        let cookie_jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
        let http_client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .build()
            .unwrap();
        let registry = std::sync::Arc::new(zibra_sources::LiveSourceRegistry::new_with_client(http_client.clone()));
        let downloader = std::sync::Arc::new(zibra_core::DesktopDownloader::new(&temp_dir, 2));
        let reader_pipeline = std::sync::Arc::new(zibra_reader::ReaderPagePipeline::new(&temp_dir, 128));
        let cover_cache = std::sync::Arc::new(zibra_reader::CoverImageCache::new(&temp_dir));

        let state = AppState {
            app_dir: temp_dir.clone(),
            settings_store: settings_store.clone(),
            library,
            registry,
            downloader,
            reader_pipeline,
            cover_cache,
            http_client,
            cookie_jar: cookie_jar.clone(),
        };

        let cookie_str = "cf_clearance=mock_cf_token_12345; __cf_bm=mock_bm_67890; csrftoken=abc";
        let res = save_source_cookies_inner(&state, "MANGADEX", cookie_str, Some("https://api.mangadex.org"));
        assert!(res.is_ok());

        // Check settings persisted
        let loaded = settings_store.load();
        assert_eq!(loaded.custom_source_cookies.get("MANGADEX").unwrap(), cookie_str);

        // Check cookie jar holds the cookies
        use reqwest::cookie::CookieStore;
        let url = "https://api.mangadex.org/manga".parse::<url::Url>().unwrap();
        let header = cookie_jar.cookies(&url).unwrap();
        let header_str = header.to_str().unwrap();
        assert!(header_str.contains("cf_clearance=mock_cf_token_12345"));
        assert!(header_str.contains("__cf_bm=mock_bm_67890"));
        assert!(header_str.contains("csrftoken=abc"));

        // Clear cookies
        let clear_res = save_source_cookies_inner(&state, "MANGADEX", "", None);
        assert!(clear_res.is_ok());
        let after_clear = settings_store.load();
        assert!(!after_clear.custom_source_cookies.contains_key("MANGADEX"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

