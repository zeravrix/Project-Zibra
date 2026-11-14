use crate::catalog_data::{EngineKind, CATALOG_SPECS};
use crate::engines::{
    azora::AzoraSource,
    dilar::DilarSource,
    generic_html::GenericHtmlEngine,
    hijala::HijalaSource,
    local::LocalFolderSource,
    madara::MadaraEngine,
    mangadex::MangaDexSource,
    mangaswat::MangaSwatSource,
    mangathemesia::MangaThemesiaEngine,
    servers::SelfHostedServerSource,
    teamx::TeamXSource,
    zeist::ZeistEngine,
};
use crate::{MangaPageResult, Source};
use crate::jvm_host::{JvmHostClient, JvmHostSource};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use zibra_core::{CatalogSortMode, LibraryStore, Manga, SourceTag};

pub struct LiveSourceRegistry {
    sources: Arc<RwLock<Vec<Arc<dyn Source>>>>,
    source_map: Arc<RwLock<HashMap<String, Arc<dyn Source>>>>,
    latency_ms: Arc<RwLock<HashMap<String, u64>>>,
    jvm_client: Arc<RwLock<Option<Arc<JvmHostClient>>>>,
}

impl Default for LiveSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveSourceRegistry {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .cookie_store(true)
            .danger_accept_invalid_certs(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        let mut sources: Vec<Arc<dyn Source>> = Vec::with_capacity(CATALOG_SPECS.len() + 32);

        // 1. Flagship Built-in Native Sources (Priority Ranks 1..7)
        sources.push(Arc::new(MangaDexSource::new(client.clone())));
        sources.push(Arc::new(MangaSwatSource::new(client.clone())));
        sources.push(Arc::new(AzoraSource::new(client.clone())));
        sources.push(Arc::new(TeamXSource::new(client.clone())));
        sources.push(Arc::new(MadaraEngine::new(
            "MANGALEK",
            "Mangalek",
            "https://mangalik.net",
            "ar",
            5,
            vec![
                SourceTag::Arabic,
                SourceTag::Manhwa,
                SourceTag::Webtoon,
                SourceTag::Popular,
                SourceTag::VerifiedWorking,
            ],
            client.clone(),
        )));
        sources.push(Arc::new(MadaraEngine::new(
            "MANGASPARK",
            "MangaSpark",
            "https://sparkmanga.net",
            "ar",
            6,
            vec![
                SourceTag::Arabic,
                SourceTag::Manhwa,
                SourceTag::Webtoon,
                SourceTag::VerifiedWorking,
            ],
            client.clone(),
        )));
        sources.push(Arc::new(HijalaSource::new(client.clone())));
        sources.push(Arc::new(DilarSource::new(client.clone())));

        // 2. Full Multi-Language Kotatsu & Mihon Catalog (1,636+ Sources across 17 Languages)
        for spec in CATALOG_SPECS {
            let tags_vec = spec.tags.to_vec();
            let engine_arc: Arc<dyn Source> = match spec.engine {
                EngineKind::Madara => Arc::new(MadaraEngine::new(
                    spec.id,
                    spec.name,
                    spec.base_url,
                    spec.lang,
                    spec.rank,
                    tags_vec,
                    client.clone(),
                )),
                EngineKind::MangaThemesia => Arc::new(MangaThemesiaEngine::new(
                    spec.id,
                    spec.name,
                    spec.base_url,
                    spec.lang,
                    spec.rank,
                    tags_vec,
                    client.clone(),
                )),
                EngineKind::Zeist => Arc::new(ZeistEngine::new(
                    spec.id,
                    spec.name,
                    spec.base_url,
                    spec.lang,
                    spec.rank,
                    tags_vec,
                    client.clone(),
                )),
                EngineKind::GenericHtml => Arc::new(GenericHtmlEngine::new(
                    spec.id,
                    spec.name,
                    spec.base_url,
                    spec.lang,
                    spec.rank,
                    tags_vec,
                    client.clone(),
                )),
            };
            sources.push(engine_arc);
        }

        // 3. Local CBZ / Folder Library & Self-Hosted Servers (Komga & Kavita)
        sources.push(Arc::new(LocalFolderSource::new(PathBuf::from("./downloads"))));
        sources.push(Arc::new(SelfHostedServerSource::new_komga("http://localhost:25600")));
        sources.push(Arc::new(SelfHostedServerSource::new_kavita("http://localhost:5000")));

        // 4. Seamlessly import any user-configured repository web sources from ~/.zeravrix or ~/.zibra
        Self::load_user_repository_sources(&mut sources, &client);

        let mut source_map = HashMap::with_capacity(sources.len() * 2);
        for s in &sources {
            source_map.insert(s.id().to_string(), s.clone());
            source_map.insert(s.id().to_uppercase(), s.clone());
        }

        let reg = Self {
            sources: Arc::new(RwLock::new(sources)),
            source_map: Arc::new(RwLock::new(source_map)),
            latency_ms: Arc::new(RwLock::new(HashMap::new())),
            jvm_client: Arc::new(RwLock::new(None)),
        };

        reg.spawn_jvm_binding_if_available();
        reg
    }

