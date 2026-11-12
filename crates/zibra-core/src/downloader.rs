use crate::archive::{ArchiveImage, ChapterArchive};
use crate::models::{Chapter, DownloadedChapter, Manga, MangaPage};
use chrono::Utc;
use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Downloading { completed_pages: usize, total_pages: usize },
    Completed { archive_path: PathBuf },
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct DownloadJobState {
    pub manga: Manga,
    pub chapter: Chapter,
    pub status: DownloadStatus,
}

#[derive(Clone)]
pub struct DesktopDownloader {
    downloads_dir: PathBuf,
    client: reqwest::Client,
    jobs: Arc<RwLock<Vec<DownloadJobState>>>,
    semaphore: Arc<Semaphore>,
}

impl DesktopDownloader {
    pub fn new(app_dir: &Path, max_concurrent: usize) -> Self {
        let downloads_dir = app_dir.join("downloads");
        let _ = fs::create_dir_all(&downloads_dir);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(45))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self {
            downloads_dir,
            client,
            jobs: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    pub fn jobs(&self) -> Vec<DownloadJobState> {
        self.jobs.read().clone()
    }

    pub fn list_downloaded(&self) -> Vec<DownloadedChapter> {
        let mut results = Vec::new();
        let Ok(manga_dirs) = fs::read_dir(&self.downloads_dir) else {
            return results;
        };
        for m_entry in manga_dirs.flatten() {
            if !m_entry.path().is_dir() {
                continue;
            }
            let manga_title = m_entry.file_name().to_string_lossy().to_string();
            if let Ok(ch_entries) = fs::read_dir(m_entry.path()) {
                for ch_entry in ch_entries.flatten() {
                    let path = ch_entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("cbz") {
                        let ch_title = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Chapter".to_string());
                        let page_count = ChapterArchive::image_entries(&path)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        let downloaded_at = ch_entry
                            .metadata()
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_millis() as i64)
                            .unwrap_or(0);
                        results.push(DownloadedChapter {
                            manga_id: manga_title.clone(),
                            source_id: "LOCAL".to_string(),
                            manga_title: manga_title.clone(),
                            manga_cover_url: None,
                            chapter_id: path.to_string_lossy().to_string(),
                            chapter_title: ch_title,
                            number: None,
                            directory: m_entry.path(),
                            page_count,
                            downloaded_at,
                            archive_path: Some(path),
                        });
                    }
                }
            }
        }
        results.sort_by(|a, b| b.downloaded_at.cmp(&a.downloaded_at));
        results
    }

    pub async fn download_chapter(
        &self,
        manga: Manga,
        chapter: Chapter,
        pages: Vec<MangaPage>,
    ) -> anyhow::Result<PathBuf> {
        if pages.is_empty() {
            anyhow::bail!("No pages returned for chapter {}", chapter.title);
        }

        {
            let mut jobs = self.jobs.write();
            jobs.retain(|j| j.chapter.id != chapter.id);
            jobs.push(DownloadJobState {
                manga: manga.clone(),
                chapter: chapter.clone(),
                status: DownloadStatus::Downloading {
                    completed_pages: 0,
                    total_pages: pages.len(),
                },
            });
        }

        let safe_manga = sanitize_filename(&manga.title);
        let safe_chapter = sanitize_filename(&chapter.title);
        let manga_dir = self.downloads_dir.join(&safe_manga);
        let temp_dir = manga_dir.join(format!(".tmp_{}", safe_chapter));
        fs::create_dir_all(&temp_dir)?;

        let total = pages.len();
        let mut archive_images = Vec::with_capacity(total);

        for (idx, page) in pages.into_iter().enumerate() {
            let _permit = self.semaphore.acquire().await?;
            let bytes = self.fetch_page_with_retry(&page).await?;

            if is_html_or_challenge(&bytes) {
                self.update_status(
                    &chapter.id,
                    DownloadStatus::Failed {
                        error: format!("Page {} returned an HTML/Cloudflare block page", idx + 1),
                    },
                );
                anyhow::bail!("Page {} returned an HTML challenge instead of image bytes", idx + 1);
            }

            let ext = detect_image_ext(&bytes);
            let entry_name = format!("{:04}.{}", idx + 1, ext);
            let file_path = temp_dir.join(&entry_name);
            fs::write(&file_path, &bytes)?;

            archive_images.push(ArchiveImage {
                entry_name,
                file_path,
            });

            self.update_status(
                &chapter.id,
                DownloadStatus::Downloading {
                    completed_pages: idx + 1,
                    total_pages: total,
                },
            );
        }

        let target_cbz = manga_dir.join(format!("{}.cbz", safe_chapter));
        let comic_info = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<ComicInfo>\n  <Series>{}</Series>\n  <Title>{}</Title>\n  <PageCount>{}</PageCount>\n</ComicInfo>",
            xml_escape(&manga.title),
            xml_escape(&chapter.title),
            total
        );
        let manifest = serde_json::json!({
            "mangaId": manga.id,
            "mangaTitle": manga.title,
            "chapterId": chapter.id,
            "chapterTitle": chapter.title,
            "downloadedAt": Utc::now().timestamp_millis(),
        })
        .to_string();

        ChapterArchive::write(&target_cbz, &archive_images, Some(&comic_info), Some(&manifest))?;
        let _ = fs::remove_dir_all(&temp_dir);

        self.update_status(
            &chapter.id,
            DownloadStatus::Completed {
                archive_path: target_cbz.clone(),
            },
        );

        Ok(target_cbz)
    }

    async fn fetch_page_with_retry(&self, page: &MangaPage) -> anyhow::Result<Vec<u8>> {
        let mut last_err = None;
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
            }
            let mut req = self.client.get(&page.image_url);
            if let Some(ref referer) = page.referer {
                req = req.header("Referer", referer);
            }
            for (k, v) in &page.headers {
                req = req.header(k, v);
            }
            match req.send().await {
                Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                    Ok(b) if b.len() >= 64 => return Ok(b.to_vec()),
                    Ok(_) => last_err = Some(anyhow::anyhow!("Image payload too small")),
                    Err(e) => last_err = Some(e.into()),
                },
                Ok(resp) => {
                    last_err = Some(anyhow::anyhow!("HTTP {} for {}", resp.status(), page.image_url));
                }
                Err(e) => last_err = Some(e.into()),
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Failed to download page")))
    }

    fn update_status(&self, chapter_id: &str, status: DownloadStatus) {
        let mut jobs = self.jobs.write();
        if let Some(job) = jobs.iter_mut().find(|j| j.chapter.id == chapter_id) {
            job.status = status;
        }
    }
}

pub fn is_html_or_challenge(bytes: &[u8]) -> bool {
    let head_len = bytes.len().min(512);
    let head = String::from_utf8_lossy(&bytes[..head_len]).trim_start().to_ascii_lowercase();
    head.starts_with("<!doctype html") || head.starts_with("<html") || head.contains("cloudflare")
}

pub fn detect_image_ext(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "png"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else {
        "jpg"
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.chars().take(100).collect()
    }
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
