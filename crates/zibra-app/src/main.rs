#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod spring;
mod theme;
mod widgets;

use crate::spring::SpringDamper;
use crate::theme::{apply_kotatsu_visuals, GlassTokens};
use crate::widgets::{
    draw_remote_image, floating_continue_button, manga_cover_card, paint_native_icon, truncate_str,
    z_chip, z_icon_button, z_search_bar, CardAction, NativeIcon, HIGH_QUALITY_TEXTURE,
};
use eframe::egui::{
    self, Align, Align2, Color32, ColorImage, FontId, Layout, Margin, Pos2, Rect, RichText,
    Rounding, ScrollArea, Sense, Stroke, TextureHandle, Vec2,
};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use zibra_core::{
    AppLanguage, AppSettings, AppSettingsStore, AppTheme, BackupManager, CatalogSortMode, Chapter,
    ColorTheme, DesktopDownloader, DownloadStatus, LibraryStore, Manga, MangaPage,
    ReaderModePreference, ReadingProgress, SourceTag,
};
use zibra_reader::{CoverImageCache, PageLoadStatus, ReaderPagePipeline, SlicedPage};
use zibra_sources::LiveSourceRegistry;

const EMBEDDED_LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NavScreen {
    History,
    Favorites,
    Explore,
    Feed,
    Downloads,
    Settings,
}

impl NavScreen {
    fn title(&self) -> &'static str {
        match self {
            NavScreen::History => "History",
            NavScreen::Favorites => "Favourites",
            NavScreen::Explore => "Explore",
            NavScreen::Feed => "Updates & Feed",
            NavScreen::Downloads => "Downloads",
            NavScreen::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExploreSubMode {
    PinnedAndSearch,
    SourcesCatalog,
    SourceBrowse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShelfFilter {
    OnDevice,
    Reading,
    Completed,
    Favourites,
    NotInFavourites,
}

impl ShelfFilter {
    fn label(&self) -> &'static str {
        match self {
            ShelfFilter::OnDevice => "On device",
            ShelfFilter::Reading => "Reading",
            ShelfFilter::Completed => "Completed",
            ShelfFilter::Favourites => "Favourites",
            ShelfFilter::NotInFavourites => "Not in favourites",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ShelfFilter::OnDevice => "🖥",
            ShelfFilter::Reading => "▶",
            ShelfFilter::Completed => "✓",
            ShelfFilter::Favourites => "❤",
            ShelfFilter::NotInFavourites => "⊘",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsSection {
    Appearance,
    Reader,
    Sources,
    Downloads,
    Storage,
    LibraryBackup,
    System,
    About,
}

impl SettingsSection {
    const ALL: &'static [SettingsSection] = &[
        SettingsSection::Appearance,
        SettingsSection::Reader,
        SettingsSection::Sources,
        SettingsSection::Downloads,
        SettingsSection::Storage,
        SettingsSection::LibraryBackup,
        SettingsSection::System,
        SettingsSection::About,
    ];

    fn icon(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "🎨",
            SettingsSection::Reader => "📖",
            SettingsSection::Sources => "🌐",
            SettingsSection::Downloads => "⬇",
            SettingsSection::Storage => "🗄",
            SettingsSection::LibraryBackup => "🛡",
            SettingsSection::System => "⚡",
            SettingsSection::About => "ℹ",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Reader => "Reader",
            SettingsSection::Sources => "Sources & Catalog",
            SettingsSection::Downloads => "Downloads",
            SettingsSection::Storage => "Storage & Cache",
            SettingsSection::LibraryBackup => "Library & Backup",
            SettingsSection::System => "System & Network",
            SettingsSection::About => "About Zibrax",
        }
    }

    fn subtitle(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "13 Kotatsu themes, dark/light, AMOLED",
            SettingsSection::Reader => "Reading mode, webtoon seam trim, preload",
            SettingsSection::Sources => "1,647+ sources, languages, custom domains",
            SettingsSection::Downloads => "Parallel CBZ archiver, threads, folder",
            SettingsSection::Storage => "Image cache cleanup, reading history",
            SettingsSection::LibraryBackup => "Import/export backups, legacy migration",
            SettingsSection::System => "DNS-over-HTTPS, User-Agent, GPU pipeline",
            SettingsSection::About => "Version, architecture, license",
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedChapterSection {
    chapter: Chapter,
    chapter_index: usize,
    total_chapters: usize,
    pages: Vec<MangaPage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaderBgPreset {
    Amoled,
    Dark,
    Sepia,
    Light,
}

impl ReaderBgPreset {
    const ALL: &'static [ReaderBgPreset] = &[
        ReaderBgPreset::Amoled,
        ReaderBgPreset::Dark,
        ReaderBgPreset::Sepia,
        ReaderBgPreset::Light,
    ];

    fn label(&self) -> &'static str {
        match self {
            ReaderBgPreset::Amoled => "AMOLED",
            ReaderBgPreset::Dark => "Dark",
            ReaderBgPreset::Sepia => "Sepia",
            ReaderBgPreset::Light => "Light",
        }
    }

    fn color(&self) -> Color32 {
        match self {
            ReaderBgPreset::Amoled => Color32::from_rgb(0x00, 0x00, 0x00),
            ReaderBgPreset::Dark => Color32::from_rgb(0x10, 0x12, 0x18),
            ReaderBgPreset::Sepia => Color32::from_rgb(0x23, 0x1C, 0x16),
            ReaderBgPreset::Light => Color32::from_rgb(0xF6, 0xF7, 0xF9),
        }
    }
}

struct ZibraApp {
    rt: Arc<Runtime>,
    app_dir: PathBuf,
    settings_store: AppSettingsStore,
    settings: AppSettings,
    library: LibraryStore,
    registry: Arc<LiveSourceRegistry>,
    downloader: DesktopDownloader,
    reader_pipeline: ReaderPagePipeline,
    cover_cache: CoverImageCache,

    logo_texture: Option<TextureHandle>,
    cover_textures: HashMap<String, TextureHandle>,

    current_screen: NavScreen,
    rail_expanded: bool,
    explore_mode: ExploreSubMode,

    // Top Chrome & Shelf states
    shelf_search: String,
    active_shelf_filter: Option<ShelfFilter>,

    // Explore & Sources Catalog states
    explore_global_search: String,
    global_search_results: Arc<RwLock<Vec<Manga>>>,
    is_searching_global: Arc<RwLock<bool>>,

    catalog_search: String,
    catalog_lang: String,
    active_tag_filters: HashSet<SourceTag>,
    active_source_id: String,
    source_search: String,
    source_latest_mode: bool,
    source_page: u32,
    browse_items: Arc<RwLock<Vec<Manga>>>,
    is_loading_browse: Arc<RwLock<bool>>,
    status_banner: Arc<RwLock<String>>,

    // Details Screen states
    selected_manga: Option<Manga>,
    manga_chapters: Arc<RwLock<Vec<Chapter>>>,
    is_loading_chapters: Arc<RwLock<bool>>,
    details_newest_first: bool,
    details_synopsis_expanded: bool,
    details_chapter_search: String,

    // Reader states (including multi-chapter continuous infinite scroll)
    active_reader_chapter: Option<Chapter>,
    reader_pages: Arc<RwLock<Vec<MangaPage>>>,
    reader_loaded_chapters: Arc<RwLock<Vec<LoadedChapterSection>>>,
    reader_loading_next_chapter: Arc<RwLock<bool>>,
    is_loading_pages: Arc<RwLock<bool>>,
    reader_textures: HashMap<String, Vec<TextureHandle>>,
    reader_zoom: f32,
    reader_current_page: usize,
    reader_show_hud: bool,
    reader_show_settings_popup: bool,
    reader_bg: ReaderBgPreset,
    reader_page_gap: f32,
    reader_contrast: f32,
    reader_saturation: f32,
    reader_brightness: f32,
    reader_invert_colors: bool,
    #[allow(dead_code)]
    reader_scroll_spring: SpringDamper,
    pending_scroll_delta: f32,

    // Settings Screen states
    settings_section: SettingsSection,
    is_fullscreen: bool,
    bg_texture: Option<(String, TextureHandle)>,
    bg_avg_luminance: Option<f32>,
    custom_bg_input: String,
    custom_domain_key: String,
    custom_domain_val: String,
    custom_cookie_key: String,
    custom_cookie_val: String,
    backup_status_msg: String,
}

impl ZibraApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let rt = Arc::new(Runtime::new().expect("Failed to initialize Tokio runtime"));

        let app_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".zibra");
        let settings_store = AppSettingsStore::new(&app_dir);
        let settings = settings_store.load();
        let library = LibraryStore::new(&app_dir).expect("Failed to open SQLite library");
        let registry = Arc::new(LiveSourceRegistry::new());
        let reg_for_jvm = registry.clone();
        rt.spawn(async move {
            match reg_for_jvm.bind_jvm_host().await {
                Ok(count) if count > 0 => {
                    eprintln!("[ZIBRA] Successfully bound {} Kotatsu sources via JVM host bridge", count);
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[ZIBRA] JVM host Kotatsu source bridge note: {}", e);
                }
            }
        });
        let downloader = DesktopDownloader::new(&app_dir, settings.max_concurrent_downloads);
        let reader_pipeline = ReaderPagePipeline::new(&app_dir, settings.ram_cache_budget_mb);
        let cover_cache = CoverImageCache::new(&app_dir);

        let total_sources = registry.total_count();
        let reader_bg = if settings.is_amoled {
            ReaderBgPreset::Amoled
        } else {
            ReaderBgPreset::Dark
        };

        Self {
            rt,
            app_dir,
            settings_store,
            settings: settings.clone(),
            library,
            registry,
            downloader,
            reader_pipeline,
            cover_cache,
            logo_texture: None,
            cover_textures: HashMap::new(),
            current_screen: NavScreen::History,
            rail_expanded: false,
            explore_mode: ExploreSubMode::PinnedAndSearch,
            shelf_search: String::new(),
            active_shelf_filter: None,
            explore_global_search: String::new(),
            global_search_results: Arc::new(RwLock::new(Vec::new())),
            is_searching_global: Arc::new(RwLock::new(false)),
            catalog_search: String::new(),
            catalog_lang: settings.preferred_source_language.clone(),
            active_tag_filters: HashSet::new(),
            active_source_id: "MANGADEX".to_string(),
            source_search: String::new(),
            source_latest_mode: false,
            source_page: 1,
            browse_items: Arc::new(RwLock::new(Vec::new())),
            is_loading_browse: Arc::new(RwLock::new(false)),
            status_banner: Arc::new(RwLock::new(format!(
                "Ready • {} Native Sources Active",
                total_sources
            ))),
            selected_manga: None,
            manga_chapters: Arc::new(RwLock::new(Vec::new())),
            is_loading_chapters: Arc::new(RwLock::new(false)),
            details_newest_first: true,
            details_synopsis_expanded: false,
            details_chapter_search: String::new(),
            active_reader_chapter: None,
            reader_pages: Arc::new(RwLock::new(Vec::new())),
            reader_loaded_chapters: Arc::new(RwLock::new(Vec::new())),
            reader_loading_next_chapter: Arc::new(RwLock::new(false)),
            is_loading_pages: Arc::new(RwLock::new(false)),
            reader_textures: HashMap::new(),
            reader_zoom: 1.0,
            reader_current_page: 0,
            reader_show_hud: true,
            reader_show_settings_popup: false,
            reader_bg,
            reader_page_gap: 0.0,
            reader_contrast: 1.0,
            reader_saturation: 1.0,
            reader_brightness: 1.0,
            reader_invert_colors: false,
            reader_scroll_spring: SpringDamper::new(0.0),
            pending_scroll_delta: 0.0,
            settings_section: SettingsSection::Appearance,
            is_fullscreen: false,
            bg_texture: None,
            bg_avg_luminance: None,
            custom_bg_input: settings.custom_background_path.clone().unwrap_or_default(),
            custom_domain_key: String::new(),
            custom_domain_val: String::new(),
            custom_cookie_key: String::new(),
            custom_cookie_val: String::new(),
            backup_status_msg: String::new(),
        }
    }

    fn ensure_logo_texture(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_some() {
            return;
        }
        let custom_img = self
            .settings
            .custom_icon_path
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| image::load_from_memory(&bytes).ok());

        if let Some(img) = custom_img.or_else(|| image::load_from_memory(EMBEDDED_LOGO_PNG).ok()) {
            let rgba = img.to_rgba8();
            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
            let color_img = ColorImage::from_rgba_unmultiplied([w, h], &rgba.into_raw());
            self.logo_texture = Some(ctx.load_texture(
                "zibra_app_logo",
                color_img,
                HIGH_QUALITY_TEXTURE,
            ));
        }
    }