    fn spawn_jvm_binding_if_available(&self) {
        if tokio::runtime::Handle::try_current().is_ok() {
            let sources_lock = self.sources.clone();
            let map_lock = self.source_map.clone();
            let jvm_lock = self.jvm_client.clone();

            tokio::spawn(async move {
                match JvmHostClient::try_start().await {
                    Ok(Some(client)) => {
                        *jvm_lock.write() = Some(client.clone());
                        if let Ok(list) = client.list_sources().await {
                            println!("[JVM-HOST] Bound {} Kotatsu parser sources into ZIBRA LiveSourceRegistry!", list.len());
                            let mut s_w = sources_lock.write();
                            let mut m_w = map_lock.write();
                            for info in list {
                                let jvm_source = Arc::new(JvmHostSource::new(client.clone(), &info));
                                m_w.insert(jvm_source.id().to_string(), jvm_source.clone());
                                m_w.insert(jvm_source.id().to_uppercase(), jvm_source.clone());
                                m_w.insert(info.id.to_string(), jvm_source.clone());
                                let name_key = info.name.to_uppercase().replace(' ', "");
                                m_w.entry(name_key).or_insert_with(|| jvm_source.clone());
                                s_w.push(jvm_source);
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("[JVM-HOST] Initialization warning: {}", e),
                }
            });
        }
    }

    pub async fn bind_jvm_host(&self) -> anyhow::Result<usize> {
        let Some(client) = JvmHostClient::try_start().await? else {
            return Ok(0);
        };
        *self.jvm_client.write() = Some(client.clone());
        let list = client.list_sources().await?;
        let count = list.len();
        {
            let mut s_w = self.sources.write();
            let mut m_w = self.source_map.write();
            for info in list {
                let jvm_source = Arc::new(JvmHostSource::new(client.clone(), &info));
                m_w.insert(jvm_source.id().to_string(), jvm_source.clone());
                m_w.insert(jvm_source.id().to_uppercase(), jvm_source.clone());
                m_w.insert(info.id.to_string(), jvm_source.clone());
                let name_key = info.name.to_uppercase().replace(' ', "");
                m_w.entry(name_key).or_insert_with(|| jvm_source.clone());
                s_w.push(jvm_source);
            }
        }
        Ok(count)
    }

    fn load_user_repository_sources(sources: &mut Vec<Arc<dyn Source>>, client: &reqwest::Client) {
        let Some(home) = dirs::home_dir() else { return };
        let mut seen: HashSet<String> = sources.iter().map(|s| s.id().to_string()).collect();

        for dir in [home.join(".zibra"), home.join(".zeravrix")] {
            let props_file = dir.join("repository-web-sources.properties");
            if let Ok(content) = fs::read_to_string(&props_file) {
                let mut map = HashMap::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once('=') {
                        map.insert(
                            k.trim().to_string(),
                            v.trim().replace("\\:", ":").replace("\\=", "="),
                        );
                    }
                }
                let count = map
                    .get("count")
                    .and_then(|c| c.parse::<usize>().ok())
                    .unwrap_or(0);
                for idx in 0..count {
                    let prefix = format!("source.{}.", idx);
                    let Some(id) = map.get(&format!("{}id", prefix)) else { continue };
                    let name = map
                        .get(&format!("{}name", prefix))
                        .cloned()
                        .unwrap_or_else(|| format!("Repo Source {}", idx + 1));
                    let lang = map
                        .get(&format!("{}language", prefix))
                        .cloned()
                        .unwrap_or_else(|| "multi".to_string());
                    let Some(home_url) = map.get(&format!("{}homeUrl", prefix)) else { continue };
                    if !home_url.starts_with("http") {
                        continue;
                    }
                    let sid = format!("REPO_{}", id);
                    if seen.insert(sid.clone()) {
                        sources.push(Arc::new(GenericHtmlEngine::new(
                            sid,
                            name,
                            home_url.clone(),
                            lang,
                            2000 + (idx as u32),
                            vec![SourceTag::Manhwa, SourceTag::Manga],
                            client.clone(),
                        )));
                    }
                }
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Source>> {
        let map = self.source_map.read();
        if let Some(found) = map.get(id) {
            return Some(found.clone());
        }
        let mapped = LibraryStore::map_legacy_source_id(id);
        map.get(&mapped)
            .or_else(|| map.get(&mapped.to_uppercase()))
            .cloned()
    }

    pub fn all_sources(&self) -> Vec<Arc<dyn Source>> {
        self.sources.read().clone()
    }

    pub fn total_count(&self) -> usize {
        self.sources.read().len()
    }

    pub fn visible_count(&self, nsfw_enabled: bool) -> usize {
        let s = self.sources.read();
        if nsfw_enabled {
            s.len()
        } else {
            s.iter().filter(|s| !s.is_nsfw()).count()
        }
    }

    pub fn latency_for(&self, id: &str) -> Option<u64> {
        self.latency_ms.read().get(id).copied()
    }

    pub fn record_latency(&self, id: &str, ms: u64) {
        self.latency_ms.write().insert(id.to_string(), ms);
    }

    pub fn ordered_catalog(
        &self,
        sort_mode: CatalogSortMode,
        pinned_ids: &HashSet<String>,
        required_tags: &HashSet<SourceTag>,
        lang_filter: &str,
        search_query: &str,
        nsfw_enabled: bool,
    ) -> Vec<Arc<dyn Source>> {
        let q = search_query.trim().to_lowercase();
        let sources_snap = self.sources.read().clone();
        let mut filtered: Vec<Arc<dyn Source>> = sources_snap
            .into_iter()
            .filter(|s| {
                if !nsfw_enabled && s.is_nsfw() {
                    return false;
                }
                let match_lang =
                    lang_filter == "ALL" || s.language().eq_ignore_ascii_case(lang_filter);
                let match_query = q.is_empty()
                    || s.name().to_lowercase().contains(&q)
                    || s.id().to_lowercase().contains(&q)
                    || s.base_url().to_lowercase().contains(&q)
                    || s.language().to_lowercase().contains(&q)
                    || s.tags().iter().any(|t| t.label().to_lowercase().contains(&q));
                let match_tags = required_tags.is_empty()
                    || required_tags.iter().all(|req| {
                        if *req == SourceTag::Nsfw {
                            s.is_nsfw()
                        } else {
                            s.tags().contains(req)
                        }
                    });
                match_lang && match_query && match_tags
            })
            .collect();

        let latencies = self.latency_ms.read().clone();
        filtered.sort_by(|a, b| match sort_mode {
            CatalogSortMode::RecommendedOrder => a.priority_rank().cmp(&b.priority_rank()),
            CatalogSortMode::PinnedFirst => {
                let a_pin = pinned_ids.contains(a.id());
                let b_pin = pinned_ids.contains(b.id());
                b_pin
                    .cmp(&a_pin)
                    .then_with(|| a.priority_rank().cmp(&b.priority_rank()))
            }
            CatalogSortMode::FastestLatency => {
                let a_lat = latencies.get(a.id()).copied().unwrap_or(9999);
                let b_lat = latencies.get(b.id()).copied().unwrap_or(9999);
                a_lat
                    .cmp(&b_lat)
                    .then_with(|| a.priority_rank().cmp(&b.priority_rank()))
            }
            CatalogSortMode::Alphabetical => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            CatalogSortMode::LanguageGroup => a
                .language()
                .cmp(b.language())
                .then_with(|| a.priority_rank().cmp(&b.priority_rank())),
        });

        filtered
    }

    pub async fn fetch_from_source(
        &self,
        source_id: &str,
        page: u32,
        latest: bool,
        query: Option<&str>,
        nsfw_enabled: bool,
    ) -> anyhow::Result<MangaPageResult> {
        let src = self
            .get(source_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown source {}", source_id))?;
        if !nsfw_enabled && src.is_nsfw() {
            anyhow::bail!("18+ NSFW source '{}' is hidden while Disable 18+ / NSFW is active.", src.name());
        }
        let start = Instant::now();
        let mut res = if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
            src.search(page, q).await
        } else if latest {
            src.latest(page).await
        } else {
            src.popular(page).await
        }?;
        self.record_latency(src.id(), start.elapsed().as_millis() as u64);
        if !nsfw_enabled {
            res.items.retain(|m| !m.is_nsfw());
        }
        Ok(res)
    }

    /// Global parallel search across top pinned/fast sources (matching ZERAVRIX LiveSourceRegistry.search)
    pub async fn search_global(
        &self,
        query: &str,
        pinned_ids: &HashSet<String>,
        nsfw_enabled: bool,
    ) -> Vec<Manga> {
        let q = query.trim().to_string();
        if q.is_empty() {
            return Vec::new();
        }
        let sources_snap = self.sources.read().clone();
        let targets: Vec<Arc<dyn Source>> = sources_snap
            .into_iter()
            .filter(|s| (nsfw_enabled || !s.is_nsfw()) && (pinned_ids.contains(s.id()) || s.priority_rank() <= 12))
            .take(16)
            .collect();

        let mut handles = Vec::new();
        for src in targets {
            let q_clone = q.clone();
            handles.push(tokio::spawn(async move {
                match tokio::time::timeout(Duration::from_secs(7), src.search(1, &q_clone)).await {
                    Ok(Ok(res)) => res.items,
                    _ => Vec::new(),
                }
            }));
        }

        let mut combined = Vec::new();
        let mut seen = HashSet::new();
        for h in handles {
            if let Ok(items) = h.await {
                for m in items {
                    if !nsfw_enabled && m.is_nsfw() {
                        continue;
                    }
                    if seen.insert(m.id.clone()) {
                        combined.push(m);
                    }
                }
            }
        }
        combined
    }
}

