use crate::{MangaPageResult, Source};
use async_trait::async_trait;
use zibra_core::{Chapter, Manga, MangaPage, SourceTag};

pub struct SelfHostedServerSource {
    id: String,
    name: String,
    server_url: String,
    rank: u32,
}

impl SelfHostedServerSource {
    pub fn new_komga(server_url: impl Into<String>) -> Self {
        Self {
            id: "KOMGA".to_string(),
            name: "Komga Server".to_string(),
            server_url: server_url.into(),
            rank: 50,
        }
    }

    pub fn new_kavita(server_url: impl Into<String>) -> Self {
        Self {
            id: "KAVITA".to_string(),
            name: "Kavita Server".to_string(),
            server_url: server_url.into(),
            rank: 51,
        }
    }
}

#[async_trait]
impl Source for SelfHostedServerSource {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn language(&self) -> &str { "multi" }
    fn base_url(&self) -> &str { &self.server_url }
    fn priority_rank(&self) -> u32 { self.rank }
    fn tags(&self) -> &[SourceTag] {
        &[SourceTag::SelfHosted, SourceTag::OfficialApi, SourceTag::Manhwa, SourceTag::Manga]
    }

    async fn popular(&self, _page: u32) -> anyhow::Result<MangaPageResult> {
        Ok(MangaPageResult { items: Vec::new(), has_next_page: false })
    }
    async fn latest(&self, _page: u32) -> anyhow::Result<MangaPageResult> {
        Ok(MangaPageResult { items: Vec::new(), has_next_page: false })
    }
    async fn search(&self, _page: u32, _query: &str) -> anyhow::Result<MangaPageResult> {
        Ok(MangaPageResult { items: Vec::new(), has_next_page: false })
    }
    async fn details(&self, manga: &Manga) -> anyhow::Result<Manga> {
        Ok(manga.clone())
    }
    async fn chapters(&self, _manga: &Manga) -> anyhow::Result<Vec<Chapter>> {
        Ok(Vec::new())
    }
    async fn pages(&self, _chapter: &Chapter) -> anyhow::Result<Vec<MangaPage>> {
        Ok(Vec::new())
    }
}
