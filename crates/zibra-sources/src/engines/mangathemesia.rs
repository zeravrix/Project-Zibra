use crate::extractor::{
    extract_best_image_url, extract_chapter_pages, fetch_html_resilient, parse_chapter_number,
    resolve_url,
};
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
        fetch_html_resilient(&self.client, url, Some(&self.base_url)).await
    }

    async fn fetch_candidates(&self, candidates: &[String]) -> anyhow::Result<MangaPageResult> {
        let mut html = String::new();
        let mut active_url = self.base_url.clone();
        let mut last_err = None;

        for u in candidates {
            match self.fetch_html(u).await {
                Ok(h) if h.len() > 200 => {
                    html = h;
                    active_url = u.clone();
                    break;
                }
                Ok(_) => {}
                Err(e) => last_err = Some(e),
            }
        }

        if html.is_empty() {
            return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("{} returned empty page", self.name)));
        }

        let doc = Html::parse_document(&html);
        let card_sel = Selector::parse(
            ".bs .bsx, .listupd .bsx, .utao .uta, article.bs, .animposx, .box, .page-item-detail, .manga-item, .col-6, article",
        ).unwrap();
        let link_sel = Selector::parse("a[href]").unwrap();
        let title_sel = Selector::parse(".tt, .entry-title, h3, h4, h5").unwrap();
        let img_sel = Selector::parse("img").unwrap();

        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for card in doc.select(&card_sel) {
            let is_anchor = card.value().name() == "a";
            let anchor_opt = if is_anchor && card.value().attr("href").is_some() {
                Some(card)
            } else {
                card.select(&link_sel).next()
            };
            let Some(a) = anchor_opt else { continue };
            let Some(href) = a.value().attr("href") else { continue };
            let Some(full_url) = resolve_url(&active_url, href) else { continue };
            let clean = full_url.trim_end_matches('/');
            if clean == self.base_url || !seen.insert(full_url.clone()) {
                continue;
            }
            let title = card
                .select(&title_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| a.value().attr("title").map(|s| s.trim().to_string()))
                .unwrap_or_else(|| "Manhwa".to_string());
            let cover = card.select(&img_sel).next().and_then(|i| extract_best_image_url(i, &active_url));
            let mut m = Manga::new(full_url, title, &self.id);
            m.cover_url = cover;
            m.is_manhwa = true;
            m.enrich_manhwa_tags();
            items.push(m);
        }

        if items.is_empty() {
            let direct_sel = Selector::parse("a[href*='/manga/'], a[href*='/series/'], a[href*='/komik/']").unwrap();
            for a in doc.select(&direct_sel) {
                let Some(href) = a.value().attr("href") else { continue };
                let Some(full_url) = resolve_url(&active_url, href) else { continue };
                if full_url.trim_end_matches('/') == self.base_url || !seen.insert(full_url.clone()) {
                    continue;
                }
                let title = a.value().attr("title")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| a.text().collect::<String>().trim().to_string());
                if title.is_empty() { continue; }
                let mut m = Manga::new(full_url, title, &self.id);
                m.is_manhwa = true;
                items.push(m);
            }
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
        let p = page.max(1);
        let candidates = if p <= 1 {
            vec![
                format!("{}/manga/?order=popular", self.base_url),
                format!("{}/manga/?page=1&order=popular", self.base_url),
                format!("{}/series/?order=popular", self.base_url),
                format!("{}/komik/?order=popular", self.base_url),
                format!("{}/manga/", self.base_url),
                format!("{}/series/", self.base_url),
                format!("{}/", self.base_url),
            ]
        } else {
            vec![
                format!("{}/manga/?page={}&order=popular", self.base_url, p),
                format!("{}/manga/page/{}/?order=popular", self.base_url, p),
                format!("{}/series/?page={}&order=popular", self.base_url, p),
                format!("{}/page/{}/", self.base_url, p),
            ]
        };
        self.fetch_candidates(&candidates).await
    }

    async fn latest(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        let p = page.max(1);
        let candidates = if p <= 1 {
            vec![
                format!("{}/manga/?order=update", self.base_url),
                format!("{}/manga/?page=1&order=update", self.base_url),
                format!("{}/series/?order=update", self.base_url),
                format!("{}/manga/", self.base_url),
                format!("{}/", self.base_url),
            ]
        } else {
            vec![
                format!("{}/manga/?page={}&order=update", self.base_url, p),
                format!("{}/series/?page={}&order=update", self.base_url, p),
                format!("{}/page/{}/", self.base_url, p),
            ]
        };
        self.fetch_candidates(&candidates).await
    }

    async fn search(&self, page: u32, query: &str) -> anyhow::Result<MangaPageResult> {
        let p = page.max(1);
        let enc = urlencoding::encode(query.trim());
        let candidates = vec![
            format!("{}/page/{}/?s={}", self.base_url, p, enc),
            format!("{}/?s={}", self.base_url, enc),
            format!("{}/manga/?s={}", self.base_url, enc),
        ];
        self.fetch_candidates(&candidates).await
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