    fn ensure_custom_bg_texture(&mut self, ctx: &egui::Context) {
        let Some(ref path_str) = self.settings.custom_background_path else {
            self.bg_texture = None;
            self.bg_avg_luminance = None;
            return;
        };

        let needs_load = match &self.bg_texture {
            Some((p, _)) => p != path_str,
            None => true,
        };

        if needs_load {
            let path = std::path::Path::new(path_str);
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                    let raw = rgba.into_raw();

                    let total_px = w * h;
                    let step = (total_px / 4000).max(1) * 4;
                    let mut sum_lum = 0.0f32;
                    let mut count = 0usize;
                    for i in (0..raw.len()).step_by(step) {
                        if i + 2 < raw.len() {
                            let r = raw[i] as f32;
                            let g = raw[i + 1] as f32;
                            let b = raw[i + 2] as f32;
                            sum_lum += (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                            count += 1;
                        }
                    }
                    if count > 0 {
                        self.bg_avg_luminance = Some(sum_lum / (count as f32));
                    }

                    let color_img = ColorImage::from_rgba_unmultiplied([w, h], &raw);
                    let tex = ctx.load_texture("zibrax_custom_bg", color_img, HIGH_QUALITY_TEXTURE);
                    self.bg_texture = Some((path_str.clone(), tex));
                }
            }
        }
    }

    fn trigger_global_search(&mut self, ctx: &egui::Context) {
        let q = self.explore_global_search.trim().to_string();
        if q.is_empty() {
            self.global_search_results.write().clear();
            return;
        }
        let reg = self.registry.clone();
        let pinned = self.settings.pinned_source_ids.clone();
        let nsfw_enabled = self.settings.nsfw_enabled;
        let out_lock = self.global_search_results.clone();
        let loading_lock = self.is_searching_global.clone();
        let status_lock = self.status_banner.clone();
        let egui_ctx = ctx.clone();

        *loading_lock.write() = true;
        *status_lock.write() = format!("Searching across pinned sources for \"{}\"…", q);

        self.rt.spawn(async move {
            let results = reg.search_global(&q, &pinned, nsfw_enabled).await;
            *status_lock.write() = format!("Found {} titles across sources for \"{}\"", results.len(), q);
            *out_lock.write() = results;
            *loading_lock.write() = false;
            egui_ctx.request_repaint();
        });
    }

    fn trigger_source_fetch(
        &mut self,
        ctx: &egui::Context,
        source_id: String,
        page: u32,
        latest: bool,
        query: Option<String>,
    ) {
        self.active_source_id = source_id.clone();
        self.source_page = page;
        self.explore_mode = ExploreSubMode::SourceBrowse;

        let reg = self.registry.clone();
        let items_lock = self.browse_items.clone();
        let loading_lock = self.is_loading_browse.clone();
        let status_lock = self.status_banner.clone();
        let egui_ctx = ctx.clone();

        if page <= 1 {
            items_lock.write().clear();
        }
        *loading_lock.write() = true;
        *status_lock.write() = format!("Fetching titles from {} (Page {})…", source_id, page);

        let nsfw_enabled = self.settings.nsfw_enabled;
        self.rt.spawn(async move {
            match reg
                .fetch_from_source(&source_id, page, latest, query.as_deref(), nsfw_enabled)
                .await
            {
                Ok(res) => {
                    *status_lock.write() =
                        format!("Loaded {} titles from {}", res.items.len(), source_id);
                    if page <= 1 {
                        *items_lock.write() = res.items;
                    } else {
                        let mut guard = items_lock.write();
                        let existing: HashSet<String> = guard.iter().map(|m| m.id.clone()).collect();
                        for item in res.items {
                            if !existing.contains(&item.id) {
                                guard.push(item);
                            }
                        }
                    }
                }
                Err(e) => {
                    *status_lock.write() = format!("Source error ({}): {}", source_id, e);
                }
            }
            *loading_lock.write() = false;
            egui_ctx.request_repaint();
        });
    }

    fn open_manga_details(&mut self, ctx: &egui::Context, manga: Manga) {
        self.library.record_history(&manga);
        self.selected_manga = Some(manga.clone());
        self.active_reader_chapter = None;
        self.details_chapter_search.clear();
        self.details_synopsis_expanded = false;

        let cached = self.library.get_cached_chapters(&manga.id);
        *self.manga_chapters.write() = cached;

        let reg = self.registry.clone();
        let ch_lock = self.manga_chapters.clone();
        let loading_lock = self.is_loading_chapters.clone();
        let status_lock = self.status_banner.clone();
        let lib = self.library.clone();
        let egui_ctx = ctx.clone();

        *loading_lock.write() = true;
        *status_lock.write() = format!("Loading details & chapters for {}…", manga.title);

        self.rt.spawn(async move {
            if let Some(src) = reg.get(&manga.source_id) {
                if let Ok(detailed) = src.details(&manga).await {
                    lib.record_history(&detailed);
                    if lib.is_favorite(&detailed.id) {
                        let _ = lib.save_favorite(&detailed);
                    }
                }
                match src.chapters(&manga).await {
                    Ok(chapters) if !chapters.is_empty() => {
                        lib.cache_chapters(&manga.id, &chapters);
                        *ch_lock.write() = chapters.clone();
                        *status_lock.write() =
                            format!("Loaded {} chapters for {}", chapters.len(), manga.title);
                    }
                    Ok(_) => {
                        *status_lock.write() = format!("No new chapters returned for {}", manga.title);
                    }
                    Err(e) => {
                        *status_lock.write() = format!("Chapter fetch error: {}", e);
                    }
                }
            }
            *loading_lock.write() = false;
            egui_ctx.request_repaint();
        });
    }

    fn open_chapter_reader(
        &mut self,
        ctx: &egui::Context,
        manga: Manga,
        chapter: Chapter,
        start_page: usize,
    ) {
        self.library.record_history(&manga);
        self.active_reader_chapter = Some(chapter.clone());
        self.reader_textures.clear();
        self.reader_pipeline.clear_all_failed();
        *self.reader_pages.write() = Vec::new();
        self.reader_loaded_chapters.write().clear();
        *self.reader_loading_next_chapter.write() = false;
        self.reader_current_page = start_page;
        self.reader_show_settings_popup = false;
        self.pending_scroll_delta = 0.0;

        let all_chs_asc = self.sorted_chapters_oldest_first();
        let ch_idx = all_chs_asc
            .iter()
            .position(|c| c.id == chapter.id)
            .unwrap_or(0);
        let total_chs = all_chs_asc.len().max(1);

        self.library.save_progress(&ReadingProgress {
            manga_id: manga.id.clone(),
            chapter_id: chapter.id.clone(),
            page_index: start_page,
            scroll_offset_y: 0.0,
            chapter_number: chapter.number,
            updated_at: chrono::Utc::now().timestamp_millis(),
        });

        // Check if chapter is already downloaded offline as a CBZ
        let offline_pages = self
            .downloader
            .downloaded_for_manga(&manga.id)
            .into_iter()
            .find(|d| d.chapter_id == chapter.id || d.chapter_title == chapter.title)
            .map(|dl| self.downloader.local_pages(&dl))
            .unwrap_or_default();

        let reg = self.registry.clone();
        let pages_lock = self.reader_pages.clone();
        let loaded_chs_lock = self.reader_loaded_chapters.clone();
        let loading_lock = self.is_loading_pages.clone();
        let status_lock = self.status_banner.clone();
        let pipeline = self.reader_pipeline.clone();
        let seam_trim = if self.settings.auto_crop_margins {
            self.settings.seam_trim_px.max(4)
        } else {
            self.settings.seam_trim_px
        };
        let preload_ahead = self.settings.preload_pages_ahead.max(4);
        let egui_ctx = ctx.clone();
        let ch_for_section = chapter.clone();

        *loading_lock.write() = true;
        *status_lock.write() = format!("Fetching chapter pages for {}…", chapter.title);

        self.rt.spawn(async move {
            let resolved_pages = if !offline_pages.is_empty() {
                Ok(offline_pages)
            } else if let Some(src) = reg.get(&chapter.source_id) {
                src.pages(&chapter).await
            } else {
                Err(anyhow::anyhow!("Unknown source {}", chapter.source_id))
            };

            match resolved_pages {
                Ok(pages) => {
                    *status_lock.write() = format!(
                        "Loaded {} pages — streaming & slicing 2048px GPU tiles…",
                        pages.len()
                    );
                    *pages_lock.write() = pages.clone();
                    *loaded_chs_lock.write() = vec![LoadedChapterSection {
                        chapter: ch_for_section,
                        chapter_index: ch_idx + 1,
                        total_chapters: total_chs,
                        pages: pages.clone(),
                    }];
                    egui_ctx.request_repaint();

                    for p in pages.into_iter().skip(start_page).take(preload_ahead + 2) {
                        let pipe = pipeline.clone();
                        let ctx_clone = egui_ctx.clone();
                        tokio::spawn(async move {
                            let _ = pipe.load_and_slice_page(&p, seam_trim).await;
                            ctx_clone.request_repaint();
                        });
                    }
                }
                Err(e) => {
                    *status_lock.write() = format!("Failed to fetch chapter pages: {}", e);
                }
            }
            *loading_lock.write() = false;
            egui_ctx.request_repaint();
        });
    }

    fn sorted_chapters_oldest_first(&self) -> Vec<Chapter> {
        let mut chs = self.manga_chapters.read().clone();
        if chs.len() <= 1 {
            return chs;
        }
        let has_numbers = chs.iter().any(|c| c.number.unwrap_or(0.0) > 0.0);
        if has_numbers {
            chs.sort_by(|a, b| {
                a.number
                    .unwrap_or(999_999.0)
                    .partial_cmp(&b.number.unwrap_or(999_999.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            chs.reverse();
        }
        chs
    }
}

impl eframe::App for ZibraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_logo_texture(ctx);
        self.ensure_custom_bg_texture(ctx);
        let tokens = GlassTokens::from_settings_with_bg(&self.settings, self.bg_avg_luminance);
        apply_kotatsu_visuals(ctx, &tokens);

        // Global shortcuts (F11 Fullscreen, Ctrl+B toggle rail)
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::B)) {
            self.rail_expanded = !self.rail_expanded;
        }

        // Fullscreen / Immersive Chapter Reader Overlay
        if let Some(chapter) = self.active_reader_chapter.clone() {
            self.render_reader_screen(ctx, &tokens, chapter);
            return;
        }

        // 1. Top Window Chrome Bar (matching ZERAVRIX WindowChrome.kt)
        self.render_top_chrome(ctx, &tokens);

        // 2. Left Compact Navigation Rail (matching ZERAVRIX AppNavigationRail.kt & screenshot)
        self.render_nav_rail(ctx, &tokens);

        // 3. Main Content Workspace
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(if self.bg_texture.is_some() { Color32::TRANSPARENT } else { tokens.background })
                    .inner_margin(Margin::symmetric(22.0, 14.0)),
            )
            .show(ctx, |ui| {
                if let Some((_, ref tex)) = self.bg_texture {
                    let rect = ui.max_rect();
                    ui.painter().image(
                        tex.id(),
                        rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    let overlay_tint = if tokens.is_dark {
                        Color32::from_rgba_unmultiplied(10, 13, 18, 120)
                    } else {
                        Color32::from_rgba_unmultiplied(245, 247, 252, 125)
                    };
                    ui.painter().rect_filled(rect, Rounding::ZERO, overlay_tint);
                }

                if let Some(manga) = self.selected_manga.clone() {
                    self.render_details_screen(ctx, ui, &tokens, manga);
                    return;
                }

                match self.current_screen {
                    NavScreen::History => self.render_shelf_screen(ctx, ui, &tokens, NavScreen::History),
                    NavScreen::Favorites => self.render_shelf_screen(ctx, ui, &tokens, NavScreen::Favorites),
                    NavScreen::Downloads => self.render_shelf_screen(ctx, ui, &tokens, NavScreen::Downloads),
                    NavScreen::Explore => self.render_explore_screen(ctx, ui, &tokens),
                    NavScreen::Feed => self.render_feed_screen(ctx, ui, &tokens),
                    NavScreen::Settings => self.render_settings_screen(ctx, ui, &tokens),
                }
            });

        // 4. Floating Bottom-Right "Continue Reading" Pill Button (matching screenshot)
        if self.selected_manga.is_none()
            && self.active_reader_chapter.is_none()
            && self.current_screen != NavScreen::Settings
        {
            if let Some((latest_manga, latest_prog)) = self.library.latest_global_read() {
                if floating_continue_button(ctx, &tokens, &latest_manga, &latest_prog) {
                    let cached = self.library.get_cached_chapters(&latest_manga.id);
                    if let Some(ch) = cached
                        .iter()
                        .find(|c| c.id == latest_prog.chapter_id)
                        .or_else(|| cached.first())
                        .cloned()
                    {
                        let start_p = latest_prog.page_index;
                        self.selected_manga = Some(latest_manga.clone());
                        self.open_chapter_reader(ctx, latest_manga, ch, start_p);
                    } else {
                        self.open_manga_details(ctx, latest_manga);
                    }
                }
            }
        }
    }
}

