use crate::models::{Chapter, Manga, ReadingProgress};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct LibraryStore {
    root: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl LibraryStore {
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(root)?;
        let db_path = root.join("zibra_library.sqlite3");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS library_manga (
                 id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 cover_url TEXT,
                 description TEXT NOT NULL DEFAULT '',
                 source_id TEXT NOT NULL,
                 author TEXT,
                 artist TEXT,
                 genres_json TEXT NOT NULL DEFAULT '[]',
                 status TEXT,
                 cached_chapter_count INTEGER,
                 url TEXT,
                 is_manhwa INTEGER NOT NULL DEFAULT 1,
                 category TEXT NOT NULL DEFAULT 'Favorites',
                 added_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS history_manga (
                 id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 cover_url TEXT,
                 description TEXT NOT NULL DEFAULT '',
                 source_id TEXT NOT NULL,
                 author TEXT,
                 artist TEXT,
                 genres_json TEXT NOT NULL DEFAULT '[]',
                 status TEXT,
                 cached_chapter_count INTEGER,
                 url TEXT,
                 is_manhwa INTEGER NOT NULL DEFAULT 1,
                 last_accessed INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS reading_progress (
                 manga_id TEXT PRIMARY KEY,
                 chapter_id TEXT NOT NULL,
                 page_index INTEGER NOT NULL DEFAULT 0,
                 scroll_offset_y REAL NOT NULL DEFAULT 0.0,
                 chapter_number REAL,
                 updated_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS cached_chapters (
                 manga_id TEXT NOT NULL,
                 chapter_id TEXT NOT NULL,
                 title TEXT NOT NULL,
                 number REAL,
                 source_id TEXT NOT NULL,
                 volume INTEGER,
                 scanlator TEXT,
                 upload_date INTEGER,
                 url TEXT,
                 is_locked INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (manga_id, chapter_id)
             );",
        )?;

        let store = Self {
            root: root.to_path_buf(),
            conn: Arc::new(Mutex::new(conn)),
        };

        let _ = store.import_legacy_properties(root);
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list_favorites(&self) -> Vec<Manga> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT id, title, cover_url, description, source_id, author, artist, genres_json, status, cached_chapter_count, url, is_manhwa
             FROM library_manga ORDER BY title COLLATE NOCASE ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map([], Self::row_to_manga)
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
    }

    pub fn list_history(&self) -> Vec<Manga> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT id, title, cover_url, description, source_id, author, artist, genres_json, status, cached_chapter_count, url, is_manhwa
             FROM history_manga ORDER BY last_accessed DESC LIMIT 250",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map([], Self::row_to_manga)
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
    }

    pub fn is_favorite(&self, manga_id: &str) -> bool {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT 1 FROM library_manga WHERE id = ?1",
            params![manga_id],
            |_| Ok(true),
        )
        .optional()
        .unwrap_or(None)
        .unwrap_or(false)
    }

    pub fn save_favorite(&self, manga: &Manga) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let genres_json = serde_json::to_string(&manga.genres).unwrap_or_else(|_| "[]".to_string());
        let now = Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO library_manga (id, title, cover_url, description, source_id, author, artist, genres_json, status, cached_chapter_count, url, is_manhwa, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                 title = excluded.title,
                 cover_url = COALESCE(excluded.cover_url, library_manga.cover_url),
                 description = CASE WHEN excluded.description != '' THEN excluded.description ELSE library_manga.description END,
                 author = COALESCE(excluded.author, library_manga.author),
                 artist = COALESCE(excluded.artist, library_manga.artist),
                 genres_json = excluded.genres_json,
                 status = COALESCE(excluded.status, library_manga.status),
                 cached_chapter_count = COALESCE(excluded.cached_chapter_count, library_manga.cached_chapter_count),
                 url = COALESCE(excluded.url, library_manga.url),
                 is_manhwa = excluded.is_manhwa",
            params![
                manga.id,
                manga.title,
                manga.cover_url,
                manga.description,
                manga.source_id,
                manga.author,
                manga.artist,
                genres_json,
                manga.status,
                manga.cached_chapter_count,
                manga.url,
                if manga.is_manhwa { 1 } else { 0 },
                now,
            ],
        )?;
        Ok(())
    }

    pub fn remove_favorite(&self, manga_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM library_manga WHERE id = ?1", params![manga_id])?;
        Ok(())
    }

    pub fn toggle_favorite(&self, manga: &Manga) -> bool {
        if self.is_favorite(&manga.id) {
            let _ = self.remove_favorite(&manga.id);
            false
        } else {
            let _ = self.save_favorite(manga);
            true
        }
    }

    pub fn record_history(&self, manga: &Manga) {
        let conn = self.conn.lock();
        let genres_json = serde_json::to_string(&manga.genres).unwrap_or_else(|_| "[]".to_string());
        let now = Utc::now().timestamp_millis();
        let _ = conn.execute(
            "INSERT INTO history_manga (id, title, cover_url, description, source_id, author, artist, genres_json, status, cached_chapter_count, url, is_manhwa, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                 title = excluded.title,
                 cover_url = COALESCE(excluded.cover_url, history_manga.cover_url),
                 last_accessed = excluded.last_accessed",
            params![
                manga.id,
                manga.title,
                manga.cover_url,
                manga.description,
                manga.source_id,
                manga.author,
                manga.artist,
                genres_json,
                manga.status,
                manga.cached_chapter_count,
                manga.url,
                if manga.is_manhwa { 1 } else { 0 },
                now,
            ],
        );
    }

    pub fn clear_history(&self) {
        let conn = self.conn.lock();
        let _ = conn.execute("DELETE FROM history_manga", []);
    }

    pub fn save_progress(&self, progress: &ReadingProgress) {
        let conn = self.conn.lock();
        let _ = conn.execute(
            "INSERT INTO reading_progress (manga_id, chapter_id, page_index, scroll_offset_y, chapter_number, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(manga_id) DO UPDATE SET
                 chapter_id = excluded.chapter_id,
                 page_index = excluded.page_index,
                 scroll_offset_y = excluded.scroll_offset_y,
                 chapter_number = excluded.chapter_number,
                 updated_at = excluded.updated_at",
            params![
                progress.manga_id,
                progress.chapter_id,
                progress.page_index as i64,
                progress.scroll_offset_y,
                progress.chapter_number,
                progress.updated_at,
            ],
        );
    }

    pub fn latest_progress(&self, manga_id: &str) -> Option<ReadingProgress> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT manga_id, chapter_id, page_index, scroll_offset_y, chapter_number, updated_at
             FROM reading_progress WHERE manga_id = ?1",
            params![manga_id],
            |row| {
                Ok(ReadingProgress {
                    manga_id: row.get(0)?,
                    chapter_id: row.get(1)?,
                    page_index: row.get::<_, i64>(2)?.max(0) as usize,
                    scroll_offset_y: row.get(3)?,
                    chapter_number: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
        .unwrap_or(None)
    }

    pub fn cache_chapters(&self, manga_id: &str, chapters: &[Chapter]) {
        let mut conn = self.conn.lock();
        if let Ok(tx) = conn.transaction() {
            for ch in chapters {
                let _ = tx.execute(
                    "INSERT INTO cached_chapters (manga_id, chapter_id, title, number, source_id, volume, scanlator, upload_date, url, is_locked)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                     ON CONFLICT(manga_id, chapter_id) DO UPDATE SET
                         title = excluded.title,
                         number = excluded.number,
                         upload_date = COALESCE(excluded.upload_date, cached_chapters.upload_date)",
                    params![
                        manga_id,
                        ch.id,
                        ch.title,
                        ch.number,
                        ch.source_id,
                        ch.volume,
                        ch.scanlator,
                        ch.upload_date,
                        ch.url,
                        if ch.is_locked { 1 } else { 0 },
                    ],
                );
            }
            let _ = tx.commit();
        };
    }

    pub fn get_cached_chapters(&self, manga_id: &str) -> Vec<Chapter> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare(
            "SELECT chapter_id, manga_id, title, number, source_id, volume, scanlator, upload_date, url, is_locked
             FROM cached_chapters WHERE manga_id = ?1 ORDER BY COALESCE(number, -1.0) DESC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map(params![manga_id], |row| {
            Ok(Chapter {
                id: row.get(0)?,
                manga_id: row.get(1)?,
                title: row.get(2)?,
                number: row.get(3)?,
                source_id: row.get(4)?,
                volume: row.get(5)?,
                scanlator: row.get(6)?,
                upload_date: row.get(7)?,
                url: row.get(8)?,
                is_locked: row.get::<_, i32>(9)? != 0,
            })
        })
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
    }

    fn row_to_manga(row: &rusqlite::Row<'_>) -> rusqlite::Result<Manga> {
        let genres_json: String = row.get(7)?;
        let genres: Vec<String> = serde_json::from_str(&genres_json).unwrap_or_default();
        Ok(Manga {
            id: row.get(0)?,
            title: row.get(1)?,
            cover_url: row.get::<_, Option<String>>(2)?.filter(|s| !s.is_empty()),
            description: row.get(3)?,
            source_id: row.get(4)?,
            author: row.get::<_, Option<String>>(5)?.filter(|s| !s.is_empty()),
            artist: row.get::<_, Option<String>>(6)?.filter(|s| !s.is_empty()),
            genres,
            status: row.get::<_, Option<String>>(8)?.filter(|s| !s.is_empty()),
            cached_chapter_count: row.get(9)?,
            url: row.get::<_, Option<String>>(10)?.filter(|s| !s.is_empty()),
            is_manhwa: row.get::<_, i32>(11)? != 0,
        })
    }

    pub fn import_legacy_properties(&self, dir: &Path) -> usize {
        let mut imported = 0;
        for sub in ["library", "history"] {
            let sub_dir = dir.join(sub);
            if sub_dir.is_dir() {
                if let Ok(entries) = fs::read_dir(&sub_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().and_then(|e| e.to_str()) == Some("properties") {
                            if let Some(manga) = Self::parse_legacy_manga_props(&entry.path()) {
                                if sub == "library" {
                                    let _ = self.save_favorite(&manga);
                                } else {
                                    self.record_history(&manga);
                                }
                                imported += 1;
                            }
                        }
                    }
                }
            }
        }
        imported
    }

    fn parse_legacy_manga_props(path: &Path) -> Option<Manga> {
        let content = fs::read_to_string(path).ok()?;
        let mut map = HashMap::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        let id = map.get("id")?.clone();
        let title = map.get("title")?.clone();
        let genres: Vec<String> = map
            .get("genres")
            .map(|g| g.split('|').filter(|s| !s.is_empty()).map(String::from).collect())
            .unwrap_or_default();
        let is_manhwa = Manga::detect_manhwa(&title, &id, &genres);
        Some(Manga {
            id: id.clone(),
            title,
            cover_url: map.get("coverUrl").cloned().filter(|s| !s.is_empty()),
            description: map.get("description").cloned().unwrap_or_default(),
            source_id: map.get("sourceId").cloned().unwrap_or_else(|| "MANGADEX".to_string()),
            author: map.get("author").cloned().filter(|s| !s.is_empty()),
            artist: map.get("artist").cloned().filter(|s| !s.is_empty()),
            genres,
            status: map.get("status").cloned().filter(|s| !s.is_empty()),
            cached_chapter_count: map.get("cachedChapterCount").and_then(|c| c.parse().ok()),
            url: map.get("url").cloned().or(Some(id)),
            is_manhwa,
        })
    }
}
