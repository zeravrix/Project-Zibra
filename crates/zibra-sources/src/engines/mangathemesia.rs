use crate::extractor::{extract_best_image_url, extract_chapter_pages, parse_chapter_number, resolve_url};
use crate::{MangaPageResult, Source};
use async_trait::async_trait;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use zibra_core::{Chapter, Manga, MangaPage, SourceTag};

pub struct MangaThemesiaEngine {
    id: String,
    name: String,
    base_url: String,
    lang: String,
    rank: u32,
    tags: Vec<SourceTag>,
    client: reqwest::Client,
}

impl MangaThemesiaEngine {
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
            .header("Referer", format!("{}/", self.base_url))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("{} returned HTTP {}", self.name, resp.status());
        }
        Ok(resp.text().await?)
    }

    async fn fetch_list(&self, url: &str) -> anyhow::Result<MangaPageResult> {
        let html = self.fetch_html(url).await?;
        let doc = Html::parse_document(&html);
        let card_sel = Selector::parse(".bs .bsx, .listupd .bsx, .utao .uta, article.bs").unwrap();
        let link_sel = Selector::parse("a[href]").unwrap();
        let title_sel = Selector::parse(".tt, .entry-title, h3, h4").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for card in doc.select(&card_sel) {
            let Some(a) = card.select(&link_sel).next() else { continue };
            let Some(href) = a.value().attr("href") else { continue };
            let Some(full_url) = resolve_url(url, href) else { continue };
            if !seen.insert(full_url.clone()) {
                continue;
            }
            let title = card
                .select(&title_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| a.value().attr("title").map(|s| s.trim().to_string()))
                .unwrap_or_else(|| "Manhwa".to_string());
            let cover = card.select(&img_sel).next().and_then(|i| extract_best_image_url(i, url));
            let mut m = Manga::new(full_url, title, &self.id);
            m.cover_url = cover;
            m.is_manhwa = true;
            m.enrich_manhwa_tags();
            items.push(m);
        }

        Ok(MangaPageResult {
            has_next_page: items.len() >= 10,
            items,
        })
    }
}

#[async_trait]
impl Source for MangaThemesiaEngine {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn language(&self) -> &str { &self.lang }
    fn base_url(&self) -> &str { &self.base_url }
    fn priority_rank(&self) -> u32 { self.rank }
    fn tags(&self) -> &[SourceTag] { &self.tags }

    async fn popular(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        let url = format!("{}/manga/?page={}&order=popular", self.base_url, page.max(1));
        self.fetch_list(&url).await
    }

    async fn latest(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        let url = format!("{}/manga/?page={}&order=update", self.base_url, page.max(1));
        self.fetch_list(&url).await
    }

    async fn search(&self, page: u32, query: &str) -> anyhow::Result<MangaPageResult> {
        let url = format!("{}/page/{}/?s={}", self.base_url, page.max(1), urlencoding::encode(query.trim()));
        self.fetch_list(&url).await
    }

    async fn details(&self, manga: &Manga) -> anyhow::Result<Manga> {
        let html = self.fetch_html(&manga.id).await?;
        let doc = Html::parse_document(&html);
        let title_sel = Selector::parse("h1.entry-title, .infox h1, h1").unwrap();
        let desc_sel = Selector::parse(".entry-content[itemprop=description], .desc, .summary__content").unwrap();
        let img_sel = Selector::parse(".thumb img, .infomanga img").unwrap();

        let mut updated = manga.clone();
        if let Some(t) = doc.select(&title_sel).next() {
            let txt = t.text().collect::<String>().trim().to_string();
            if !txt.is_empty() {
                updated.title = txt;
            }
        }
        if let Some(d) = doc.select(&desc_sel).next() {
            updated.description = d.text().collect::<String>().trim().to_string();
        }
        if let Some(img) = doc.select(&img_sel).next() {
            if let Some(c) = extract_best_image_url(img, &manga.id) {
                updated.cover_url = Some(c);
            }
        }
        updated.enrich_manhwa_tags();
        Ok(updated)
    }

    async fn chapters(&self, manga: &Manga) -> anyhow::Result<Vec<Chapter>> {
        let html = self.fetch_html(&manga.id).await?;
        let doc = Html::parse_document(&html);
        let ch_sel = Selector::parse("#chapterlist li a, .eplister ul li a, .bxcl ul li a").unwrap();
        let num_sel = Selector::parse(".chapternum").unwrap();

        let mut chapters = Vec::new();
        let mut seen = HashSet::new();
        for a in doc.select(&ch_sel) {
            if let Some(href) = a.value().attr("href") {
                if let Some(full_url) = resolve_url(&manga.id, href) {
                    if !seen.insert(full_url.clone()) {
                        continue;
                    }
                    let title = a
                        .select(&num_sel)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| a.text().collect::<String>().trim().to_string());
                    let num = parse_chapter_number(&title);
                    chapters.push(Chapter::new(full_url, manga.id.clone(), title, num, &self.id));
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
            "#readerarea img, .rdminimal img, .reading-content img",
            &headers,
        ))
    }
}
