use crate::extractor::{extract_best_image_url, extract_chapter_pages, parse_chapter_number, resolve_url};
use crate::{MangaPageResult, Source};
use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use std::collections::{HashMap, HashSet};
use zibra_core::{Chapter, Manga, MangaPage, SourceTag};

pub struct MadaraEngine {
    id: String,
    name: String,
    base_url: String,
    lang: String,
    rank: u32,
    tags: Vec<SourceTag>,
    client: reqwest::Client,
}

impl MadaraEngine {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        lang: impl Into<String>,
        rank: u32,
        tags: Vec<SourceTag>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            lang: lang.into(),
            rank,
            tags,
            client,
        }
    }

    async fn fetch_html(&self, url: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .get(url)
            .header("Accept-Language", "en-US,en;q=0.9,ar;q=0.8")
            .header("Referer", format!("{}/", self.base_url))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("{} returned HTTP {} at {}", self.name, resp.status(), url);
        }
        Ok(resp.text().await?)
    }

    fn parse_card(&self, card: ElementRef<'_>, page_url: &str) -> Option<Manga> {
        let link_sel = Selector::parse("a[href*='/manga/'], a[href*='/series/'], a[href*='/comics/'], .post-title a, h3 a, h4 a, a").ok()?;
        let anchor = card.select(&link_sel).next()?;
        let href = anchor.value().attr("href")?;
        let url = resolve_url(page_url, href)?;
        if url == self.base_url || url.ends_with("/manga/") || url.ends_with("/series/") {
            return None;
        }

        let title_sel = Selector::parse(".post-title, .entry-title, .data h3, h3, h4").ok()?;
        let title = card
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| anchor.value().attr("title").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| anchor.text().collect::<String>().trim().to_string());

        if title.is_empty() {
            return None;
        }

        let img_sel = Selector::parse("img").ok()?;
        let cover_url = card.select(&img_sel).next().and_then(|img| extract_best_image_url(img, page_url));

        let mut manga = Manga::new(url, title, &self.id);
        manga.cover_url = cover_url;
        manga.enrich_manhwa_tags();
        Some(manga)
    }

    async fn fetch_archive(&self, page: u32, order: &str, query: Option<&str>) -> anyhow::Result<MangaPageResult> {
        let p = page.max(1);
        let url = if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
            format!("{}/page/{}/?s={}&post_type=wp-manga", self.base_url, p, urlencoding::encode(q.trim()))
        } else {
            format!("{}/manga/page/{}/?m_orderby={}", self.base_url, p, order)
        };

        let html = match self.fetch_html(&url).await {
            Ok(h) => h,
            Err(_) if query.is_none() => {
                let fallback = format!("{}/page/{}/?s=&post_type=wp-manga&m_orderby={}", self.base_url, p, order);
                self.fetch_html(&fallback).await?
            }
            Err(e) => return Err(e),
        };

        let doc = Html::parse_document(&html);
        let card_sel = Selector::parse("div.page-item-detail, .row.c-tabs-item__content, .manga__item, .manga-item, .item-summary, .bsx, .listupd .bs, .box").unwrap();
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for card in doc.select(&card_sel) {
            if let Some(m) = self.parse_card(card, &url) {
                if seen.insert(m.id.clone()) {
                    items.push(m);
                }
            }
        }

        let has_next = items.len() >= 12;
        Ok(MangaPageResult {
            items,
            has_next_page: has_next,
        })
    }

    fn parse_chapters_from_html(&self, html: &str, manga_id: &str) -> Vec<Chapter> {
        let doc = Html::parse_document(html);
        let ch_sel = Selector::parse("li.wp-manga-chapter a, .chapter-item a, .listing-chapters_wrap a, #chapter-list a[href]").unwrap();
        let mut list = Vec::new();
        let mut seen = HashSet::new();

        for link in doc.select(&ch_sel) {
            if let Some(href) = link.value().attr("href") {
                if let Some(full_url) = resolve_url(manga_id, href) {
                    let clean_url = full_url.split("?style=paged").next().unwrap_or(&full_url).to_string();
                    if clean_url == manga_id.trim_end_matches('/') || !seen.insert(clean_url.clone()) {
                        continue;
                    }
                    let title = link.text().collect::<String>().trim().to_string();
                    let label = if title.is_empty() { "Chapter".to_string() } else { title };
                    let num = parse_chapter_number(&label);
                    list.push(Chapter::new(clean_url, manga_id.to_string(), label, num, &self.id));
                }
            }
        }
        list
    }
}

#[async_trait]
impl Source for MadaraEngine {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn language(&self) -> &str { &self.lang }
    fn base_url(&self) -> &str { &self.base_url }
    fn priority_rank(&self) -> u32 { self.rank }
    fn tags(&self) -> &[SourceTag] { &self.tags }

    async fn popular(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        self.fetch_archive(page, "views", None).await
    }

    async fn latest(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        self.fetch_archive(page, "latest", None).await
    }

    async fn search(&self, page: u32, query: &str) -> anyhow::Result<MangaPageResult> {
        self.fetch_archive(page, "latest", Some(query)).await
    }

    async fn details(&self, manga: &Manga) -> anyhow::Result<Manga> {
        let html = self.fetch_html(&manga.id).await?;
        let doc = Html::parse_document(&html);
        let title_sel = Selector::parse(".post-title h1, .summary_content h1, h1.entry-title, h1").unwrap();
        let img_sel = Selector::parse(".summary_image img, .profile-manga img, img").unwrap();
        let desc_sel = Selector::parse(".description-summary, .summary__content, .summary_content, .description").unwrap();
        let genre_sel = Selector::parse(".genres-content a, .genres a, .mgen a").unwrap();

        let title = doc
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| manga.title.clone());

        let cover = doc
            .select(&img_sel)
            .next()
            .and_then(|img| extract_best_image_url(img, &manga.id))
            .or_else(|| manga.cover_url.clone());

        let description = doc
            .select(&desc_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| manga.description.clone());

        let genres: Vec<String> = doc
            .select(&genre_sel)
            .map(|e| e.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut updated = manga.clone();
        updated.title = title;
        updated.cover_url = cover;
        updated.description = description;
        if !genres.is_empty() {
            updated.genres = genres;
        }
        updated.enrich_manhwa_tags();
        Ok(updated)
    }

    async fn chapters(&self, manga: &Manga) -> anyhow::Result<Vec<Chapter>> {
        let html = self.fetch_html(&manga.id).await?;
        let mut chapters = self.parse_chapters_from_html(&html, &manga.id);

        if chapters.is_empty() {
            let ajax_url = format!("{}/ajax/chapters/", manga.id.trim_end_matches('/'));
            if let Ok(resp) = self
                .client
                .post(&ajax_url)
                .header("Referer", &manga.id)
                .header("X-Requested-With", "XMLHttpRequest")
                .send()
                .await
            {
                if let Ok(xhr_html) = resp.text().await {
                    chapters = self.parse_chapters_from_html(&xhr_html, &manga.id);
                }
            }
        }

        Ok(chapters)
    }

    async fn pages(&self, chapter: &Chapter) -> anyhow::Result<Vec<MangaPage>> {
        let html = self.fetch_html(&chapter.id).await?;
        let mut headers = HashMap::new();
        headers.insert("Referer".to_string(), chapter.id.clone());
        Ok(extract_chapter_pages(
            &html,
            &chapter.id,
            "div.wp-manga-chapter-img img, .reading-content img, .page-break img, .chapter_image img, img.wp-manga-chapter-img",
            &headers,
        ))
    }
}
