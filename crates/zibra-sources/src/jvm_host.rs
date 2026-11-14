use crate::{MangaPageResult, Source};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::oneshot;
use zibra_core::{Chapter, Manga, MangaPage, SourceTag};

#[derive(Debug, Clone, Deserialize)]
pub struct HostSourceInfo {
    pub id: i64,
    pub name: String,
    pub language: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(default)]
    pub popular: bool,
    #[serde(default)]
    pub latest: bool,
    #[serde(default)]
    pub search: bool,
    #[serde(default)]
    pub enabled: bool,
}

pub struct JvmHostClient {
    stdin: Arc<tokio::sync::Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    counter: AtomicU64,
}

impl JvmHostClient {
    pub fn find_executable() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("ZERAVRIX_JVM_HOST").or_else(|_| std::env::var("ZIBRA_JVM_HOST")) {
            let p = PathBuf::from(path);
            if p.is_file() {
                return Some(p);
            }
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let candidate = parent.join("jvm-host").join("bin").join("jvm-host");
                if candidate.is_file() {
                    return Some(candidate);
                }
                let candidate2 = parent.join("bin").join("jvm-host");
                if candidate2.is_file() {
                    return Some(candidate2);
                }
            }
        }

        let release_candidate = PathBuf::from("/home/u1m/Documents/ZERAVRIX/release/zibra-linux-x86_64/jvm-host/bin/jvm-host");
        if release_candidate.is_file() {
            return Some(release_candidate);
        }

        let dev_candidate = PathBuf::from("/home/u1m/Documents/ZERAVRIX/pc_repo/jvm-host/build/install/jvm-host/bin/jvm-host");
        if dev_candidate.is_file() {
            return Some(dev_candidate);
        }

        if let Some(data) = dirs::data_local_dir() {
            let p = data.join("zeravrix").join("jvm-host").join("bin").join("jvm-host");
            if p.is_file() {
                return Some(p);
            }
        }

        None
    }

    pub async fn try_start() -> anyhow::Result<Option<Arc<Self>>> {
        let Some(bin_path) = Self::find_executable() else {
            return Ok(None);
        };

        let mut child = match Command::new(&bin_path)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[JVM-HOST] Warning: failed to spawn {}: {}", bin_path.display(), e);
                return Ok(None);
            }
        };

        let stdin = match child.stdin.take() {
            Some(s) => s,
            None => return Ok(None),
        };
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return Ok(None),
        };

        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                    if let Some(req_id) = val.get("requestId").and_then(|r| r.as_str()) {
                        let tx = pending_clone.lock().remove(req_id);
                        if let Some(tx) = tx {
                            let _ = tx.send(val);
                        }
                    }
                }
            }
        });

        let client = Arc::new(Self {
            stdin: Arc::new(tokio::sync::Mutex::new(stdin)),
            pending,
            counter: AtomicU64::new(1),
        });

        // Handshake: hello
        match client.request("hello", json!({})).await {
            Ok(resp) if resp.get("ok").and_then(|v| v.as_bool()) == Some(true) => {
                println!("[JVM-HOST] Handshake successful with {}", bin_path.display());
                Ok(Some(client))
            }
            Ok(resp) => {
                eprintln!("[JVM-HOST] Handshake returned error: {:?}", resp);
                Ok(None)
            }
            Err(e) => {
                eprintln!("[JVM-HOST] Handshake timed out or failed: {}", e);
                Ok(None)
            }
        }
    }

    pub async fn request(&self, op: &str, mut payload: Value) -> anyhow::Result<Value> {
        let req_id = format!("req-{}", self.counter.fetch_add(1, Ordering::SeqCst));
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(req_id.clone(), tx);

        if let Some(obj) = payload.as_object_mut() {
            obj.insert("protocol".to_string(), json!(1));
            obj.insert("requestId".to_string(), json!(req_id));
            obj.insert("op".to_string(), json!(op));
        }

        let mut data = serde_json::to_vec(&payload)?;
        data.push(b'\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(&data).await?;
            stdin.flush().await?;
        }

        match tokio::time::timeout(Duration::from_secs(35), rx).await {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(_)) => anyhow::bail!("JVM host connection closed unexpectedly"),
            Err(_) => {
                self.pending.lock().remove(&req_id);
                anyhow::bail!("JVM host request timed out: op={}", op)
            }
        }
    }

    pub async fn list_sources(&self) -> anyhow::Result<Vec<HostSourceInfo>> {
        let resp = self.request("list_sources", json!({})).await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("list_sources failed: {:?}", resp.get("error"));
        }
        let sources_val = resp
            .get("result")
            .and_then(|r| r.get("sources"))
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        let list: Vec<HostSourceInfo> = serde_json::from_value(sources_val)?;
        Ok(list)
    }
}

