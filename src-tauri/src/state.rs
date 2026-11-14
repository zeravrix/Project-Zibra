use std::path::PathBuf;
use std::sync::Arc;
use reqwest::cookie::Jar;
use zibra_core::{AppSettingsStore, DesktopDownloader, LibraryStore};
use zibra_reader::{CoverImageCache, ReaderPagePipeline};
use zibra_sources::LiveSourceRegistry;

#[derive(Clone)]
pub struct AppState {
    pub app_dir: PathBuf,
    pub settings_store: Arc<AppSettingsStore>,
    pub library: Arc<LibraryStore>,
    pub registry: Arc<LiveSourceRegistry>,
    pub downloader: Arc<DesktopDownloader>,
    pub reader_pipeline: Arc<ReaderPagePipeline>,
    pub cover_cache: Arc<CoverImageCache>,
    pub http_client: reqwest::Client,
    pub cookie_jar: Arc<Jar>,
}

impl AppState {
    pub fn new() -> Self {
        let app_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".zibra");
        let _ = std::fs::create_dir_all(&app_dir);

        let settings_store = Arc::new(AppSettingsStore::new(&app_dir));
        let settings = settings_store.load();

        let library = Arc::new(LibraryStore::new(&app_dir).expect("Failed to initialize SQLite library"));
        let cookie_jar = Arc::new(Jar::default());

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .cookie_provider(cookie_jar.clone())
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        let registry = Arc::new(LiveSourceRegistry::new_with_client(http_client.clone()));
        let downloader = Arc::new(DesktopDownloader::new(&app_dir, settings.max_concurrent_downloads));
        let reader_pipeline = Arc::new(ReaderPagePipeline::new(&app_dir, settings.ram_cache_budget_mb));
        let cover_cache = Arc::new(CoverImageCache::new(&app_dir));

        for (source_id_or_url, cookie_str) in &settings.custom_source_cookies {
            let target_url = source_id_or_url
                .parse::<url::Url>()
                .ok()
                .or_else(|| {
                    registry.get(source_id_or_url).and_then(|s| s.base_url().parse::<url::Url>().ok())
                });

            if let Some(url) = target_url {
                for part in cookie_str.split(';') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        cookie_jar.add_cookie_str(trimmed, &url);
                    }
                }
            }
        }

        Self {
            app_dir,
            settings_store,
            library,
            registry,
            downloader,
            reader_pipeline,
            cover_cache,
            http_client,
            cookie_jar,
        }
    }
}