impl ZibraApp {
    fn render_top_chrome(&mut self, ctx: &egui::Context, tokens: &GlassTokens) {
        egui::TopBottomPanel::top("zibra_top_window_chrome")
            .exact_height(46.0)
            .frame(
                egui::Frame::none()
                    .fill(tokens.surface_glass)
                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                    .inner_margin(Margin::symmetric(14.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Transparent brand logo only (no title text, no redundant clutter)
                    if let Some(ref logo) = self.logo_texture {
                        ui.add(
                            egui::Image::from_texture(logo)
                                .fit_to_exact_size(Vec2::new(26.0, 26.0))
                                .rounding(Rounding::same(6.0)),
                        );
                    } else {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, Rounding::same(6.0), tokens.primary_container);
                        paint_native_icon(&painter, rect, NativeIcon::Explore, tokens.primary, 1.8_f32);
                    }

                    // Right-aligned controls: native vector Fullscreen and AMOLED toggles
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Fullscreen toggle button
                        if z_icon_button(
                            ui,
                            tokens,
                            NativeIcon::Fullscreen,
                            Vec2::new(32.0, 32.0),
                            self.is_fullscreen,
                            if self.is_fullscreen { "Exit Fullscreen (F11)" } else { "Enter Fullscreen (F11)" },
                        ) {
                            self.is_fullscreen = !self.is_fullscreen;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                        }

                        ui.add_space(6.0);

                        // AMOLED / Dark mode toggle
                        if z_icon_button(
                            ui,
                            tokens,
                            NativeIcon::Amoled,
                            Vec2::new(32.0, 32.0),
                            self.settings.is_amoled,
                            if self.settings.is_amoled { "AMOLED Black Active" } else { "Switch to AMOLED Black" },
                        ) {
                            self.settings.is_amoled = !self.settings.is_amoled;
                            let _ = self.settings_store.save(&self.settings);
                        }
                    });
                });
            });
    }

    fn render_nav_rail(&mut self, ctx: &egui::Context, tokens: &GlassTokens) {
        let rail_w = if self.rail_expanded { 176.0 } else { 72.0 };
        egui::SidePanel::left("zibra_kotatsu_nav_rail")
            .resizable(false)
            .exact_width(rail_w)
            .frame(
                egui::Frame::none()
                    .fill(tokens.surface_glass)
                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                    .inner_margin(Margin::symmetric(10.0, 14.0)),
            )
            .show(ctx, |ui| {
                // Top Group: 1. History, 2. Favorites, 3. Explore (exact match to AppNavigationRail.kt)
                let top_group = [
                    (NavScreen::History, NativeIcon::History, "History"),
                    (NavScreen::Favorites, NativeIcon::Favorites, "Favorites"),
                    (NavScreen::Explore, NativeIcon::Explore, "Explore"),
                ];

                ui.vertical_centered(|ui| {
                    for (scr, icon, label) in top_group {
                        if self.nav_rail_button(ui, tokens, scr, icon, label, None) {
                            self.current_screen = scr;
                            self.selected_manga = None;
                        }
                        ui.add_space(6.0);
                    }
                });

                // Bottom Group: Feed, Downloads, Settings + Expand toggle
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    let toggle_lbl = if self.rail_expanded { "◀ Collapse" } else { "▶" };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(toggle_lbl).size(11.0).color(tokens.text_muted),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .on_hover_text("Toggle navigation labels (Ctrl+B)")
                        .clicked()
                    {
                        self.rail_expanded = !self.rail_expanded;
                    }
                    ui.add_space(4.0);

                    if self.nav_rail_button(ui, tokens, NavScreen::Settings, NativeIcon::Settings, "Settings", None) {
                        self.current_screen = NavScreen::Settings;
                        self.selected_manga = None;
                    }
                    ui.add_space(6.0);

                    let active_dl_count = self
                        .downloader
                        .jobs()
                        .iter()
                        .filter(|j| matches!(j.status, DownloadStatus::Downloading { .. }))
                        .count();
                    let dl_badge = if active_dl_count > 0 {
                        Some(active_dl_count.to_string())
                    } else {
                        None
                    };
                    if self.nav_rail_button(
                        ui,
                        tokens,
                        NavScreen::Downloads,
                        NativeIcon::Downloads,
                        "Downloads",
                        dl_badge.as_deref(),
                    ) {
                        self.current_screen = NavScreen::Downloads;
                        self.selected_manga = None;
                    }
                    ui.add_space(6.0);

                    if self.nav_rail_button(ui, tokens, NavScreen::Feed, NativeIcon::Feed, "Feed", None) {
                        self.current_screen = NavScreen::Feed;
                        self.selected_manga = None;
                    }
                });
            });
    }

    fn nav_rail_button(
        &self,
        ui: &mut egui::Ui,
        tokens: &GlassTokens,
        screen: NavScreen,
        icon: NativeIcon,
        label: &str,
        badge: Option<&str>,
    ) -> bool {
        let selected = self.current_screen == screen && self.selected_manga.is_none();
        let btn_size = if self.rail_expanded {
            Vec2::new(152.0, 42.0)
        } else {
            Vec2::new(48.0, 42.0)
        };

        let (rect, resp) = ui.allocate_exact_size(btn_size, Sense::click());
        let hovered = resp.hovered();
        let hover_t = ui.ctx().animate_bool_responsive(resp.id.with("rail_hover"), hovered);
        let sel_t = ui.ctx().animate_bool_responsive(resp.id.with("rail_sel"), selected);

        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let fg = if sel_t > 0.01 {
            Color32::from_rgba_premultiplied(
                egui::lerp(tokens.text_muted.r() as f32..=tokens.primary.r() as f32, sel_t) as u8,
                egui::lerp(tokens.text_muted.g() as f32..=tokens.primary.g() as f32, sel_t) as u8,
                egui::lerp(tokens.text_muted.b() as f32..=tokens.primary.b() as f32, sel_t) as u8,
                255,
            )
        } else {
            Color32::from_rgba_premultiplied(
                egui::lerp(tokens.text_muted.r() as f32..=tokens.text_primary.r() as f32, hover_t) as u8,
                egui::lerp(tokens.text_muted.g() as f32..=tokens.text_primary.g() as f32, hover_t) as u8,
                egui::lerp(tokens.text_muted.b() as f32..=tokens.text_primary.b() as f32, hover_t) as u8,
                255,
            )
        };

        let fill = if sel_t > 0.01 {
            let [r, g, b, _] = tokens.primary_container.to_array();
            Color32::from_rgba_unmultiplied(r, g, b, (sel_t * 220.0) as u8)
        } else if hover_t > 0.01 {
            let [r, g, b, _] = tokens.surface_container_high.to_array();
            Color32::from_rgba_unmultiplied(r, g, b, (hover_t * 180.0) as u8)
        } else {
            Color32::TRANSPARENT
        };

        let stroke = if sel_t > 0.01 {
            Stroke::new(1.5_f32, tokens.border_glow)
        } else if hover_t > 0.01 {
            Stroke::new(1.0_f32, tokens.border_glass)
        } else {
            Stroke::NONE
        };

        let painter = ui.painter_at(rect);
        if fill != Color32::TRANSPARENT {
            painter.rect_filled(rect, Rounding::same(12.0), fill);
        }
        if stroke != Stroke::NONE {
            painter.rect_stroke(rect, Rounding::same(12.0), stroke);
        }

        if self.rail_expanded {
            let icon_rect = Rect::from_min_size(rect.min + Vec2::new(12.0, 9.0), Vec2::new(24.0, 24.0));
            paint_native_icon(&painter, icon_rect, icon, fg, 1.8_f32);

            let text_pos = Pos2::new(rect.min.x + 44.0, rect.center().y);
            let display_text = if let Some(b) = badge {
                format!("{} ({})", label, b)
            } else {
                label.to_string()
            };
            painter.text(
                text_pos,
                Align2::LEFT_CENTER,
                display_text,
                FontId::proportional(13.5),
                fg,
            );
        } else {
            let icon_rect = Rect::from_center_size(rect.center(), Vec2::new(24.0, 24.0));
            paint_native_icon(&painter, icon_rect, icon, fg, 1.8_f32);
        }

        resp.on_hover_text(label).clicked()
    }

    fn render_shelf_screen(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tokens: &GlassTokens,
        kind: NavScreen,
    ) {
        // 1. Full-width Search Bar (matching MangaShelfScreen.kt & screenshot)
        let hint = format!("Search in {}…", kind.title().to_lowercase());
        let avail_w = ui.available_width();
        z_search_bar(ui, tokens, "shelf_search_bar", &mut self.shelf_search, &hint, avail_w);
        ui.add_space(10.0);

        // 2. Filter Chips Row (On device, Reading, Completed, Favourites, Not in favourites)
        let filters = match kind {
            NavScreen::History => vec![
                ShelfFilter::OnDevice,
                ShelfFilter::Reading,
                ShelfFilter::Completed,
                ShelfFilter::Favourites,
                ShelfFilter::NotInFavourites,
            ],
            NavScreen::Favorites => vec![
                ShelfFilter::OnDevice,
                ShelfFilter::Reading,
                ShelfFilter::Completed,
            ],
            _ => vec![
                ShelfFilter::Reading,
                ShelfFilter::Completed,
                ShelfFilter::Favourites,
                ShelfFilter::NotInFavourites,
            ],
        };

        ui.horizontal_wrapped(|ui| {
            for f in filters {
                let selected = self.active_shelf_filter == Some(f);
                if z_chip(ui, tokens, f.label(), Some(f.icon()), selected) {
                    self.active_shelf_filter = if selected { None } else { Some(f) };
                }
            }
        });
        ui.add_space(12.0);

        // If on Downloads shelf, also show active download queue jobs at the top
        if kind == NavScreen::Downloads {
            let jobs = self.downloader.jobs();
            if !jobs.is_empty() {
                egui::Frame::none()
                    .fill(tokens.surface_card)
                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("ACTIVE DOWNLOAD QUEUE")
                                .size(11.5)
                                .color(tokens.primary)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        for job in jobs {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&job.manga.title).strong().color(tokens.text_primary));
                                ui.label(RichText::new("•").color(tokens.text_muted));
                                ui.label(RichText::new(&job.chapter.title).color(tokens.primary));
                                match job.status {
                                    DownloadStatus::Queued => {
                                        ui.label(RichText::new("Queued…").color(tokens.text_muted));
                                    }
                                    DownloadStatus::Downloading {
                                        completed_pages,
                                        total_pages,
                                    } => {
                                        let frac = completed_pages as f32 / (total_pages.max(1) as f32);
                                        ui.add(
                                            egui::ProgressBar::new(frac)
                                                .desired_width(140.0)
                                                .text(format!("{}/{} pages", completed_pages, total_pages)),
                                        );
                                    }
                                    DownloadStatus::Completed { .. } => {
                                        ui.label(
                                            RichText::new("✓ Saved CBZ")
                                                .color(Color32::from_rgb(81, 207, 102))
                                                .strong(),
                                        );
                                    }
                                    DownloadStatus::Failed { ref error } => {
                                        ui.label(
                                            RichText::new(format!("⚠ {}", error))
                                                .color(Color32::from_rgb(255, 107, 107)),
                                        );
                                    }
                                }
                            });
                        }
                    });
                ui.add_space(12.0);
            }
        }

        // Load shelf entries + compute reading progress & filter predicates
        let fav_list = self.library.list_favorites();
        let fav_ids: HashSet<String> = fav_list.iter().map(|m| m.id.clone()).collect();
        let dl_list = self.downloader.list_downloaded();
        let dl_ids: HashSet<String> = dl_list.iter().map(|d| d.manga_id.clone()).collect();

        let raw_items: Vec<(Manga, i64)> = match kind {
            NavScreen::History => self.library.list_history_with_timestamps(),
            NavScreen::Favorites => fav_list.into_iter().map(|m| (m, 0)).collect(),
            _ => {
                let mut seen = HashSet::new();
                let mut out = Vec::new();
                for d in dl_list {
                    if seen.insert(d.manga_id.clone()) {
                        let mut m = Manga::new(d.manga_id.clone(), d.manga_title.clone(), d.source_id.clone());
                        m.cover_url = d.manga_cover_url.clone();
                        out.push((m, d.downloaded_at));
                    }
                }
                out
            }
        };

        let q = self.shelf_search.trim().to_lowercase();
        let mut filtered_items: Vec<(Manga, Option<u32>, i64)> = Vec::new();

        for (m, ts) in raw_items {
            if !q.is_empty() && !m.title.to_lowercase().contains(&q) {
                continue;
            }
            let prog = self.library.latest_progress(&m.id);
            let cached_chs = self.library.get_cached_chapters(&m.id);
            let pct = prog.as_ref().map(|_p| {
                let total_chs = cached_chs.len();
                if total_chs > 0 {
                    let read_chs = self.library.read_chapter_ids(&m.id).len();
                    ((read_chs * 100) / total_chs).clamp(5, 100) as u32
                } else {
                    15
                }
            });

            let in_fav = fav_ids.contains(&m.id);
            let on_dev = dl_ids.contains(&m.id);

            let pass_filter = match self.active_shelf_filter {
                None => true,
                Some(ShelfFilter::OnDevice) => on_dev,
                Some(ShelfFilter::Reading) => pct.is_some() && pct.unwrap_or(0) < 100,
                Some(ShelfFilter::Completed) => pct.unwrap_or(0) >= 100,
                Some(ShelfFilter::Favourites) => in_fav,
                Some(ShelfFilter::NotInFavourites) => !in_fav,
            };

            if pass_filter {
                filtered_items.push((m, pct, ts));
            }
        }

        if filtered_items.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    let empty_icon = match kind {
                        NavScreen::History => "🕒",
                        NavScreen::Favorites => "❤",
                        _ => "⬇",
                    };
                    ui.label(RichText::new(empty_icon).size(44.0).color(tokens.primary));
                    ui.add_space(10.0);
                    let empty_title = if !q.is_empty() {
                        "No matching titles"
                    } else if kind == NavScreen::History {
                        "Reading history is empty"
                    } else if kind == NavScreen::Favorites {
                        "No favourites yet"
                    } else {
                        "No downloaded chapters yet"
                    };
                    ui.label(
                        RichText::new(empty_title)
                            .size(18.0)
                            .strong()
                            .color(tokens.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Browse 1,647+ sources in Explore to discover Manhwa & Manga!")
                            .size(13.0)
                            .color(tokens.text_muted),
                    );
                    ui.add_space(14.0);
                    if z_chip(ui, tokens, "Explore Manhwa & Manga", Some("🧭"), true) {
                        self.current_screen = NavScreen::Explore;
                    }
                });
            });
            return;
        }

        let mut to_open: Option<Manga> = None;
        let mut to_remove_fav: Option<String> = None;
        let mut to_remove_hist: Option<String> = None;

        ScrollArea::vertical()
            .id_salt(("shelf_scroll_area", kind.title()))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if kind == NavScreen::History {
                    // Group by time buckets ("Today", "Recently viewed", etc., matching screenshot)
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let mut groups: Vec<(&'static str, Vec<(Manga, Option<u32>)>)> = Vec::new();
                    for (m, pct, ts) in filtered_items {
                        let age_ms = (now_ms - ts).max(0);
                        let bucket = if age_ms < 24 * 3600 * 1000 {
                            "Today"
                        } else if age_ms < 48 * 3600 * 1000 {
                            "Yesterday"
                        } else if age_ms < 7 * 24 * 3600 * 1000 {
                            "This week"
                        } else {
                            "Recently viewed"
                        };
                        if let Some((_, list)) = groups.iter_mut().find(|(b, _)| *b == bucket) {
                            list.push((m, pct));
                        } else {
                            groups.push((bucket, vec![(m, pct)]));
                        }
                    }

                    for (header, items) in groups {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(header)
                                .size(15.0)
                                .strong()
                                .color(tokens.text_primary),
                        );
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(14.0, 16.0);
                            for (m, pct) in items {
                                match manga_cover_card(
                                    ui,
                                    ctx,
                                    &self.rt,
                                    &self.cover_cache,
                                    &mut self.cover_textures,
                                    tokens,
                                    &m,
                                    pct,
                                    true,
                                ) {
                                    CardAction::Open => to_open = Some(m),
                                    CardAction::RemoveFavorite => to_remove_fav = Some(m.id),
                                    CardAction::RemoveHistory => to_remove_hist = Some(m.id),
                                    CardAction::None => {}
                                }
                            }
                        });
                        ui.add_space(14.0);
                    }
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(14.0, 16.0);
                        for (m, pct, _) in filtered_items {
                            match manga_cover_card(
                                ui,
                                ctx,
                                &self.rt,
                                &self.cover_cache,
                                &mut self.cover_textures,
                                tokens,
                                &m,
                                pct,
                                true,
                            ) {
                                CardAction::Open => to_open = Some(m),
                                CardAction::RemoveFavorite => to_remove_fav = Some(m.id),
                                CardAction::RemoveHistory => to_remove_hist = Some(m.id),
                                CardAction::None => {}
                            }
                        }
                    });
                }
                ui.add_space(64.0);
            });

        if let Some(id) = to_remove_fav {
            let _ = self.library.remove_favorite(&id);
        }
        if let Some(id) = to_remove_hist {
            let _ = self.library.remove_history(&id);
        }
        if let Some(m) = to_open {
            self.open_manga_details(ctx, m);
        }
    }

    fn render_explore_screen(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tokens: &GlassTokens,
    ) {
        match self.explore_mode {
            ExploreSubMode::PinnedAndSearch => {
                // 1. Global Search Bar
                let avail_w = ui.available_width();
                if z_search_bar(
                    ui,
                    tokens,
                    "explore_global_search_input",
                    &mut self.explore_global_search,
                    "Search manga, manhwa, authors across pinned sources…",
                    avail_w,
                ) {
                    self.trigger_global_search(ctx);
                }
                ui.add_space(10.0);

                // 2. 3 Quick Action Pills (Local Files, Bookmarks, Downloads — matching ExploreScreen.kt)
                ui.columns(3, |cols| {
                    let btn_local = egui::Button::new(
                        RichText::new("📁  Local CBZ & Files")
                            .size(12.5)
                            .strong()
                            .color(tokens.text_primary),
                    )
                    .fill(tokens.surface_card)
                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(cols[0].available_width(), 40.0));
                    if cols[0].add(btn_local).clicked() {
                        self.trigger_source_fetch(ctx, "LOCAL_FILES".to_string(), 1, false, None);
                    }

                    let btn_fav = egui::Button::new(
                        RichText::new("❤  Bookmarks & Library")
                            .size(12.5)
                            .strong()
                            .color(tokens.text_primary),
                    )
                    .fill(tokens.surface_card)
                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(cols[1].available_width(), 40.0));
                    if cols[1].add(btn_fav).clicked() {
                        self.current_screen = NavScreen::Favorites;
                    }

                    let btn_dl = egui::Button::new(
                        RichText::new("⬇  Offline Downloads")
                            .size(12.5)
                            .strong()
                            .color(tokens.text_primary),
                    )
                    .fill(tokens.surface_card)
                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(cols[2].available_width(), 40.0));
                    if cols[2].add(btn_dl).clicked() {
                        self.current_screen = NavScreen::Downloads;
                    }
                });
                ui.add_space(14.0);

                // 3. Pinned Sources Header + Sources Catalog Button
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("PINNED SOURCES")
                            .size(12.0)
                            .strong()
                            .color(tokens.text_primary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let cat_label = format!(
                            "🌐  Sources Catalog ({} Sources)",
                            self.registry.visible_count(self.settings.nsfw_enabled)
                        );
                        if z_chip(ui, tokens, &cat_label, None, true) {
                            self.explore_mode = ExploreSubMode::SourcesCatalog;
                        }
                    });
                });
                ui.add_space(8.0);

                let pinned_ids = self.settings.pinned_source_ids.clone();
                let pinned_sources: Vec<_> = self
                    .registry
                    .ordered_catalog(
                        self.settings.catalog_sort_mode,
                        &pinned_ids,
                        &HashSet::new(),
                        "ALL",
                        "",
                        self.settings.nsfw_enabled,
                    )
                    .into_iter()
                    .filter(|s| pinned_ids.contains(s.id()))
                    .collect();

                let mut to_fetch_src: Option<String> = None;
                let mut to_unpin_src: Option<String> = None;

                // Horizontal Pinned Sources Badge Strip (matching PinnedSourceBadge in ExploreScreen.kt)
                ScrollArea::horizontal()
                    .id_salt("explore_pinned_badge_strip")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(12.0, 8.0);
                            for src in &pinned_sources {
                                egui::Frame::none()
                                    .fill(tokens.surface_container_high)
                                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                    .rounding(Rounding::same(14.0))
                                    .inner_margin(Margin::symmetric(10.0, 8.0))
                                    .show(ui, |ui| {
                                        ui.set_width(78.0);
                                        ui.vertical_centered(|ui| {
                                            let icon_url = src.icon_url();
                                            let resp = draw_remote_image(
                                                ui,
                                                ctx,
                                                &self.rt,
                                                &self.cover_cache,
                                                &mut self.cover_textures,
                                                tokens,
                                                Some(&icon_url),
                                                src.name(),
                                                Vec2::new(42.0, 42.0),
                                                10.0,
                                            );
                                            ui.add_space(4.0);
                                            let lbl = ui.add(
                                                egui::Label::new(
                                                    RichText::new(truncate_str(src.name(), 11))
                                                        .size(11.0)
                                                        .strong()
                                                        .color(tokens.text_primary),
                                                )
                                                .sense(Sense::click()),
                                            );
                                            if resp.clicked() || lbl.clicked() {
                                                to_fetch_src = Some(src.id().to_string());
                                            }
                                            lbl.context_menu(|ui| {
                                                if ui.button("📌 Unpin from Explore").clicked() {
                                                    to_unpin_src = Some(src.id().to_string());
                                                    ui.close_menu();
                                                }
                                            });
                                        });
                                    });
                            }
                        });
                    });

                if let Some(unpin_id) = to_unpin_src {
                    self.settings.pinned_source_ids.remove(&unpin_id);
                    let _ = self.settings_store.save(&self.settings);
                }

                ui.add_space(14.0);

                // 4. Global Search Results OR Explore By Source Grid
                if !self.explore_global_search.trim().is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("GLOBAL SEARCH RESULTS")
                                .size(12.0)
                                .strong()
                                .color(tokens.primary),
                        );
                        if *self.is_searching_global.read() {
                            ui.spinner();
                        }
                    });
                    ui.add_space(8.0);
                    let results = self.global_search_results.read().clone();
                    let mut to_open_manga = None;
                    ScrollArea::vertical()
                        .id_salt("explore_global_search_results_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(14.0, 16.0);
                                for m in results {
                                    if manga_cover_card(
                                        ui,
                                        ctx,
                                        &self.rt,
                                        &self.cover_cache,
                                        &mut self.cover_textures,
                                        tokens,
                                        &m,
                                        None,
                                        true,
                                    ) == CardAction::Open
                                    {
                                        to_open_manga = Some(m);
                                    }
                                }
                            });
                        });
                    if let Some(m) = to_open_manga {
                        self.open_manga_details(ctx, m);
                    }
                } else {
                    ui.label(
                        RichText::new("EXPLORE BY SOURCE")
                            .size(12.0)
                            .strong()
                            .color(tokens.text_primary),
                    );
                    ui.add_space(8.0);

                    ScrollArea::vertical()
                        .id_salt("explore_by_source_grid_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                                for src in &pinned_sources {
                                    egui::Frame::none()
                                        .fill(tokens.surface_container_high)
                                        .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                        .rounding(Rounding::same(14.0))
                                        .inner_margin(Margin::same(13.0))
                                        .show(ui, |ui| {
                                            ui.set_width(248.0);
                                            ui.horizontal(|ui| {
                                                let icon_url = src.icon_url();
                                                draw_remote_image(
                                                    ui,
                                                    ctx,
                                                    &self.rt,
                                                    &self.cover_cache,
                                                    &mut self.cover_textures,
                                                    tokens,
                                                    Some(&icon_url),
                                                    src.name(),
                                                    Vec2::new(38.0, 38.0),
                                                    9.0,
                                                );
                                                ui.add_space(6.0);
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            RichText::new(truncate_str(src.name(), 18))
                                                                .size(13.5)
                                                                .strong()
                                                                .color(tokens.text_primary),
                                                        );
                                                        if let Some(ms) = self.registry.latency_for(src.id()) {
                                                            ui.label(
                                                                RichText::new(format!("{}ms", ms))
                                                                    .size(10.0)
                                                                    .color(Color32::from_rgb(81, 207, 102)),
                                                            );
                                                        }
                                                    });
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} • Browse Manhwa & Manga",
                                                            src.language().to_uppercase()
                                                        ))
                                                        .size(11.0)
                                                        .color(tokens.primary),
                                                    );
                                                });
                                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                RichText::new("➡")
                                                                    .size(14.0)
                                                                    .color(tokens.primary)
                                                                    .strong(),
                                                            )
                                                            .fill(tokens.primary_container)
                                                            .rounding(Rounding::same(8.0))
                                                            .min_size(Vec2::new(30.0, 30.0)),
                                                        )
                                                        .clicked()
                                                    {
                                                        to_fetch_src = Some(src.id().to_string());
                                                    }
                                                });
                                            });
                                        });
                                }
                            });
                            ui.add_space(64.0);
                        });
                }

                if let Some(src_id) = to_fetch_src {
                    self.trigger_source_fetch(ctx, src_id, 1, false, None);
                }
            }

            ExploreSubMode::SourcesCatalog => {
                // Full 1,647+ Sources Catalog View (Virtualized for 60 FPS scrolling)
                ui.horizontal(|ui| {
                    if z_chip(ui, tokens, "Back", Some("⬅"), false) {
                        self.explore_mode = ExploreSubMode::PinnedAndSearch;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "SOURCES CATALOG ({} Total Sources)",
                            self.registry.visible_count(self.settings.nsfw_enabled)
                        ))
                        .size(15.0)
                        .strong()
                        .color(tokens.primary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let nsfw_lbl = if self.settings.nsfw_enabled {
                            "🔞 18+ Sources: SHOWN"
                        } else {
                            "🛡 18+ Sources: HIDDEN"
                        };
                        if z_chip(ui, tokens, nsfw_lbl, None, !self.settings.nsfw_enabled) {
                            self.settings.nsfw_enabled = !self.settings.nsfw_enabled;
                            let _ = self.settings_store.save(&self.settings);
                        }

                        egui::ComboBox::from_id_salt("catalog_sort_selector")
                            .selected_text(format!("Sort: {}", self.settings.catalog_sort_mode.label()))
                            .show_ui(ui, |ui| {
                                for mode in CatalogSortMode::ALL {
                                    if ui
                                        .selectable_value(
                                            &mut self.settings.catalog_sort_mode,
                                            *mode,
                                            mode.label(),
                                        )
                                        .clicked()
                                    {
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                }
                            });
                    });
                });
                ui.add_space(10.0);

                // Catalog Search Input
                let avail_w = ui.available_width();
                z_search_bar(
                    ui,
                    tokens,
                    "sources_catalog_search_box",
                    &mut self.catalog_search,
                    "Search 1,647+ sources by name, domain, or language…",
                    avail_w,
                );
                ui.add_space(8.0);

                // Language Filter Chips (18 Languages)
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Language:")
                            .size(11.5)
                            .strong()
                            .color(tokens.text_muted),
                    );
                    let langs = [
                        "ALL", "ar", "en", "multi", "ja", "ko", "zh", "fr", "es", "pt", "id", "tr",
                        "ru", "vi", "de", "it", "pl", "th",
                    ];
                    for lang in langs {
                        let active = self.catalog_lang.eq_ignore_ascii_case(lang);
                        if z_chip(ui, tokens, &lang.to_uppercase(), None, active) {
                            self.catalog_lang = lang.to_string();
                        }
                    }
                });
                ui.add_space(6.0);

                // Tag Filter Chips
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Tags:")
                            .size(11.5)
                            .strong()
                            .color(tokens.text_muted),
                    );
                    let filterable_tags = [
                        SourceTag::Manhwa,
                        SourceTag::Webtoon,
                        SourceTag::Manga,
                        SourceTag::Manhua,
                        SourceTag::Arabic,
                        SourceTag::English,
                        SourceTag::OfficialApi,
                        SourceTag::FastCdn,
                        SourceTag::VerifiedWorking,
                        SourceTag::Popular,
                        SourceTag::EcdhEncrypted,
                    ];
                    for tag in filterable_tags {
                        let active = self.active_tag_filters.contains(&tag);
                        let label = format!("#{}", tag.label());
                        if z_chip(ui, tokens, &label, None, active) {
                            if active {
                                self.active_tag_filters.remove(&tag);
                            } else {
                                self.active_tag_filters.insert(tag);
                            }
                        }
                    }
                    if !self.active_tag_filters.is_empty()
                        && z_chip(ui, tokens, "Clear Tags", Some("✕"), false)
                    {
                        self.active_tag_filters.clear();
                    }
                });
                ui.add_space(8.0);

                let ordered = self.registry.ordered_catalog(
                    self.settings.catalog_sort_mode,
                    &self.settings.pinned_source_ids,
                    &self.active_tag_filters,
                    &self.catalog_lang,
                    &self.catalog_search,
                    self.settings.nsfw_enabled,
                );

                ui.label(
                    RichText::new(format!("Showing {} matching sources", ordered.len()))
                        .size(11.5)
                        .color(tokens.text_muted),
                );
                ui.add_space(6.0);

                let mut to_open_source: Option<String> = None;
                let mut to_toggle_pin: Option<String> = None;

                // Virtualized row list: renders only visible rows so 1,647+ sources scroll at 60 FPS!
                let row_h = 62.0;
                ScrollArea::vertical()
                    .id_salt("sources_catalog_virtual_scroll")
                    .auto_shrink([false, false])
                    .show_rows(ui, row_h, ordered.len(), |ui, row_range| {
                        for idx in row_range {
                            let src = &ordered[idx];
                            let is_pinned = self.settings.pinned_source_ids.contains(src.id());
                            egui::Frame::none()
                                .fill(tokens.surface_card)
                                .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::symmetric(14.0, 9.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let icon_url = src.icon_url();
                                        draw_remote_image(
                                            ui,
                                            ctx,
                                            &self.rt,
                                            &self.cover_cache,
                                            &mut self.cover_textures,
                                            tokens,
                                            Some(&icon_url),
                                            src.name(),
                                            Vec2::new(38.0, 38.0),
                                            9.0,
                                        );
                                        ui.add_space(8.0);
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!("#{:03}", src.priority_rank()))
                                                        .size(11.0)
                                                        .color(tokens.primary)
                                                        .strong(),
                                                );
                                                ui.label(
                                                    RichText::new(src.name())
                                                        .size(14.0)
                                                        .strong()
                                                        .color(tokens.text_primary),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "[{}]",
                                                        src.language().to_uppercase()
                                                    ))
                                                    .size(11.0)
                                                    .color(tokens.text_muted),
                                                );
                                                for tag in src.tags().iter().take(4) {
                                                    let (r, g, b) = tag.badge_color_rgb();
                                                    ui.label(
                                                        RichText::new(format!("#{}", tag.label()))
                                                            .size(10.0)
                                                            .color(Color32::from_rgb(r, g, b))
                                                            .strong(),
                                                    );
                                                }
                                            });
                                            ui.label(
                                                RichText::new(src.base_url())
                                                    .size(11.0)
                                                    .color(tokens.text_muted),
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if z_chip(ui, tokens, "Browse ➡", None, true) {
                                                to_open_source = Some(src.id().to_string());
                                            }
                                            let pin_lbl = if is_pinned { "📌 Pinned" } else { "＋ Pin" };
                                            if z_chip(ui, tokens, pin_lbl, None, is_pinned) {
                                                to_toggle_pin = Some(src.id().to_string());
                                            }
                                        });
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    });

                if let Some(pin_id) = to_toggle_pin {
                    if self.settings.pinned_source_ids.contains(&pin_id) {
                        self.settings.pinned_source_ids.remove(&pin_id);
                    } else {
                        self.settings.pinned_source_ids.insert(pin_id);
                    }
                    let _ = self.settings_store.save(&self.settings);
                }
                if let Some(src_id) = to_open_source {
                    self.trigger_source_fetch(ctx, src_id, 1, false, None);
                }
            }

            ExploreSubMode::SourceBrowse => {
                let src_id = self.active_source_id.clone();
                let src_name = self
                    .registry
                    .get(&src_id)
                    .map(|s| s.name().to_string())
                    .unwrap_or_else(|| src_id.clone());
                let items = self.browse_items.read().clone();
                let loading = *self.is_loading_browse.read();

                ui.horizontal(|ui| {
                    if z_chip(ui, tokens, "Sources", Some("⬅"), false) {
                        self.explore_mode = ExploreSubMode::PinnedAndSearch;
                    }
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&src_name)
                                .size(16.0)
                                .strong()
                                .color(tokens.text_primary),
                        );
                        ui.label(
                            RichText::new(format!("{} titles loaded • Page {}", items.len(), self.source_page))
                                .size(11.0)
                                .color(tokens.primary),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let is_pinned = self.settings.pinned_source_ids.contains(&src_id);
                        if z_chip(
                            ui,
                            tokens,
                            if is_pinned { "📌 Pinned" } else { "＋ Pin Source" },
                            None,
                            is_pinned,
                        ) {
                            if is_pinned {
                                self.settings.pinned_source_ids.remove(&src_id);
                            } else {
                                self.settings.pinned_source_ids.insert(src_id.clone());
                            }
                            let _ = self.settings_store.save(&self.settings);
                        }

                        if z_chip(ui, tokens, "Latest", Some("🆕"), self.source_latest_mode) {
                            self.source_latest_mode = true;
                            self.trigger_source_fetch(ctx, src_id.clone(), 1, true, None);
                        }
                        if z_chip(ui, tokens, "Popular", Some("🔥"), !self.source_latest_mode) {
                            self.source_latest_mode = false;
                            self.trigger_source_fetch(ctx, src_id.clone(), 1, false, None);
                        }
                    });
                });
                ui.add_space(10.0);

                let avail_w = ui.available_width();
                if z_search_bar(
                    ui,
                    tokens,
                    "source_specific_search_bar",
                    &mut self.source_search,
                    &format!("Search on {}…", src_name),
                    avail_w,
                ) {
                    let q = if self.source_search.trim().is_empty() {
                        None
                    } else {
                        Some(self.source_search.clone())
                    };
                    self.trigger_source_fetch(ctx, src_id.clone(), 1, self.source_latest_mode, q);
                }
                ui.add_space(12.0);

                if loading && items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(90.0);
                            ui.spinner();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("Connecting to {}…", src_name))
                                    .color(tokens.text_muted),
                            );
                        });
                    });
                } else if items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(90.0);
                            ui.label(
                                RichText::new("No titles returned from this source.")
                                    .size(15.0)
                                    .strong()
                                    .color(tokens.text_primary),
                            );
                            ui.label(
                                RichText::new(
                                    "Try switching between Popular / Latest or check domain overrides in Settings.",
                                )
                                .size(12.5)
                                .color(tokens.text_muted),
                            );
                            ui.add_space(10.0);
                            if z_chip(ui, tokens, "Retry Source", Some("🔄"), true) {
                                self.trigger_source_fetch(
                                    ctx,
                                    src_id.clone(),
                                    1,
                                    self.source_latest_mode,
                                    None,
                                );
                            }
                        });
                    });
                } else {
                    let mut to_open: Option<Manga> = None;
                    ScrollArea::vertical()
                        .id_salt(("source_browse_grid_scroll", &src_id))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(14.0, 16.0);
                                for m in &items {
                                    if manga_cover_card(
                                        ui,
                                        ctx,
                                        &self.rt,
                                        &self.cover_cache,
                                        &mut self.cover_textures,
                                        tokens,
                                        m,
                                        None,
                                        false,
                                    ) == CardAction::Open
                                    {
                                        to_open = Some(m.clone());
                                    }
                                }
                            });
                            ui.add_space(18.0);
                            ui.vertical_centered(|ui| {
                                let (sentinel_rect, _) = ui.allocate_exact_size(Vec2::new(240.0, 36.0), Sense::hover());
                                let near_bottom = ui.is_rect_visible(sentinel_rect);
                                if loading {
                                    ui.spinner();
                                } else {
                                    let clicked = z_chip(
                                        ui,
                                        tokens,
                                        &format!("Load Next Page (Page {}) ⬇", self.source_page + 1),
                                        None,
                                        true,
                                    );
                                    if clicked || (near_bottom && items.len() >= 12) {
                                        let next_p = self.source_page + 1;
                                        let latest = self.source_latest_mode;
                                        let q = if self.source_search.trim().is_empty() {
                                            None
                                        } else {
                                            Some(self.source_search.clone())
                                        };
                                        self.trigger_source_fetch(ctx, src_id.clone(), next_p, latest, q);
                                    }
                                }
                            });
                            ui.add_space(48.0);
                        });

                    if let Some(m) = to_open {
                        self.open_manga_details(ctx, m);
                    }
                }
            }
        }
    }

    fn render_details_screen(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tokens: &GlassTokens,
        manga: Manga,
    ) {
        let total_w = ui.available_width();
        let total_h = ui.available_height();
        let left_w = (total_w * 0.40).clamp(300.0, 420.0);

        let all_chapters_asc = self.sorted_chapters_oldest_first();
        let progress = self.library.latest_progress(&manga.id);
        let read_ids = self.library.read_chapter_ids(&manga.id);
        let downloaded_list = self.downloader.downloaded_for_manga(&manga.id);
        let downloaded_ids: HashSet<String> = downloaded_list.iter().map(|d| d.chapter_id.clone()).collect();

        let current_ch_id = progress.as_ref().map(|p| p.chapter_id.clone());
        let pct = if !all_chapters_asc.is_empty() {
            ((read_ids.len() * 100) / all_chapters_asc.len()).min(100)
        } else {
            0
        };

        let mut chapter_to_read: Option<(Chapter, usize)> = None;
        let mut chapters_to_download: Vec<Chapter> = Vec::new();

        ui.horizontal(|ui| {
            // LEFT PANE: Back Button, 3D Cover Card, Favorite Toggle, InfoCard, Synopsis, Genres
            ui.allocate_ui_with_layout(
                Vec2::new(left_w, total_h),
                Layout::top_down(Align::Min),
                |ui| {
                    ScrollArea::vertical()
                        .id_salt("details_left_pane_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if z_chip(ui, tokens, "Back", Some("⬅"), false) {
                                self.selected_manga = None;
                            }
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                draw_remote_image(
                                    ui,
                                    ctx,
                                    &self.rt,
                                    &self.cover_cache,
                                    &mut self.cover_textures,
                                    tokens,
                                    manga.cover_url.as_deref(),
                                    &manga.title,
                                    Vec2::new(128.0, 178.0),
                                    12.0,
                                );
                                ui.add_space(12.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&manga.title)
                                            .size(19.0)
                                            .strong()
                                            .color(tokens.text_primary),
                                    );
                                    if let Some(ref auth) = manga.author {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(auth)
                                                .size(13.0)
                                                .color(tokens.text_muted),
                                        );
                                    }
                                    ui.add_space(12.0);
                                    let is_fav = self.library.is_favorite(&manga.id);
                                    let fav_lbl = if is_fav {
                                        "In favourites"
                                    } else {
                                        "Add to favourites"
                                    };
                                    let fav_ic = if is_fav { "❤" } else { "🤍" };
                                    if z_chip(ui, tokens, fav_lbl, Some(fav_ic), is_fav) {
                                        self.library.toggle_favorite(&manga);
                                    }
                                });
                            });

                            ui.add_space(16.0);

                            // InfoCard (matching DetailsScreen.kt InfoCard)
                            egui::Frame::none()
                                .fill(tokens.surface_card)
                                .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                .rounding(Rounding::same(14.0))
                                .inner_margin(Margin::symmetric(16.0, 12.0))
                                .show(ui, |ui| {
                                    let src_name = self
                                        .registry
                                        .get(&manga.source_id)
                                        .map(|s| s.name().to_string())
                                        .unwrap_or_else(|| manga.source_id.clone());
                                    Self::info_row(ui, tokens, "Source", &src_name);
                                    if let Some(ref a) = manga.author {
                                        Self::info_row(ui, tokens, "Author", a);
                                    }
                                    Self::info_row(
                                        ui,
                                        tokens,
                                        "Status",
                                        manga.status.as_deref().unwrap_or("Ongoing"),
                                    );
                                    Self::info_row(
                                        ui,
                                        tokens,
                                        "Chapters",
                                        &format!("{} chapters", all_chapters_asc.len()),
                                    );
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Progress")
                                                .size(12.5)
                                                .color(tokens.text_muted),
                                        );
                                        ui.add_space(18.0);
                                        ui.add(
                                            egui::ProgressBar::new(pct as f32 / 100.0)
                                                .desired_width(140.0)
                                                .text(format!("{}%", pct)),
                                        );
                                    });
                                });

                            ui.add_space(14.0);

                            // Synopsis Header + Expand Toggle
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Synopsis")
                                        .size(14.5)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let toggle_txt = if self.details_synopsis_expanded {
                                        "Show less"
                                    } else {
                                        "Read more"
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(toggle_txt)
                                                    .size(12.0)
                                                    .color(tokens.primary)
                                                    .strong(),
                                            )
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE),
                                        )
                                        .clicked()
                                    {
                                        self.details_synopsis_expanded = !self.details_synopsis_expanded;
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            let desc_raw = if manga.description.trim().is_empty() {
                                "No synopsis available for this title."
                            } else {
                                manga.description.as_str()
                            };
                            let desc_shown = if self.details_synopsis_expanded {
                                desc_raw.to_string()
                            } else {
                                truncate_str(desc_raw, 260)
                            };
                            ui.label(
                                RichText::new(desc_shown)
                                    .size(12.5)
                                    .color(tokens.text_muted),
                            );

                            if !manga.genres.is_empty() {
                                ui.add_space(12.0);
                                ui.horizontal_wrapped(|ui| {
                                    for g in &manga.genres {
                                        z_chip(ui, tokens, g, None, false);
                                    }
                                });
                            }
                            ui.add_space(24.0);
                        });
                },
            );

            ui.add_space(12.0);

            // RIGHT PANE: Elevated Glass Chapter Panel with Continue Button, Search, Sort & Virtualized Chapter List
            let right_w = ui.available_width();
            egui::Frame::none()
                .fill(tokens.surface_glass_raised)
                .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                .rounding(Rounding::same(18.0))
                .inner_margin(Margin::symmetric(18.0, 14.0))
                .show(ui, |ui| {
                    ui.set_width((right_w - 38.0).max(260.0));
                    ui.set_height((total_h - 30.0).max(260.0));

                    // Top Action Bar (Continue / Start Reading, First, Latest, Next 5 CBZ, Sort Toggle)
                    ui.horizontal_wrapped(|ui| {
                        let target_ch = current_ch_id
                            .as_ref()
                            .and_then(|cid| all_chapters_asc.iter().find(|c| &c.id == cid).cloned())
                            .or_else(|| all_chapters_asc.first().cloned());

                        if let Some(ref target) = target_ch {
                            let start_page = progress
                                .as_ref()
                                .filter(|p| p.chapter_id == target.id)
                                .map(|p| p.page_index)
                                .unwrap_or(0);
                            let main_lbl = if progress.is_some() {
                                "▶  Continue Reading"
                            } else {
                                "▶  Start Reading"
                            };
                            let primary_btn = egui::Button::new(
                                RichText::new(main_lbl)
                                    .size(13.5)
                                    .strong()
                                    .color(tokens.on_primary),
                            )
                            .fill(tokens.primary)
                            .rounding(Rounding::same(14.0))
                            .min_size(Vec2::new(150.0, 36.0));
                            if ui.add(primary_btn).clicked() {
                                chapter_to_read = Some((target.clone(), start_page));
                            }
                        }

                        if let Some(first_ch) = all_chapters_asc.first() {
                            if z_chip(ui, tokens, "First Ch.", Some("⏮"), false) {
                                chapter_to_read = Some((first_ch.clone(), 0));
                            }
                        }
                        if let Some(last_ch) = all_chapters_asc.last() {
                            if z_chip(ui, tokens, "Latest Ch.", Some("⏭"), false) {
                                chapter_to_read = Some((last_ch.clone(), 0));
                            }
                        }
                        if !all_chapters_asc.is_empty()
                            && z_chip(ui, tokens, "Next 5 CBZ", Some("⬇"), false)
                        {
                            for ch in all_chapters_asc
                                .iter()
                                .filter(|c| !downloaded_ids.contains(&c.id))
                                .take(5)
                            {
                                chapters_to_download.push(ch.clone());
                            }
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let sort_lbl = if self.details_newest_first {
                                "Newest first"
                            } else {
                                "Oldest first"
                            };
                            let sort_ic = if self.details_newest_first { "⬇" } else { "⬆" };
                            if z_chip(ui, tokens, sort_lbl, Some(sort_ic), false) {
                                self.details_newest_first = !self.details_newest_first;
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Chapter Filter Search Bar
                    let ch_search_w = ui.available_width();
                    z_search_bar(
                        ui,
                        tokens,
                        "details_chapter_search_input",
                        &mut self.details_chapter_search,
                        &format!("Filter {} chapters by number or title…", all_chapters_asc.len()),
                        ch_search_w,
                    );
                    ui.add_space(8.0);

                    let mut display_chapters = all_chapters_asc;
                    if self.details_newest_first {
                        display_chapters.reverse();
                    }
                    let ch_q = self.details_chapter_search.trim().to_lowercase();
                    if !ch_q.is_empty() {
                        display_chapters.retain(|c| c.title.to_lowercase().contains(&ch_q));
                    }

                    let is_loading_chs = *self.is_loading_chapters.read();
                    if is_loading_chs && display_chapters.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(60.0);
                                ui.spinner();
                                ui.label(
                                    RichText::new("Loading chapter list…").color(tokens.text_muted),
                                );
                            });
                        });
                    } else if display_chapters.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new("No chapters found for this title.")
                                    .color(tokens.text_muted),
                            );
                        });
                    } else {
                        // Virtualized Scrollable Chapter List (54px per row)
                        let row_h = 52.0;
                        ScrollArea::vertical()
                            .id_salt(("details_chapters_virtual_scroll", &manga.id))
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                            .show_rows(ui, row_h, display_chapters.len(), |ui, row_range| {
                                for idx in row_range {
                                    let ch = &display_chapters[idx];
                                    let is_current = current_ch_id.as_deref() == Some(ch.id.as_str());
                                    let is_read = read_ids.contains(&ch.id);
                                    let is_downloaded = downloaded_ids.contains(&ch.id);

                                    let row_fill = if is_current {
                                        tokens.primary_container
                                    } else {
                                        tokens.surface_card
                                    };
                                    let row_stroke = if is_current {
                                        Stroke::new(1.5_f32, tokens.primary)
                                    } else {
                                        Stroke::new(1.0_f32, tokens.border_glass)
                                    };
                                    let title_color = if is_current {
                                        tokens.primary
                                    } else if is_read {
                                        tokens.text_muted
                                    } else {
                                        tokens.text_primary
                                    };

                                    egui::Frame::none()
                                        .fill(row_fill)
                                        .stroke(row_stroke)
                                        .rounding(Rounding::same(10.0))
                                        .inner_margin(Margin::symmetric(12.0, 8.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let prefix_icon = if is_current {
                                                    "▶"
                                                } else if is_read {
                                                    "✓"
                                                } else {
                                                    "•"
                                                };
                                                ui.label(
                                                    RichText::new(prefix_icon)
                                                        .color(title_color)
                                                        .strong(),
                                                );
                                                ui.vertical(|ui| {
                                                    let resp = ui.add(
                                                        egui::Label::new(
                                                            RichText::new(truncate_str(&ch.title, 54))
                                                                .size(13.5)
                                                                .color(title_color)
                                                                .strong(),
                                                        )
                                                        .sense(Sense::click()),
                                                    );
                                                    let num_txt = ch
                                                        .number
                                                        .map(|n| format!("#{}", n))
                                                        .unwrap_or_else(|| "Chapter".to_string());
                                                    let sub = match &ch.scanlator {
                                                        Some(sc) if !sc.is_empty() => {
                                                            format!("{} • {}", num_txt, sc)
                                                        }
                                                        _ => num_txt,
                                                    };
                                                    ui.label(
                                                        RichText::new(sub)
                                                            .size(11.0)
                                                            .color(tokens.text_muted),
                                                    );
                                                    if resp.clicked() {
                                                        let saved_p = self
                                                            .library
                                                            .chapter_progress(&manga.id, &ch.id)
                                                            .map(|p| p.page_index)
                                                            .unwrap_or(0);
                                                        chapter_to_read = Some((ch.clone(), saved_p));
                                                    }
                                                });

                                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                    if z_chip(ui, tokens, "Read", Some("▶"), is_current) {
                                                        let saved_p = self
                                                            .library
                                                            .chapter_progress(&manga.id, &ch.id)
                                                            .map(|p| p.page_index)
                                                            .unwrap_or(0);
                                                        chapter_to_read = Some((ch.clone(), saved_p));
                                                    }

                                                    let active_dl_pct = self
                                                        .downloader
                                                        .jobs()
                                                        .into_iter()
                                                        .find(|j| j.chapter.id == ch.id)
                                                        .and_then(|j| match j.status {
                                                            DownloadStatus::Downloading {
                                                                completed_pages,
                                                                total_pages,
                                                            } => Some(
                                                                ((completed_pages * 100)
                                                                    / total_pages.max(1))
                                                                .min(100),
                                                            ),
                                                            _ => None,
                                                        });

                                                    let dl_lbl = if is_downloaded {
                                                        "✓ CBZ".to_string()
                                                    } else if let Some(pct_dl) = active_dl_pct {
                                                        format!("⏳ {}%", pct_dl)
                                                    } else {
                                                        "⬇ CBZ".to_string()
                                                    };
                                                    if z_chip(
                                                        ui,
                                                        tokens,
                                                        &dl_lbl,
                                                        None,
                                                        is_downloaded || active_dl_pct.is_some(),
                                                    ) && !is_downloaded
                                                        && active_dl_pct.is_none()
                                                    {
                                                        chapters_to_download.push(ch.clone());
                                                    }
                                                });
                                            });
                                        });
                                    ui.add_space(4.0);
                                }
                            });
                    }
                });
        });

        for ch in chapters_to_download {
            let reg = self.registry.clone();
            let dl = self.downloader.clone();
            let m_clone = manga.clone();
            let egui_ctx = ctx.clone();
            self.rt.spawn(async move {
                if let Some(src) = reg.get(&ch.source_id) {
                    if let Ok(pages) = src.pages(&ch).await {
                        let _ = dl.download_chapter(m_clone, ch, pages).await;
                        egui_ctx.request_repaint();
                    }
                }
            });
        }

        if let Some((ch, start_p)) = chapter_to_read {
            self.open_chapter_reader(ctx, manga, ch, start_p);
        }
    }

    fn info_row(ui: &mut egui::Ui, tokens: &GlassTokens, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new(label).size(12.5).color(tokens.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(truncate_str(value, 28))
                        .size(12.5)
                        .strong()
                        .color(tokens.text_primary),
                );
            });
        });
    }

    fn render_reader_screen(
        &mut self,
        ctx: &egui::Context,
        tokens: &GlassTokens,
        chapter: Chapter,
    ) {
        let pages = self.reader_pages.read().clone();
        let loaded_sections = self.reader_loaded_chapters.read().clone();
        let is_loading = *self.is_loading_pages.read();
        let is_loading_next_ch = *self.reader_loading_next_chapter.read();
        let total_pages = pages.len();
        let all_chapters_asc = self.sorted_chapters_oldest_first();
        let cur_ch_idx = all_chapters_asc.iter().position(|c| c.id == chapter.id);
        let prev_chapter = cur_ch_idx
            .and_then(|idx| idx.checked_sub(1))
            .and_then(|i| all_chapters_asc.get(i).cloned());
        let next_chapter = cur_ch_idx
            .and_then(|idx| all_chapters_asc.get(idx + 1).cloned());

        let effective_mode = match self.settings.reader_mode {
            ReaderModePreference::Auto => {
                if self.selected_manga.as_ref().map(|m| m.is_manhwa).unwrap_or(true) {
                    ReaderModePreference::Vertical
                } else {
                    ReaderModePreference::Single
                }
            }
            other => other,
        };

        // Keyboard controls in Reader (Escape, Tab, Up/Down/J/K/Space/PageDown/PageUp, Left/Right, +/-)
        let mut next_ch_requested: Option<Chapter> = None;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                if self.reader_show_settings_popup {
                    self.reader_show_settings_popup = false;
                } else {
                    self.active_reader_chapter = None;
                }
            }
            if i.key_pressed(egui::Key::Tab) {
                self.reader_show_settings_popup = !self.reader_show_settings_popup;
                if self.reader_show_settings_popup {
                    self.reader_show_hud = true;
                }
            }
            if i.key_pressed(egui::Key::H) {
                self.reader_show_hud = !self.reader_show_hud;
            }
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.reader_zoom = (self.reader_zoom + 0.15).min(2.6);
            }
            if i.key_pressed(egui::Key::Minus) {
                self.reader_zoom = (self.reader_zoom - 0.15).max(0.35);
            }
            if i.key_pressed(egui::Key::Num0) {
                self.reader_zoom = 1.0;
            }

            if effective_mode == ReaderModePreference::Vertical {
                if i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::J) {
                    self.pending_scroll_delta -= 240.0;
                }
                if i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::K) {
                    self.pending_scroll_delta += 240.0;
                }
                if i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::PageDown) {
                    self.pending_scroll_delta -= 620.0;
                }
                if i.key_pressed(egui::Key::PageUp) {
                    self.pending_scroll_delta += 620.0;
                }
            } else {
                let step = if effective_mode == ReaderModePreference::Double { 2 } else { 1 };
                let rtl = self.settings.reading_direction.eq_ignore_ascii_case("RTL");
                let forward_key = if rtl {
                    i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A)
                } else {
                    i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D)
                } || i.key_pressed(egui::Key::Space)
                    || i.key_pressed(egui::Key::ArrowDown);

                let backward_key = if rtl {
                    i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D)
                } else {
                    i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A)
                } || i.key_pressed(egui::Key::ArrowUp);

                if forward_key {
                    if self.reader_current_page + step < total_pages {
                        self.reader_current_page += step;
                    } else if let Some(ref nxt) = next_chapter {
                        next_ch_requested = Some(nxt.clone());
                    }
                }
                if backward_key {
                    if self.reader_current_page >= step {
                        self.reader_current_page -= step;
                    } else if self.reader_current_page > 0 {
                        self.reader_current_page = 0;
                    } else if let Some(ref prv) = prev_chapter {
                        next_ch_requested = Some(prv.clone());
                    }
                }
            }

            // Ctrl + Mouse Wheel Zoom
            if i.modifiers.ctrl && i.raw_scroll_delta.y.abs() > 0.1 {
                if i.raw_scroll_delta.y > 0.0 {
                    self.reader_zoom = (self.reader_zoom + 0.10).min(2.6);
                } else {
                    self.reader_zoom = (self.reader_zoom - 0.10).max(0.35);
                }
            }
        });

        // Floating Top Reader HUD Bar (matching DesktopReaderView.kt)
        if self.reader_show_hud {
            egui::TopBottomPanel::top("zibra_reader_top_hud")
                .exact_height(52.0)
                .frame(
                    egui::Frame::none()
                        .fill(tokens.surface_container_high)
                        .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                        .inner_margin(Margin::symmetric(16.0, 8.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        if z_icon_button(ui, tokens, NativeIcon::Back, Vec2::new(32.0, 32.0), false, "Back") {
                            self.active_reader_chapter = None;
                        }
                        ui.add_space(6.0);

                        if let Some(ref prv) = prev_chapter {
                            if z_chip(ui, tokens, "Prev Ch", None, false) {
                                next_ch_requested = Some(prv.clone());
                            }
                        }

                        ui.vertical(|ui| {
                            if let Some(ref m) = self.selected_manga {
                                ui.label(
                                    RichText::new(truncate_str(&m.title, 32))
                                        .size(13.0)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                            }
                            ui.label(
                                RichText::new(truncate_str(&chapter.title, 38))
                                    .size(11.0)
                                    .color(tokens.primary),
                            );
                        });

                        if let Some(ref nxt) = next_chapter {
                            if z_chip(ui, tokens, "Next Ch", None, false) {
                                next_ch_requested = Some(nxt.clone());
                            }
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if z_icon_button(
                                ui,
                                tokens,
                                NativeIcon::Settings,
                                Vec2::new(32.0, 32.0),
                                self.reader_show_settings_popup,
                                "Reader Settings",
                            ) {
                                self.reader_show_settings_popup = !self.reader_show_settings_popup;
                            }

                            if z_icon_button(
                                ui,
                                tokens,
                                NativeIcon::Fullscreen,
                                Vec2::new(32.0, 32.0),
                                self.is_fullscreen,
                                if self.is_fullscreen { "Exit Fullscreen (F11)" } else { "Enter Fullscreen (F11)" },
                            ) {
                                self.is_fullscreen = !self.is_fullscreen;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                            }

                            if z_icon_button(ui, tokens, NativeIcon::ZoomIn, Vec2::new(30.0, 30.0), false, "Zoom In (+)") {
                                self.reader_zoom = (self.reader_zoom + 0.15).min(2.6);
                            }
                            ui.label(
                                RichText::new(format!("{:.0}%", self.reader_zoom * 100.0))
                                    .size(12.0)
                                    .strong()
                                    .color(tokens.text_primary),
                            );
                            if z_icon_button(ui, tokens, NativeIcon::ZoomOut, Vec2::new(30.0, 30.0), false, "Zoom Out (-)") {
                                self.reader_zoom = (self.reader_zoom - 0.15).max(0.35);
                            }

                            // Mode quick switcher
                            for mode in [
                                ReaderModePreference::Vertical,
                                ReaderModePreference::Single,
                                ReaderModePreference::Double,
                            ] {
                                if z_chip(
                                    ui,
                                    tokens,
                                    mode.short_label(),
                                    None,
                                    effective_mode == mode,
                                ) {
                                    self.settings.reader_mode = mode;
                                    let _ = self.settings_store.save(&self.settings);
                                }
                            }

                            // Page Counter Pill
                            if total_pages > 0 {
                                egui::Frame::none()
                                    .fill(tokens.surface_card)
                                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::symmetric(10.0, 4.0))
                                    .show(ui, |ui| {
                                        let label = if effective_mode == ReaderModePreference::Double
                                            && self.reader_current_page + 1 < total_pages
                                        {
                                            format!(
                                                "Pages {}-{} / {}",
                                                self.reader_current_page + 1,
                                                self.reader_current_page + 2,
                                                total_pages
                                            )
                                        } else {
                                            format!(
                                                "Page {} / {}",
                                                (self.reader_current_page + 1).min(total_pages),
                                                total_pages
                                            )
                                        };
                                        ui.label(
                                            RichText::new(label)
                                                .size(12.0)
                                                .strong()
                                                .color(tokens.primary),
                                        );
                                    });
                            }
                        });
                    });
                });
        }

        // Bottom HUD Bar for Single / Double Page Modes
        if self.reader_show_hud && effective_mode != ReaderModePreference::Vertical && total_pages > 0 {
            egui::TopBottomPanel::bottom("zibra_reader_bottom_hud")
                .exact_height(48.0)
                .frame(
                    egui::Frame::none()
                        .fill(tokens.surface_container_high)
                        .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                        .inner_margin(Margin::symmetric(18.0, 6.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        let step = if effective_mode == ReaderModePreference::Double { 2 } else { 1 };
                        if z_chip(ui, tokens, "Prev Page", None, false) {
                            if self.reader_current_page >= step {
                                self.reader_current_page -= step;
                            } else if let Some(ref prv) = prev_chapter {
                                next_ch_requested = Some(prv.clone());
                            }
                        }

                        let mut page_slider = self.reader_current_page + 1;
                        if ui
                            .add(
                                egui::Slider::new(&mut page_slider, 1..=total_pages.max(1))
                                    .text("Page"),
                            )
                            .changed()
                        {
                            self.reader_current_page = page_slider.saturating_sub(1);
                        }

                        if z_chip(ui, tokens, "Next Page", None, false) {
                            if self.reader_current_page + step < total_pages {
                                self.reader_current_page += step;
                            } else if let Some(ref nxt) = next_chapter {
                                next_ch_requested = Some(nxt.clone());
                            }
                        }
                    });
                });
        }

        // Reader Settings Popover Card (matching KotatsuReaderSettingsCard in DesktopReaderView.kt)
        if self.reader_show_settings_popup {
            egui::Window::new("⚙ Reader Settings")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(365.0, 440.0))
                .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-20.0, 60.0))
                .show(ctx, |ui| {
                    ui.label(RichText::new("READING MODE").size(11.5).strong().color(tokens.primary));
                    ui.horizontal(|ui| {
                        for m in ReaderModePreference::ALL {
                            if z_chip(ui, tokens, m.short_label(), None, self.settings.reader_mode == *m) {
                                self.settings.reader_mode = *m;
                                let _ = self.settings_store.save(&self.settings);
                            }
                        }
                    });
                    ui.add_space(8.0);

                    ui.label(RichText::new("IMAGE ADJUSTMENTS").size(11.5).strong().color(tokens.primary));
                    let mut adj_changed = false;
                    adj_changed |= ui
                        .add(egui::Slider::new(&mut self.reader_contrast, 0.5..=1.5).text("Contrast"))
                        .changed();
                    adj_changed |= ui
                        .add(egui::Slider::new(&mut self.reader_saturation, 0.0..=2.0).text("Saturation"))
                        .changed();
                    adj_changed |= ui
                        .add(egui::Slider::new(&mut self.reader_brightness, 0.5..=1.5).text("Brightness"))
                        .changed();
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.reader_invert_colors, "Invert Colors").changed() {
                            adj_changed = true;
                        }
                        if z_chip(ui, tokens, "Reset Adjustments", None, false) {
                            self.reader_contrast = 1.0;
                            self.reader_saturation = 1.0;
                            self.reader_brightness = 1.0;
                            self.reader_invert_colors = false;
                            adj_changed = true;
                        }
                    });
                    if adj_changed {
                        self.reader_textures.clear();
                    }
                    ui.add_space(8.0);

                    ui.label(RichText::new("BACKGROUND COLOR").size(11.5).strong().color(tokens.primary));
                    ui.horizontal(|ui| {
                        for bg in ReaderBgPreset::ALL {
                            if z_chip(ui, tokens, bg.label(), None, self.reader_bg == *bg) {
                                self.reader_bg = *bg;
                            }
                        }
                    });
                    ui.add_space(8.0);

                    ui.label(RichText::new("WEBTOON & CONTINUOUS SCROLL").size(11.5).strong().color(tokens.primary));
                    if ui
                        .checkbox(
                            &mut self.settings.infinite_chapter_scroll,
                            "Infinite Chapter Scroll (Auto-append next chapter)",
                        )
                        .changed()
                    {
                        let _ = self.settings_store.save(&self.settings);
                    }
                    if ui
                        .checkbox(&mut self.settings.fit_to_width, "Fit Webtoon Strip to Window Width")
                        .changed()
                    {
                        let _ = self.settings_store.save(&self.settings);
                    }
                    if ui
                        .checkbox(&mut self.settings.auto_crop_margins, "Auto-Crop White Margins")
                        .changed()
                    {
                        let _ = self.settings_store.save(&self.settings);
                    }
                    if ui
                        .add(
                            egui::Slider::new(&mut self.settings.seam_trim_px, 0..=12)
                                .text("White-Seam Trim (px)"),
                        )
                        .changed()
                    {
                        let _ = self.settings_store.save(&self.settings);
                    }
                    ui.add(
                        egui::Slider::new(&mut self.reader_page_gap, 0.0..=24.0)
                            .text("Page Gap (px)"),
                    );
                    if ui
                        .add(
                            egui::Slider::new(&mut self.settings.preload_pages_ahead, 2..=12)
                                .text("Preload Pages Ahead"),
                        )
                        .changed()
                    {
                        let _ = self.settings_store.save(&self.settings);
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Shortcuts: Space/J/Down (Scroll), K/Up, Tab (HUD), F11 (Fullscreen), Esc (Exit)")
                            .size(10.5)
                            .color(tokens.text_muted),
                    );
                });
        }

        // Main Reader Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.reader_bg.color()))
            .show(ctx, |ui| {
                if is_loading && pages.is_empty() && loaded_sections.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(120.0);
                            ui.spinner();
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("Fetching & slicing 2048px GPU chapter tiles…")
                                    .size(14.0)
                                    .color(tokens.text_primary),
                            );
                        });
                    });
                    return;
                }

                if pages.is_empty() && loaded_sections.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(120.0);
                            ui.label(
                                RichText::new("No pages returned for this chapter.")
                                    .size(16.0)
                                    .strong()
                                    .color(tokens.text_primary),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Check source domain or cookies in Settings, or click Retry below.")
                                    .size(12.5)
                                    .color(tokens.text_muted),
                            );
                            ui.add_space(12.0);
                            if let Some(ref m) = self.selected_manga.clone() {
                                if z_chip(ui, tokens, "Retry Chapter", Some("🔄"), true) {
                                    self.open_chapter_reader(ctx, m.clone(), chapter.clone(), 0);
                                }
                            }
                        });
                    });
                    return;
                }

                let effective_trim = if self.settings.auto_crop_margins {
                    self.settings.seam_trim_px.max(4)
                } else {
                    self.settings.seam_trim_px
                };

                match effective_mode {
                    ReaderModePreference::Vertical | ReaderModePreference::Auto => {
                        let sections = if loaded_sections.is_empty() {
                            vec![LoadedChapterSection {
                                chapter: chapter.clone(),
                                chapter_index: cur_ch_idx.unwrap_or(0) + 1,
                                total_chapters: all_chapters_asc.len().max(1),
                                pages: pages.clone(),
                            }]
                        } else {
                            loaded_sections
                        };

                        // Flatten all pages across all loaded chapters for continuous vertical reading
                        let mut flat_pages: Vec<(Chapter, MangaPage, usize)> = Vec::new();
                        for sec in &sections {
                            let sec_total = sec.pages.len().max(1);
                            for p in &sec.pages {
                                flat_pages.push((sec.chapter.clone(), p.clone(), sec_total));
                            }
                        }

                        // Rolling Prefetch around current visible page across loaded sections
                        let cur_flat_idx = flat_pages
                            .iter()
                            .position(|(ch, p, _)| {
                                ch.id == chapter.id && p.index == self.reader_current_page
                            })
                            .unwrap_or(0);
                        let preload_to = (cur_flat_idx + self.settings.preload_pages_ahead.max(5))
                            .min(flat_pages.len().saturating_sub(1));
                        let preload_from = cur_flat_idx.saturating_sub(3);
                        if !flat_pages.is_empty() {
                            for (_, p, _) in &flat_pages[preload_from..=preload_to] {
                                if matches!(
                                    self.reader_pipeline.get_status(&p.image_url),
                                    PageLoadStatus::Idle
                                ) {
                                    let pipe = self.reader_pipeline.clone();
                                    let p_clone = p.clone();
                                    let egui_ctx = ctx.clone();
                                    self.rt.spawn(async move {
                                        let _ = pipe.load_and_slice_page(&p_clone, effective_trim).await;
                                        egui_ctx.request_repaint();
                                    });
                                }
                            }
                        }

                        // Auto-append next chapter when within 5 pages of the end of the last loaded chapter (matching DesktopReaderView.kt L204-220)
                        if self.settings.infinite_chapter_scroll
                            && !is_loading_next_ch
                            && !flat_pages.is_empty()
                            && cur_flat_idx + 5 >= flat_pages.len()
                        {
                            if let Some(last_sec) = sections.last() {
                                if let Some(last_pos) =
                                    all_chapters_asc.iter().position(|c| c.id == last_sec.chapter.id)
                                {
                                    if let Some(next_ch_to_append) =
                                        all_chapters_asc.get(last_pos + 1).cloned()
                                    {
                                        let already_loaded = sections
                                            .iter()
                                            .any(|s| s.chapter.id == next_ch_to_append.id);
                                        if !already_loaded {
                                            *self.reader_loading_next_chapter.write() = true;
                                            let reg = self.registry.clone();
                                            let dl = self.downloader.clone();
                                            let loaded_lock = self.reader_loaded_chapters.clone();
                                            let loading_next_lock =
                                                self.reader_loading_next_chapter.clone();
                                            let pipe = self.reader_pipeline.clone();
                                            let status_lock = self.status_banner.clone();
                                            let egui_ctx = ctx.clone();
                                            let next_idx_num = last_pos + 2;
                                            let total_chs_num = all_chapters_asc.len().max(1);
                                            let manga_id_opt =
                                                self.selected_manga.as_ref().map(|m| m.id.clone());

                                            self.rt.spawn(async move {
                                                let offline = manga_id_opt
                                                    .as_deref()
                                                    .map(|mid| {
                                                        dl.downloaded_for_manga(mid)
                                                            .into_iter()
                                                            .find(|d| {
                                                                d.chapter_id == next_ch_to_append.id
                                                                    || d.chapter_title
                                                                        == next_ch_to_append.title
                                                            })
                                                            .map(|d| dl.local_pages(&d))
                                                            .unwrap_or_default()
                                                    })
                                                    .unwrap_or_default();

                                                let res = if !offline.is_empty() {
                                                    Ok(offline)
                                                } else if let Some(src) =
                                                    reg.get(&next_ch_to_append.source_id)
                                                {
                                                    src.pages(&next_ch_to_append).await
                                                } else {
                                                    Err(anyhow::anyhow!("Source not found"))
                                                };

                                                if let Ok(next_pages) = res {
                                                    if !next_pages.is_empty() {
                                                        for p in next_pages.iter().take(5) {
                                                            let pipe_c = pipe.clone();
                                                            let p_c = p.clone();
                                                            tokio::spawn(async move {
                                                                let _ = pipe_c
                                                                    .load_and_slice_page(
                                                                        &p_c,
                                                                        effective_trim,
                                                                    )
                                                                    .await;
                                                            });
                                                        }
                                                        *status_lock.write() = format!(
                                                            "Appended next chapter: {}",
                                                            next_ch_to_append.title
                                                        );
                                                        loaded_lock.write().push(
                                                            LoadedChapterSection {
                                                                chapter: next_ch_to_append,
                                                                chapter_index: next_idx_num,
                                                                total_chapters: total_chs_num,
                                                                pages: next_pages,
                                                            },
                                                        );
                                                    }
                                                }
                                                *loading_next_lock.write() = false;
                                                egui_ctx.request_repaint();
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        let viewport_rect = ui.max_rect();
                        let viewport_center_y = viewport_rect.center().y;
                        let base_fraction = if self.settings.fit_to_width { 0.86 } else { 0.68 };
                        let avail_w = (ui.available_width() * base_fraction * self.reader_zoom)
                            .clamp(280.0, 1800.0);

                        let mut new_visible_ch_and_page: Option<(Chapter, usize, Vec<MangaPage>)> =
                            None;

                        let manga_id_salt = self
                            .selected_manga
                            .as_ref()
                            .map(|m| m.id.as_str())
                            .unwrap_or("reader");

                        ScrollArea::vertical()
                            .id_salt(("zibra_reader_continuous_scroll", manga_id_salt))
                            .auto_shrink([false, false])
                            .drag_to_scroll(true)
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
                            .show(ui, |ui| {
                                if self.pending_scroll_delta.abs() > 0.5 {
                                    ui.scroll_with_delta(Vec2::new(0.0, self.pending_scroll_delta));
                                    self.pending_scroll_delta = 0.0;
                                }

                                ui.set_min_width(ui.available_width());
                                ui.vertical_centered(|ui| {
                                    ui.spacing_mut().item_spacing.y = self.reader_page_gap;

                                    let cur_p = self.reader_current_page;
                                    let min_p = cur_p.saturating_sub(2);
                                    let max_p = cur_p + 3;

                                    // Windowed memory management: retain only [cur_p - 2, cur_p + 3] active pages in GPU and RAM
                                    let mut active_urls: HashSet<String> = HashSet::new();
                                    for sec in sections.iter() {
                                        for p in &sec.pages {
                                            if p.index >= min_p && p.index <= max_p {
                                                active_urls.insert(p.image_url.clone());
                                            }
                                        }
                                    }
                                    self.reader_textures.retain(|url, _| active_urls.contains(url));
                                    let active_list: Vec<String> = active_urls.into_iter().collect();
                                    self.reader_pipeline.prune_except(&active_list);

                                    // Asynchronously preload upcoming 3 pages
                                    for sec in sections.iter() {
                                        for p in &sec.pages {
                                            if p.index > cur_p && p.index <= cur_p + 3 {
                                                if matches!(self.reader_pipeline.get_status(&p.image_url), PageLoadStatus::Idle) {
                                                    let pipe = self.reader_pipeline.clone();
                                                    let p_clone = p.clone();
                                                    let effective_trim = if self.settings.auto_crop_margins {
                                                        self.settings.seam_trim_px.max(4)
                                                    } else {
                                                        self.settings.seam_trim_px
                                                    };
                                                    let egui_ctx = ctx.clone();
                                                    self.rt.spawn(async move {
                                                        let _ = pipe.load_and_slice_page(&p_clone, effective_trim).await;
                                                        egui_ctx.request_repaint();
                                                    });
                                                }
                                            }
                                        }
                                    }

                                    for (sec_idx, sec) in sections.iter().enumerate() {
                                        // Chapter Divider Banner between appended chapters (matching DesktopReaderView.kt ContinuousReaderItem.ChapterDivider)
                                        if sec_idx > 0 {
                                            ui.add_space(22.0);
                                            egui::Frame::none()
                                                .fill(tokens.surface_container_high)
                                                .stroke(Stroke::new(1.5_f32, tokens.primary))
                                                .rounding(Rounding::same(14.0))
                                                .inner_margin(Margin::symmetric(24.0, 16.0))
                                                .show(ui, |ui| {
                                                    ui.set_width(avail_w.min(620.0));
                                                    ui.vertical_centered(|ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "CHAPTER {} OF {}",
                                                                sec.chapter_index,
                                                                sec.total_chapters
                                                            ))
                                                            .size(11.5)
                                                            .strong()
                                                            .color(tokens.primary),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.label(
                                                            RichText::new(&sec.chapter.title)
                                                                .size(15.0)
                                                                .strong()
                                                                .color(tokens.text_primary),
                                                        );
                                                    });
                                                });
                                            ui.add_space(18.0);
                                        }

                                        let sec_pages_len = sec.pages.len();
                                        for page in &sec.pages {
                                            let page_start_y = ui.cursor().min.y;
                                            let in_window = page.index >= min_p && page.index <= max_p;
                                            if in_window {
                                                match self.reader_pipeline.get_status(&page.image_url) {
                                                PageLoadStatus::Ready(sliced) => {
                                                    let c_adj = self.reader_contrast;
                                                    let s_adj = self.reader_saturation;
                                                    let b_adj = self.reader_brightness;
                                                    let inv_adj = self.reader_invert_colors;
                                                    let handles = self
                                                        .reader_textures
                                                        .entry(page.image_url.clone())
                                                        .or_insert_with(|| {
                                                            sliced_to_textures_adjusted(
                                                                ctx,
                                                                &page.image_url,
                                                                &sliced,
                                                                c_adj,
                                                                s_adj,
                                                                b_adj,
                                                                inv_adj,
                                                            )
                                                        });

                                                    // Zero gap between 2048px tiles of the same page
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.y = 0.0;
                                                        for (idx, tex) in handles.iter().enumerate()
                                                        {
                                                            let tile = &sliced.tiles[idx];
                                                            let scale = avail_w
                                                                / (tile.width.max(1) as f32);
                                                            let draw_h =
                                                                (tile.height as f32) * scale;
                                                            let img_resp = ui.add(
                                                                egui::Image::from_texture(tex)
                                                                    .fit_to_exact_size(Vec2::new(
                                                                        avail_w, draw_h,
                                                                    ))
                                                                    .sense(Sense::click()),
                                                            );
                                                            if img_resp.clicked() {
                                                                self.reader_show_hud =
                                                                    !self.reader_show_hud;
                                                            }
                                                        }
                                                    });
                                                }
                                                PageLoadStatus::Failed(err_msg) => {
                                                    egui::Frame::none()
                                                        .fill(tokens.surface_card)
                                                        .stroke(Stroke::new(
                                                            1.0_f32,
                                                            tokens.border_glass,
                                                        ))
                                                        .rounding(Rounding::same(12.0))
                                                        .inner_margin(Margin::same(24.0))
                                                        .show(ui, |ui| {
                                                            ui.set_width(avail_w);
                                                            ui.vertical_centered(|ui| {
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "⚠ Page {} failed to load",
                                                                        page.index + 1
                                                                    ))
                                                                    .strong()
                                                                    .color(Color32::from_rgb(
                                                                        255, 107, 107,
                                                                    )),
                                                                );
                                                                ui.label(
                                                                    RichText::new(truncate_str(
                                                                        &err_msg, 80,
                                                                    ))
                                                                    .size(11.0)
                                                                    .color(tokens.text_muted),
                                                                );
                                                                ui.add_space(6.0);
                                                                if z_chip(
                                                                    ui,
                                                                    tokens,
                                                                    "Retry Page",
                                                                    Some("🔄"),
                                                                    true,
                                                                ) {
                                                                    self.reader_pipeline
                                                                        .clear_failed(
                                                                            &page.image_url,
                                                                        );
                                                                }
                                                            });
                                                        });
                                                }
                                                PageLoadStatus::Loading | PageLoadStatus::Idle => {
                                                    let placeholder_h =
                                                        (avail_w * 1.35).clamp(360.0, 900.0);
                                                    let (rect, _) = ui.allocate_exact_size(
                                                        Vec2::new(avail_w, placeholder_h),
                                                        Sense::hover(),
                                                    );
                                                    if ui.is_rect_visible(rect) {
                                                        ui.painter().rect_filled(
                                                            rect,
                                                            Rounding::same(8.0),
                                                            Color32::from_rgba_unmultiplied(
                                                                24, 28, 38, 180,
                                                            ),
                                                        );
                                                        ui.painter().text(
                                                            rect.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            format!(
                                                                "Loading Page {} / {}…",
                                                                page.index + 1,
                                                                sec_pages_len
                                                            ),
                                                            egui::FontId::proportional(13.0),
                                                            tokens.text_muted,
                                                        );
                                                    }
                                                }
                                            }
                                        } else {
                                            let placeholder_h = match self.reader_pipeline.get_status(&page.image_url) {
                                                PageLoadStatus::Ready(ref s) => (s.total_height as f32) * (avail_w / (s.total_width.max(1) as f32)),
                                                _ => (avail_w * 1.35).clamp(360.0, 900.0),
                                            };
                                            let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, placeholder_h), Sense::hover());
                                            if ui.is_rect_visible(rect) {
                                                ui.painter().rect_filled(rect, Rounding::ZERO, tokens.surface_container_high);
                                            }
                                        }

                                            let page_end_y = ui.cursor().min.y;
                                            if page_start_y <= viewport_center_y
                                                && viewport_center_y <= page_end_y
                                            {
                                                new_visible_ch_and_page = Some((
                                                    sec.chapter.clone(),
                                                    page.index,
                                                    sec.pages.clone(),
                                                ));
                                            }
                                        }
                                    }

                                    if is_loading_next_ch {
                                        ui.add_space(20.0);
                                        ui.spinner();
                                        ui.label(
                                            RichText::new("Loading next chapter seamlessly…")
                                                .size(12.5)
                                                .color(tokens.primary),
                                        );
                                    }

                                    // End-of-Manga / Chapter Transition Card (matching DesktopReaderView.kt)
                                    let last_loaded_ch = sections
                                        .last()
                                        .map(|s| s.chapter.clone())
                                        .unwrap_or_else(|| chapter.clone());
                                    let last_ch_pos = all_chapters_asc
                                        .iter()
                                        .position(|c| c.id == last_loaded_ch.id);
                                    let next_after_last = last_ch_pos
                                        .and_then(|idx| all_chapters_asc.get(idx + 1).cloned());

                                    ui.add_space(28.0);
                                    egui::Frame::none()
                                        .fill(tokens.surface_container_high)
                                        .stroke(Stroke::new(1.5_f32, tokens.border_glow))
                                        .rounding(Rounding::same(16.0))
                                        .inner_margin(Margin::symmetric(28.0, 22.0))
                                        .show(ui, |ui| {
                                            ui.set_width(avail_w.min(560.0));
                                            ui.vertical_centered(|ui| {
                                                if next_after_last.is_none() {
                                                    ui.label(
                                                        RichText::new("✓  You're all caught up!")
                                                            .size(17.0)
                                                            .strong()
                                                            .color(tokens.primary),
                                                    );
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        RichText::new(
                                                            "You've reached the final page of the latest chapter.",
                                                        )
                                                        .size(12.5)
                                                        .color(tokens.text_muted),
                                                    );
                                                } else {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "✓  Finished {}",
                                                            last_loaded_ch.title
                                                        ))
                                                        .size(16.0)
                                                        .strong()
                                                        .color(tokens.primary),
                                                    );
                                                }
                                                ui.add_space(12.0);
                                                ui.horizontal(|ui| {
                                                    if let Some(ref prv) = prev_chapter {
                                                        if z_chip(
                                                            ui,
                                                            tokens,
                                                            &format!(
                                                                "⏮  Prev: {}",
                                                                truncate_str(&prv.title, 22)
                                                            ),
                                                            None,
                                                            false,
                                                        ) {
                                                            next_ch_requested = Some(prv.clone());
                                                        }
                                                    }
                                                    if let Some(ref nxt) = next_after_last {
                                                        if z_chip(
                                                            ui,
                                                            tokens,
                                                            &format!(
                                                                "▶  Next: {}",
                                                                truncate_str(&nxt.title, 24)
                                                            ),
                                                            None,
                                                            true,
                                                        ) {
                                                            next_ch_requested = Some(nxt.clone());
                                                        }
                                                    } else if z_chip(
                                                        ui,
                                                        tokens,
                                                        "⬅ Close Reader",
                                                        None,
                                                        true,
                                                    ) {
                                                        self.active_reader_chapter = None;
                                                    }
                                                });
                                            });
                                        });
                                    ui.add_space(80.0);
                                });
                            });

                        // Update active chapter & reading progress as user scrolls across chapters
                        if let Some((vis_ch, vis_page, vis_pages)) = new_visible_ch_and_page {
                            if vis_ch.id != chapter.id || vis_page != self.reader_current_page {
                                self.active_reader_chapter = Some(vis_ch.clone());
                                self.reader_current_page = vis_page;
                                *self.reader_pages.write() = vis_pages;
                                if let Some(ref m) = self.selected_manga {
                                    self.library.save_progress(&ReadingProgress {
                                        manga_id: m.id.clone(),
                                        chapter_id: vis_ch.id.clone(),
                                        page_index: vis_page,
                                        scroll_offset_y: 0.0,
                                        chapter_number: vis_ch.number,
                                        updated_at: chrono::Utc::now().timestamp_millis(),
                                    });
                                }
                            }
                        }

                        // Window-aware GPU texture eviction around current visible position
                        if self.reader_textures.len() > 28 && !flat_pages.is_empty() {
                            let win_start = cur_flat_idx.saturating_sub(8);
                            let win_end =
                                (cur_flat_idx + 14).min(flat_pages.len().saturating_sub(1));
                            let keep_urls: HashSet<String> = flat_pages[win_start..=win_end]
                                .iter()
                                .map(|(_, p, _)| p.image_url.clone())
                                .collect();
                            self.reader_textures.retain(|url, _| keep_urls.contains(url));
                        }
                    }

                    ReaderModePreference::Single | ReaderModePreference::Double => {
                        // Rolling Prefetch and Windowed Memory Management for Single / Double Page Mode
                        let cur_idx = self.reader_current_page.min(pages.len().saturating_sub(1));
                        let min_p = cur_idx.saturating_sub(2);
                        let max_p = (cur_idx + 3).min(pages.len().saturating_sub(1));

                        // Retain only [cur_p - 2, cur_p + 3] active pages in GPU and RAM
                        let mut active_urls: HashSet<String> = HashSet::new();
                        for p in &pages[min_p..=max_p] {
                            active_urls.insert(p.image_url.clone());
                        }
                        self.reader_textures.retain(|url, _| active_urls.contains(url));
                        let active_list: Vec<String> = active_urls.into_iter().collect();
                        self.reader_pipeline.prune_except(&active_list);

                        let preload_to = (cur_idx + self.settings.preload_pages_ahead.max(3))
                            .min(pages.len().saturating_sub(1));
                        for p in &pages[min_p..=preload_to] {
                            if matches!(
                                self.reader_pipeline.get_status(&p.image_url),
                                PageLoadStatus::Idle
                            ) {
                                let pipe = self.reader_pipeline.clone();
                                let p_clone = p.clone();
                                let egui_ctx = ctx.clone();
                                self.rt.spawn(async move {
                                    let _ =
                                        pipe.load_and_slice_page(&p_clone, effective_trim).await;
                                    egui_ctx.request_repaint();
                                });
                            }
                        }

                        let is_double = effective_mode == ReaderModePreference::Double;
                        let avail_h = (ui.available_height() - 16.0).max(200.0) * self.reader_zoom;
                        let page1 = pages.get(self.reader_current_page);
                        let page2 = if is_double {
                            pages.get(self.reader_current_page + 1)
                        } else {
                            None
                        };

                        let pair: Vec<Option<&MangaPage>> =
                            if is_double && self.settings.reading_direction.eq_ignore_ascii_case("RTL") {
                                vec![page2, page1]
                            } else {
                                vec![page1, page2]
                            };

                        ScrollArea::both()
                            .id_salt("zibra_reader_paged_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.horizontal(|ui| {
                                        for p_opt in pair {
                                            let Some(p) = p_opt else { continue };
                                            if let PageLoadStatus::Ready(sliced) =
                                                self.reader_pipeline.get_status(&p.image_url)
                                            {
                                                let c_adj = self.reader_contrast;
                                                let s_adj = self.reader_saturation;
                                                let b_adj = self.reader_brightness;
                                                let inv_adj = self.reader_invert_colors;
                                                let handles = self
                                                    .reader_textures
                                                    .entry(p.image_url.clone())
                                                    .or_insert_with(|| {
                                                        sliced_to_textures_adjusted(
                                                            ctx,
                                                            &p.image_url,
                                                            &sliced,
                                                            c_adj,
                                                            s_adj,
                                                            b_adj,
                                                            inv_adj,
                                                        )
                                                    });
                                                let scale = avail_h
                                                    / (sliced.total_height.max(1) as f32);
                                                let draw_w = (sliced.total_width as f32) * scale;
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 0.0;
                                                    for (idx, tex) in handles.iter().enumerate() {
                                                        let tile = &sliced.tiles[idx];
                                                        let tile_h = (tile.height as f32) * scale;
                                                        ui.add(
                                                            egui::Image::from_texture(tex)
                                                                .fit_to_exact_size(Vec2::new(
                                                                    draw_w, tile_h,
                                                                )),
                                                        );
                                                    }
                                                });
                                            } else {
                                                ui.spinner();
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Loading Page {}…",
                                                        p.index + 1
                                                    ))
                                                    .color(tokens.text_muted),
                                                );
                                            }
                                        }
                                    });
                                });
                            });
                    }
                }
            });

        if let Some(next_ch) = next_ch_requested {
            if let Some(m) = self.selected_manga.clone() {
                self.open_chapter_reader(ctx, m, next_ch, 0);
            }
        }
    }

    fn render_feed_screen(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tokens: &GlassTokens,
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("UPDATES & FEED")
                        .size(13.0)
                        .strong()
                        .color(tokens.primary),
                );
                ui.label(
                    RichText::new("Latest chapters from your saved Favourites library.")
                        .size(12.0)
                        .color(tokens.text_muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if z_chip(ui, tokens, "Refresh Feed", Some("🔄"), true) {
                    let favs = self.library.list_favorites();
                    let reg = self.registry.clone();
                    let lib = self.library.clone();
                    let status = self.status_banner.clone();
                    let egui_ctx = ctx.clone();
                    self.rt.spawn(async move {
                        *status.write() = "Refreshing favourites for new chapters…".to_string();
                        for m in favs {
                            if let Some(src) = reg.get(&m.source_id) {
                                if let Ok(chs) = src.chapters(&m).await {
                                    if !chs.is_empty() {
                                        lib.cache_chapters(&m.id, &chs);
                                    }
                                }
                            }
                        }
                        *status.write() = "Feed updated.".to_string();
                        egui_ctx.request_repaint();
                    });
                }
            });
        });
        ui.add_space(14.0);

        let favs = self.library.list_favorites();
        if favs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label(RichText::new("📡").size(44.0).color(tokens.primary));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Your feed is empty")
                            .size(18.0)
                            .strong()
                            .color(tokens.text_primary),
                    );
                    ui.label(
                        RichText::new("Add manga or manhwa to Favourites to track new releases here.")
                            .size(13.0)
                            .color(tokens.text_muted),
                    );
                    ui.add_space(12.0);
                    if z_chip(ui, tokens, "Explore Sources", Some("🧭"), true) {
                        self.current_screen = NavScreen::Explore;
                    }
                });
            });
            return;
        }

        let mut to_read: Option<(Manga, Chapter)> = None;
        let mut to_open_details: Option<Manga> = None;

        ScrollArea::vertical()
            .id_salt("feed_updates_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for m in favs {
                    let cached = self.library.get_cached_chapters(&m.id);
                    for ch in cached.into_iter().take(3) {
                        let is_read = self.library.chapter_progress(&m.id, &ch.id).is_some();
                        egui::Frame::none()
                            .fill(tokens.surface_card)
                            .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let resp = draw_remote_image(
                                        ui,
                                        ctx,
                                        &self.rt,
                                        &self.cover_cache,
                                        &mut self.cover_textures,
                                        tokens,
                                        m.cover_url.as_deref(),
                                        &m.title,
                                        Vec2::new(44.0, 60.0),
                                        8.0,
                                    );
                                    if resp.clicked() {
                                        to_open_details = Some(m.clone());
                                    }
                                    ui.add_space(10.0);
                                    ui.vertical(|ui| {
                                        if ui
                                            .add(
                                                egui::Label::new(
                                                    RichText::new(&m.title)
                                                        .size(14.0)
                                                        .strong()
                                                        .color(tokens.text_primary),
                                                )
                                                .sense(Sense::click()),
                                            )
                                            .clicked()
                                        {
                                            to_open_details = Some(m.clone());
                                        }
                                        ui.add_space(3.0);
                                        ui.label(
                                            RichText::new(format!("{} • {}", ch.title, m.source_id))
                                                .size(12.0)
                                                .color(tokens.primary),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let btn_lbl = if is_read { "Re-read" } else { "▶ Read" };
                                        if z_chip(ui, tokens, btn_lbl, None, !is_read) {
                                            to_read = Some((m.clone(), ch.clone()));
                                        }
                                    });
                                });
                            });
                        ui.add_space(6.0);
                    }
                }
            });

        if let Some(m) = to_open_details {
            self.open_manga_details(ctx, m);
        }
        if let Some((m, ch)) = to_read {
            self.selected_manga = Some(m.clone());
            self.open_chapter_reader(ctx, m, ch, 0);
        }
    }

    fn render_settings_screen(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tokens: &GlassTokens,
    ) {
        let total_h = ui.available_height();
        ui.horizontal(|ui| {
            // Left Master Category Column (265px, matching SettingsScreen.kt)
            ui.allocate_ui_with_layout(
                Vec2::new(265.0, total_h),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::Frame::none()
                        .fill(tokens.surface_glass)
                        .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                        .rounding(Rounding::same(14.0))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("SETTINGS")
                                    .size(12.0)
                                    .strong()
                                    .color(tokens.primary),
                            );
                            ui.add_space(8.0);
                            for sec in SettingsSection::ALL {
                                let selected = self.settings_section == *sec;
                                let fill = if selected {
                                    tokens.primary_container
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let stroke = if selected {
                                    Stroke::new(1.5_f32, tokens.primary)
                                } else {
                                    Stroke::NONE
                                };
                                egui::Frame::none()
                                    .fill(fill)
                                    .stroke(stroke)
                                    .rounding(Rounding::same(10.0))
                                    .inner_margin(Margin::symmetric(10.0, 8.0))
                                    .show(ui, |ui| {
                                        ui.set_width(230.0);
                                        let resp = ui.interact(
                                            ui.max_rect(),
                                            egui::Id::new(("settings_sec", sec.title())),
                                            Sense::click(),
                                        );
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(sec.icon())
                                                    .size(16.0)
                                                    .color(if selected {
                                                        tokens.primary
                                                    } else {
                                                        tokens.text_muted
                                                    }),
                                            );
                                            ui.add_space(6.0);
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new(sec.title())
                                                        .size(13.0)
                                                        .strong()
                                                        .color(if selected {
                                                            tokens.primary
                                                        } else {
                                                            tokens.text_primary
                                                        }),
                                                );
                                                ui.label(
                                                    RichText::new(sec.subtitle())
                                                        .size(10.5)
                                                        .color(tokens.text_muted),
                                                );
                                            });
                                        });
                                        if resp.clicked() {
                                            self.settings_section = *sec;
                                        }
                                    });
                                ui.add_space(3.0);
                            }
                        });
                },
            );

            ui.add_space(14.0);

            // Right Detail Column
            let right_w = ui.available_width();
            egui::Frame::none()
                .fill(tokens.surface_glass_raised)
                .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                .rounding(Rounding::same(16.0))
                .inner_margin(Margin::same(22.0))
                .show(ui, |ui| {
                    ui.set_width((right_w - 46.0).max(280.0));
                    ui.set_height((total_h - 46.0).max(280.0));

                    ScrollArea::vertical()
                        .id_salt("settings_detail_pane_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| match self.settings_section {
                            SettingsSection::Appearance => {
                                ui.label(
                                    RichText::new("APPEARANCE & 13 KOTATSU PALETTES")
                                        .size(14.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.label(
                                    RichText::new(
                                        "Select any Material 3 scheme and toggle Dark/Light, pitch-black AMOLED, animations, or custom icon.",
                                    )
                                    .size(12.0)
                                    .color(tokens.text_muted),
                                );
                                ui.add_space(12.0);

                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                                    for theme in ColorTheme::ALL {
                                        let is_sel = self.settings.color_theme == *theme;
                                        let (r, g, b) = theme.primary_rgb();
                                        let swatch = Color32::from_rgb(r, g, b);
                                        egui::Frame::none()
                                            .fill(if is_sel {
                                                tokens.primary_container
                                            } else {
                                                tokens.surface_card
                                            })
                                            .stroke(Stroke::new(
                                                if is_sel { 2.0 } else { 1.0_f32 },
                                                if is_sel {
                                                    tokens.primary
                                                } else {
                                                    tokens.border_glass
                                                },
                                            ))
                                            .rounding(Rounding::same(12.0))
                                            .inner_margin(Margin::symmetric(12.0, 10.0))
                                            .show(ui, |ui| {
                                                ui.set_width(185.0);
                                                ui.horizontal(|ui| {
                                                    let (rect, _) = ui.allocate_exact_size(
                                                        Vec2::new(18.0, 18.0),
                                                        Sense::hover(),
                                                    );
                                                    ui.painter().circle_filled(
                                                        rect.center(),
                                                        9.0,
                                                        swatch,
                                                    );
                                                    ui.vertical(|ui| {
                                                        if ui
                                                            .add(
                                                                egui::Label::new(
                                                                    RichText::new(theme.title())
                                                                        .size(12.5)
                                                                        .strong()
                                                                        .color(if is_sel {
                                                                            tokens.primary
                                                                        } else {
                                                                            tokens.text_primary
                                                                        }),
                                                                )
                                                                .sense(Sense::click()),
                                                            )
                                                            .clicked()
                                                        {
                                                            self.settings.color_theme = *theme;
                                                            if *theme == ColorTheme::AmoledVoid {
                                                                self.settings.is_amoled = true;
                                                            }
                                                            let _ = self
                                                                .settings_store
                                                                .save(&self.settings);
                                                        }
                                                        ui.label(
                                                            RichText::new(theme.description())
                                                                .size(10.0)
                                                                .color(tokens.text_muted),
                                                        );
                                                    });
                                                });
                                            });
                                    }
                                });

                                ui.add_space(16.0);
                                ui.horizontal_wrapped(|ui| {
                                    if z_chip(
                                        ui,
                                        tokens,
                                        "Dark Mode",
                                        Some("🌙"),
                                        self.settings.theme == AppTheme::Dark,
                                    ) {
                                        self.settings.theme = AppTheme::Dark;
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    if z_chip(
                                        ui,
                                        tokens,
                                        "Light Mode",
                                        Some("☀"),
                                        self.settings.theme == AppTheme::Light,
                                    ) {
                                        self.settings.theme = AppTheme::Light;
                                        self.settings.is_amoled = false;
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    if z_chip(
                                        ui,
                                        tokens,
                                        "True Pitch-Black AMOLED (#000000)",
                                        Some("⬛"),
                                        self.settings.is_amoled,
                                    ) {
                                        self.settings.is_amoled = !self.settings.is_amoled;
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    let anim_smooth = matches!(
                                        self.settings.animation_level,
                                        zibra_core::AnimationLevel::Smooth
                                    );
                                    if z_chip(
                                        ui,
                                        tokens,
                                        "Smooth Spring Animations",
                                        Some("✨"),
                                        anim_smooth,
                                    ) {
                                        self.settings.animation_level = if anim_smooth {
                                            zibra_core::AnimationLevel::Off
                                        } else {
                                            zibra_core::AnimationLevel::Smooth
                                        };
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });

                                ui.add_space(14.0);
                                ui.label(
                                    RichText::new("CUSTOM APP BACKGROUND & WALLPAPER")
                                        .size(12.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.label(
                                    RichText::new(
                                        "Set custom desktop wallpaper (.png, .jpg, .webp). Colors automatically optimize for image luminance.",
                                    )
                                    .size(11.5)
                                    .color(tokens.text_muted),
                                );
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.custom_bg_input)
                                            .hint_text("/path/to/wallpaper.png…")
                                            .desired_width(380.0),
                                    );
                                    if ui.button("Apply Wallpaper").clicked() {
                                        let trimmed = self.custom_bg_input.trim();
                                        if !trimmed.is_empty() && std::path::Path::new(trimmed).exists() {
                                            self.settings.custom_background_path = Some(trimmed.to_string());
                                            self.bg_texture = None;
                                            let _ = self.settings_store.save(&self.settings);
                                        }
                                    }
                                    if self.settings.custom_background_path.is_some() {
                                        if ui.button("Clear Wallpaper").clicked() {
                                            self.settings.custom_background_path = None;
                                            self.custom_bg_input.clear();
                                            self.bg_texture = None;
                                            self.bg_avg_luminance = None;
                                            let _ = self.settings_store.save(&self.settings);
                                        }
                                    }
                                });
                                ui.add_space(6.0);
                                if ui.checkbox(&mut self.settings.auto_optimize_bg, "Auto-Optimize Theme for Background Luminance (contrast & clarity)").changed() {
                                    let _ = self.settings_store.save(&self.settings);
                                }

                                ui.add_space(14.0);
                                ui.label(
                                    RichText::new("INTERFACE LANGUAGE")
                                        .size(12.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.horizontal(|ui| {
                                    for lang in AppLanguage::ALL {
                                        if z_chip(
                                            ui,
                                            tokens,
                                            lang.label(),
                                            None,
                                            self.settings.language == *lang,
                                        ) {
                                            self.settings.language = *lang;
                                            let _ = self.settings_store.save(&self.settings);
                                        }
                                    }
                                });

                                ui.add_space(14.0);
                                ui.label(
                                    RichText::new("APPLICATION ICON PERSONALIZATION")
                                        .size(12.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                let mut icon_path_str =
                                    self.settings.custom_icon_path.clone().unwrap_or_default();
                                ui.horizontal(|ui| {
                                    ui.label("Custom Icon Path (.png / .ico):");
                                    if ui.text_edit_singleline(&mut icon_path_str).changed() {
                                        self.settings.custom_icon_path =
                                            if icon_path_str.trim().is_empty() {
                                                None
                                            } else {
                                                Some(icon_path_str.trim().to_string())
                                            };
                                        self.logo_texture = None;
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    if z_chip(ui, tokens, "Reset Default Icon", None, false) {
                                        self.settings.custom_icon_path = None;
                                        self.logo_texture = None;
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });
                            }

                            SettingsSection::Reader => {
                                ui.label(
                                    RichText::new("READER & CONTINUOUS CHAPTER PIPELINE")
                                        .size(14.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.label(
                                    RichText::new(
                                        "Configure default reading mode, direction, infinite chapter scrolling, and 2048px GPU tile slicing.",
                                    )
                                    .size(12.0)
                                    .color(tokens.text_muted),
                                );
                                ui.add_space(10.0);

                                ui.label(
                                    RichText::new("DEFAULT READING MODE")
                                        .size(12.0)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.add_space(4.0);
                                for mode in ReaderModePreference::ALL {
                                    if z_chip(
                                        ui,
                                        tokens,
                                        mode.label(),
                                        None,
                                        self.settings.reader_mode == *mode,
                                    ) {
                                        self.settings.reader_mode = *mode;
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    ui.add_space(4.0);
                                }

                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("PAGED READING DIRECTION")
                                        .size(12.0)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.horizontal(|ui| {
                                    let is_ltr =
                                        !self.settings.reading_direction.eq_ignore_ascii_case("RTL");
                                    if z_chip(
                                        ui,
                                        tokens,
                                        "Left-to-Right (Manhwa / Comic)",
                                        None,
                                        is_ltr,
                                    ) {
                                        self.settings.reading_direction = "LTR".to_string();
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    if z_chip(
                                        ui,
                                        tokens,
                                        "Right-to-Left (Japanese Manga)",
                                        None,
                                        !is_ltr,
                                    ) {
                                        self.settings.reading_direction = "RTL".to_string();
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });

                                ui.add_space(12.0);
                                if ui
                                    .checkbox(
                                        &mut self.settings.infinite_chapter_scroll,
                                        "Infinite Chapter Scroll (Automatically append & stream next chapters until the last chapter)",
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .checkbox(
                                        &mut self.settings.fit_to_width,
                                        "Fit Webtoon Strip to Window Width",
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .checkbox(
                                        &mut self.settings.auto_crop_margins,
                                        "Auto-Crop White Page Margins",
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(&mut self.settings.seam_trim_px, 0..=12)
                                            .text("Webtoon White-Seam Trim (px)"),
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut self.settings.preload_pages_ahead,
                                            1..=12,
                                        )
                                        .text("Preload Pages Ahead"),
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut self.settings.ram_cache_budget_mb,
                                            64..=512,
                                        )
                                        .text("Decoded GPU/RAM Tile Cache Budget (MB)"),
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                            }

                            SettingsSection::Sources => {
                                ui.label(
                                    RichText::new(format!(
                                        "SOURCES, 18+ NSFW FILTER & MIRRORS ({} Active / {} Total Sources)",
                                        self.registry.visible_count(self.settings.nsfw_enabled),
                                        self.registry.total_count()
                                    ))
                                    .size(14.0)
                                    .strong()
                                    .color(tokens.primary),
                                );
                                ui.add_space(10.0);

                                // 18+ / NSFW Filter Card
                                egui::Frame::none()
                                    .fill(tokens.surface_card)
                                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                    .rounding(Rounding::same(12.0))
                                    .inner_margin(Margin::same(14.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("CONTENT SAFETY & 18+ NSFW FILTER")
                                                .size(12.5)
                                                .strong()
                                                .color(tokens.primary),
                                        );
                                        ui.add_space(4.0);
                                        let mut disable_nsfw = !self.settings.nsfw_enabled;
                                        if ui
                                            .checkbox(
                                                &mut disable_nsfw,
                                                "Disable 18+ / NSFW Sources & Adult Content (Hides all +18 sources from Explore, Catalog & Search)",
                                            )
                                            .changed()
                                        {
                                            self.settings.nsfw_enabled = !disable_nsfw;
                                            let _ = self.settings_store.save(&self.settings);
                                        }
                                    });

                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new("Preferred Source Language")
                                        .size(12.5)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.horizontal_wrapped(|ui| {
                                    for lang in [
                                        "ALL", "ar", "en", "multi", "ja", "ko", "zh", "fr", "es",
                                        "pt", "id", "tr", "ru", "vi", "de", "it", "pl", "th",
                                    ] {
                                        let active = self
                                            .settings
                                            .preferred_source_language
                                            .eq_ignore_ascii_case(lang);
                                        if z_chip(ui, tokens, &lang.to_uppercase(), None, active) {
                                            self.settings.preferred_source_language =
                                                lang.to_string();
                                            self.catalog_lang = lang.to_string();
                                            let _ = self.settings_store.save(&self.settings);
                                        }
                                    }
                                });

                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new("Self-Hosted Komga & Kavita Servers")
                                        .size(12.5)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Komga URL:");
                                    if ui
                                        .text_edit_singleline(&mut self.settings.komga_server_url)
                                        .changed()
                                    {
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    ui.label("API Key:");
                                    if ui
                                        .text_edit_singleline(&mut self.settings.komga_api_key)
                                        .changed()
                                    {
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Kavita URL:");
                                    if ui
                                        .text_edit_singleline(&mut self.settings.kavita_server_url)
                                        .changed()
                                    {
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                    ui.label("API Key:");
                                    if ui
                                        .text_edit_singleline(&mut self.settings.kavita_api_key)
                                        .changed()
                                    {
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });

                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new("Custom Source Domain Override (Mirror Switcher)")
                                        .size(12.5)
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Source ID:");
                                    ui.text_edit_singleline(&mut self.custom_domain_key);
                                    ui.label("Mirror URL:");
                                    ui.text_edit_singleline(&mut self.custom_domain_val);
                                    if z_chip(ui, tokens, "Save Mirror", None, true)
                                        && !self.custom_domain_key.trim().is_empty()
                                    {
                                        self.settings.custom_source_domains.insert(
                                            self.custom_domain_key.trim().to_uppercase(),
                                            self.custom_domain_val.trim().to_string(),
                                        );
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });

                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(
                                        "Custom Source Cookie Override (Cloudflare cf_clearance)",
                                    )
                                    .size(12.5)
                                    .strong()
                                    .color(tokens.text_primary),
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Source ID:");
                                    ui.text_edit_singleline(&mut self.custom_cookie_key);
                                    ui.label("Cookie:");
                                    ui.text_edit_singleline(&mut self.custom_cookie_val);
                                    if z_chip(ui, tokens, "Save Cookie", None, true)
                                        && !self.custom_cookie_key.trim().is_empty()
                                    {
                                        self.settings.custom_source_cookies.insert(
                                            self.custom_cookie_key.trim().to_uppercase(),
                                            self.custom_cookie_val.trim().to_string(),
                                        );
                                        let _ = self.settings_store.save(&self.settings);
                                    }
                                });
                            }

                            SettingsSection::Downloads => {
                                ui.label(
                                    RichText::new("PARALLEL CBZ DOWNLOADER")
                                        .size(14.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.label(
                                    RichText::new(
                                        "Chapters are downloaded in parallel and packed into uncompressed stored .cbz archives with ComicInfo.xml.",
                                    )
                                    .size(12.0)
                                    .color(tokens.text_muted),
                                );
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label("Concurrent Page Download Threads:");
                                    for count in [1usize, 2, 3, 4, 6, 8] {
                                        let active = self.settings.max_concurrent_downloads == count;
                                        if z_chip(
                                            ui,
                                            tokens,
                                            &format!("{} Threads", count),
                                            None,
                                            active,
                                        ) {
                                            self.settings.max_concurrent_downloads = count;
                                            let _ = self.settings_store.save(&self.settings);
                                        }
                                    }
                                });
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(format!(
                                        "Downloads Folder: {}",
                                        self.downloader.downloads_dir().display()
                                    ))
                                    .size(12.5)
                                    .color(tokens.primary),
                                );
                            }

                            SettingsSection::Storage => {
                                ui.label(
                                    RichText::new("STORAGE & CACHE MANAGEMENT")
                                        .size(14.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.add_space(10.0);
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut self.settings.disk_cache_budget_mb,
                                            128..=4096,
                                        )
                                        .text("Disk Image Cache Budget (MB)"),
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    if z_chip(ui, tokens, "Clear Cover & Page Cache", Some("🧹"), true)
                                    {
                                        self.cover_cache.clear_caches();
                                        self.reader_pipeline.clear_caches();
                                        self.cover_textures.clear();
                                        self.reader_textures.clear();
                                        *self.status_banner.write() =
                                            "Cleared all cover and page caches.".to_string();
                                    }
                                    if z_chip(ui, tokens, "Clear Reading History", Some("🗑"), false)
                                    {
                                        self.library.clear_history();
                                        *self.status_banner.write() =
                                            "Cleared reading history.".to_string();
                                    }
                                });
                            }

                            SettingsSection::LibraryBackup => {
                                ui.label(
                                    RichText::new("LIBRARY, AUTO-REFRESH & BACKUP")
                                        .size(14.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.add_space(10.0);
                                if ui
                                    .checkbox(
                                        &mut self.settings.refresh_on_open,
                                        "Refresh Library on Startup (Automatically check favourites for new chapters on launch)",
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut self.settings.refresh_interval_hours,
                                            2..=48,
                                        )
                                        .text("Auto-Refresh Interval (Hours)"),
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                ui.add_space(12.0);
                                let backup_file = self.app_dir.join("zibra_backup.zip");
                                ui.horizontal(|ui| {
                                    if z_chip(ui, tokens, "Export Backup ZIP", Some("📤"), true) {
                                        let mgr = BackupManager::new(&self.app_dir);
                                        match mgr.export_backup(&backup_file) {
                                            Ok(p) => {
                                                self.backup_status_msg =
                                                    format!("✓ Backup exported to {}", p.display());
                                            }
                                            Err(e) => {
                                                self.backup_status_msg =
                                                    format!("⚠ Export failed: {}", e);
                                            }
                                        }
                                    }
                                    if z_chip(ui, tokens, "Restore Backup ZIP", Some("📥"), false) {
                                        let mgr = BackupManager::new(&self.app_dir);
                                        match mgr.import_backup(&backup_file) {
                                            Ok(n) => {
                                                self.backup_status_msg =
                                                    format!("✓ Restored {} records from backup.", n);
                                            }
                                            Err(e) => {
                                                self.backup_status_msg =
                                                    format!("⚠ Restore failed: {}", e);
                                            }
                                        }
                                    }
                                });
                                if !self.backup_status_msg.is_empty() {
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(&self.backup_status_msg)
                                            .size(12.5)
                                            .color(tokens.primary),
                                    );
                                }
                            }

                            SettingsSection::System => {
                                ui.label(
                                    RichText::new("SYSTEM, HARDWARE PIPELINE & NETWORK")
                                        .size(14.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.add_space(10.0);
                                if ui
                                    .checkbox(
                                        &mut self.settings.minimize_to_tray,
                                        "Minimize to System Tray when closed",
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                if ui
                                    .checkbox(
                                        &mut self.settings.dns_over_https,
                                        "Enable DNS-over-HTTPS (DoH)",
                                    )
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                ui.add_space(8.0);
                                ui.label("HTTP Browser User-Agent:");
                                if ui
                                    .text_edit_singleline(&mut self.settings.custom_user_agent)
                                    .changed()
                                {
                                    let _ = self.settings_store.save(&self.settings);
                                }
                                ui.add_space(12.0);
                                egui::Frame::none()
                                    .fill(tokens.surface_card)
                                    .stroke(Stroke::new(1.0_f32, tokens.border_glass))
                                    .rounding(Rounding::same(12.0))
                                    .inner_margin(Margin::same(12.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("Graphics & Hardware Acceleration")
                                                .size(12.5)
                                                .strong()
                                                .color(tokens.text_primary),
                                        );
                                        ui.label(
                                            RichText::new(
                                                "Native eframe/egui OpenGL (glow) + 2048px GPU Tile Slicer (Optimized for Windows 10/11 & Linux integrated GPUs)",
                                            )
                                            .size(11.5)
                                            .color(tokens.primary),
                                        );
                                    });
                            }

                            SettingsSection::About => {
                                ui.label(
                                    RichText::new("Zibrax v2.2.0 — Native Manga & Manhwa Reader")
                                        .size(18.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "High-performance 100% native Rust desktop Manga, Manhua & Manhwa reader engineered for Windows and Linux with 1,647+ integrated sources, 2048px GPU tile slicing, parallel CBZ archiver, and Kotatsu Material 3 Glass UI.",
                                    )
                                    .size(13.0)
                                    .color(tokens.text_primary),
                                );
                                ui.add_space(14.0);
                                ui.label(
                                    RichText::new("OFFICIAL LINKS")
                                        .size(12.0)
                                        .strong()
                                        .color(tokens.primary),
                                );
                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    if z_chip(ui, tokens, "🌐 Official Website", None, true) {
                                        ctx.open_url(egui::OpenUrl::new_tab(
                                            "https://zeravrix.pages.dev",
                                        ));
                                    }
                                    if z_chip(ui, tokens, "💻 GitHub Repository", None, false) {
                                        ctx.open_url(egui::OpenUrl::new_tab(
                                            "https://github.com/zeravrix/ZERAVRIX-PC",
                                        ));
                                    }
                                    if z_chip(ui, tokens, "💬 Discord Community", None, false) {
                                        ctx.open_url(egui::OpenUrl::new_tab(
                                            "https://discord.gg/zeravrix",
                                        ));
                                    }
                                });
                                ui.add_space(14.0);
                                ui.label(
                                    RichText::new("Author: Jhon Naji (U1M) • License: GNU General Public License v3.0")
                                        .size(12.0)
                                        .color(tokens.text_muted),
                                );
                            }
                        });
                });
        });
    }
}

fn sliced_to_textures_adjusted(
    ctx: &egui::Context,
    url: &str,
    sliced: &SlicedPage,
    contrast: f32,
    saturation: f32,
    brightness: f32,
    invert_colors: bool,
) -> Vec<TextureHandle> {
    let unmodified = (contrast - 1.0).abs() < 0.01
        && (saturation - 1.0).abs() < 0.01
        && (brightness - 1.0).abs() < 0.01
        && !invert_colors;

    sliced
        .tiles
        .iter()
        .map(|tile| {
            let color_img = if unmodified {
                ColorImage::from_rgba_unmultiplied(
                    [tile.width as usize, tile.height as usize],
                    &tile.rgba_bytes,
                )
            } else {
                let mut adj = Vec::with_capacity(tile.rgba_bytes.len());
                for px in tile.rgba_bytes.chunks_exact(4) {
                    let mut r = px[0] as f32;
                    let mut g = px[1] as f32;
                    let mut b = px[2] as f32;
                    let a = px[3];

                    if (saturation - 1.0).abs() >= 0.01 {
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        r = luma + (r - luma) * saturation;
                        g = luma + (g - luma) * saturation;
                        b = luma + (b - luma) * saturation;
                    }
                    if (contrast - 1.0).abs() >= 0.01 || (brightness - 1.0).abs() >= 0.01 {
                        let scale = contrast * brightness;
                        let offset = (1.0 - contrast) * 64.0;
                        r = r * scale + offset;
                        g = g * scale + offset;
                        b = b * scale + offset;
                    }
                    if invert_colors {
                        r = 255.0 - r;
                        g = 255.0 - g;
                        b = 255.0 - b;
                    }
                    adj.push(r.clamp(0.0, 255.0) as u8);
                    adj.push(g.clamp(0.0, 255.0) as u8);
                    adj.push(b.clamp(0.0, 255.0) as u8);
                    adj.push(a);
                }
                ColorImage::from_rgba_unmultiplied(
                    [tile.width as usize, tile.height as usize],
                    &adj,
                )
            };
            ctx.load_texture(
                format!("{}_tile_{}", url, tile.tile_index),
                color_img,
                HIGH_QUALITY_TEXTURE,
            )
        })
        .collect()
}

fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(EMBEDDED_LOGO_PNG).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1240.0, 780.0])
        .with_min_inner_size([500.0, 360.0])
        .with_resizable(true)
        .with_title("Zibrax • Native Manga & Manhwa Reader");
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Zibrax",
        options,
        Box::new(|cc| Ok(Box::new(ZibraApp::new(cc)))),
    )
}