pub struct JvmHostSource {
    client: Arc<JvmHostClient>,
    pub raw_id: i64,
    pub id_str: String,
    pub name_str: String,
    pub lang_str: String,
    pub base_url_str: String,
    pub tags_vec: Vec<SourceTag>,
    pub rank: u32,
}

impl JvmHostSource {
    pub fn new(client: Arc<JvmHostClient>, info: &HostSourceInfo) -> Self {
        Self::with_rank(client, info, 10)
    }

    pub fn with_rank(client: Arc<JvmHostClient>, info: &HostSourceInfo, rank: u32) -> Self {
        let lang = if info.language.trim().is_empty() {
            "multi".to_string()
        } else {
            info.language.to_lowercase()
        };
        let mut tags = Vec::new();
        match lang.as_str() {
            "ar" => tags.push(SourceTag::Arabic),
            "en" => tags.push(SourceTag::English),
            "fr" => tags.push(SourceTag::French),
            "es" => tags.push(SourceTag::Spanish),
            "ko" => tags.push(SourceTag::Korean),
            "ja" => tags.push(SourceTag::Japanese),
            _ => tags.push(SourceTag::MultiLang),
        }
        tags.push(SourceTag::VerifiedWorking);

        let base_url_str = if info.base_url.starts_with("http://") || info.base_url.starts_with("https://") {
            info.base_url.clone()
        } else {
            format!("https://{}.com", info.base_url.replace('_', "-"))
        };

        let id_str = format!("KOTATSU_{}", info.id);
        Self {
            client,
            raw_id: info.id,
            id_str,
            name_str: info.name.clone(),
            lang_str: lang,
            base_url_str,
            tags_vec: tags,
            rank,
        }
    }

    fn parse_manga_list(&self, val: &Value) -> MangaPageResult {
        let mut items = Vec::new();
        let has_next_page = val
            .get("result")
            .and_then(|r| r.get("hasNextPage"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(arr) = val.get("result").and_then(|r| r.get("items")).and_then(|i| i.as_array()) {
            for it in arr {
                let id = it.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() || title.is_empty() {
                    continue;
                }
                let cover_url = it.get("coverUrl").and_then(|v| v.as_str()).map(String::from);
                let description = it.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let author = it.get("author").and_then(|v| v.as_str()).map(String::from);
                let artist = it.get("artist").and_then(|v| v.as_str()).map(String::from);
                let status = it.get("status").and_then(|v| v.as_str()).map(String::from);
                let url = it.get("url").and_then(|v| v.as_str()).map(String::from).or_else(|| Some(id.clone()));

                let mut m = Manga::new(&id, &title, &self.id_str);
                m.cover_url = cover_url;
                m.description = description;
                m.author = author;
                m.artist = artist;
                m.status = status;
                m.url = url;
                m.enrich_manhwa_tags();
                items.push(m);
            }
        }

        MangaPageResult {
            items,
            has_next_page,
        }
    }
}

#[async_trait]
impl Source for JvmHostSource {
    fn id(&self) -> &str {
        &self.id_str
    }
    fn name(&self) -> &str {
        &self.name_str
    }
    fn language(&self) -> &str {
        &self.lang_str
    }
    fn base_url(&self) -> &str {
        &self.base_url_str
    }
    fn priority_rank(&self) -> u32 {
        self.rank
    }
    fn tags(&self) -> &[SourceTag] {
        &self.tags_vec
    }

    async fn popular(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        let resp = self
            .client
            .request(
                "popular",
                json!({
                    "sourceId": self.raw_id,
                    "page": page,
                }),
            )
            .await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("popular failed: {:?}", resp.get("error"));
        }
        Ok(self.parse_manga_list(&resp))
    }

    async fn latest(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        let resp = self
            .client
            .request(
                "latest",
                json!({
                    "sourceId": self.raw_id,
                    "page": page,
                }),
            )
            .await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("latest failed: {:?}", resp.get("error"));
        }
        Ok(self.parse_manga_list(&resp))
    }

    async fn search(&self, page: u32, query: &str) -> anyhow::Result<MangaPageResult> {
        let resp = self
            .client
            .request(
                "search",
                json!({
                    "sourceId": self.raw_id,
                    "page": page,
                    "query": query,
                }),
            )
            .await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("search failed: {:?}", resp.get("error"));
        }
        Ok(self.parse_manga_list(&resp))
    }

    async fn details(&self, manga: &Manga) -> anyhow::Result<Manga> {
        let resp = self
            .client
            .request(
                "details",
                json!({
                    "sourceId": self.raw_id,
                    "manga": {
                        "id": manga.id,
                        "title": manga.title,
                        "coverUrl": manga.cover_url,
                        "description": manga.description,
                        "sourceId": self.raw_id,
                    }
                }),
            )
            .await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Ok(manga.clone());
        }

        if let Some(m_val) = resp.get("result").and_then(|r| r.get("manga")) {
            let mut updated = manga.clone();
            if let Some(title) = m_val.get("title").and_then(|v| v.as_str()) {
                if !title.is_empty() {
                    updated.title = title.to_string();
                }
            }
            if let Some(cover) = m_val.get("coverUrl").and_then(|v| v.as_str()) {
                updated.cover_url = Some(cover.to_string());
            }
            if let Some(desc) = m_val.get("description").and_then(|v| v.as_str()) {
                if !desc.is_empty() {
                    updated.description = desc.to_string();
                }
            }
            if let Some(author) = m_val.get("author").and_then(|v| v.as_str()) {
                updated.author = Some(author.to_string());
            }
            if let Some(genres) = m_val.get("genres").and_then(|v| v.as_array()) {
                let g_vec: Vec<String> = genres.iter().filter_map(|g| g.as_str().map(String::from)).collect();
                if !g_vec.is_empty() {
                    updated.genres = g_vec;
                }
            }
            updated.enrich_manhwa_tags();
            return Ok(updated);
        }
        Ok(manga.clone())
    }

    async fn chapters(&self, manga: &Manga) -> anyhow::Result<Vec<Chapter>> {
        let resp = self
            .client
            .request(
                "chapters",
                json!({
                    "sourceId": self.raw_id,
                    "manga": {
                        "id": manga.id,
                        "title": manga.title,
                        "coverUrl": manga.cover_url,
                        "description": manga.description,
                        "sourceId": self.raw_id,
                    }
                }),
            )
            .await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("chapters failed: {:?}", resp.get("error"));
        }

        let mut chapters = Vec::new();
        if let Some(arr) = resp.get("result").and_then(|r| r.get("chapters")).and_then(|c| c.as_array()) {
            for ch in arr {
                let id = ch.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title = ch.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() {
                    continue;
                }
                let number = ch.get("number").and_then(|v| v.as_f64()).map(|n| n as f32);
                let volume = ch.get("volume").and_then(|v| v.as_i64()).map(|v| v as i32);
                let scanlator = ch.get("scanlator").and_then(|v| v.as_str()).map(String::from);
                let upload_date = ch.get("uploadDate").and_then(|v| v.as_i64());

                let mut chapter = Chapter::new(&id, &manga.id, &title, number, &self.id_str);
                chapter.volume = volume;
                chapter.scanlator = scanlator;
                chapter.upload_date = upload_date;
                chapters.push(chapter);
            }
        }
        Ok(chapters)
    }

    async fn pages(&self, chapter: &Chapter) -> anyhow::Result<Vec<MangaPage>> {
        let resp = self
            .client
            .request(
                "pages",
                json!({
                    "sourceId": self.raw_id,
                    "chapter": {
                        "id": chapter.id,
                        "mangaId": chapter.manga_id,
                        "title": chapter.title,
                        "number": chapter.number,
                        "sourceId": self.raw_id,
                        "volume": chapter.volume,
                        "scanlator": chapter.scanlator,
                        "uploadDate": chapter.upload_date,
                    }
                }),
            )
            .await?;
        if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("pages failed: {:?}", resp.get("error"));
        }

        let mut pages = Vec::new();
        if let Some(arr) = resp.get("result").and_then(|r| r.get("pages")).and_then(|p| p.as_array()) {
            for pg in arr {
                let idx = pg.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let img_url = pg.get("imageUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if img_url.is_empty() {
                    continue;
                }
                let referer = pg.get("referer").and_then(|v| v.as_str()).map(String::from);
                let mut page = MangaPage::new(idx, img_url, referer);
                if let Some(headers) = pg.get("headers").and_then(|v| v.as_object()) {
                    for (k, v) in headers {
                        if let Some(s) = v.as_str() {
                            page.headers.insert(k.clone(), s.to_string());
                        }
                    }
                }
                pages.push(page);
            }
        }
        Ok(pages)
    }
}
