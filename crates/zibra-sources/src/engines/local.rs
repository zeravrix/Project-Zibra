use crate::{MangaPageResult, Source};
use async_trait::async_trait;
use std::path::PathBuf;
use zibra_core::{Chapter, Manga, MangaPage, SourceTag};

pub struct LocalFolderSource {
    root_dir: PathBuf,
}

impl LocalFolderSource {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }
}

#[async_trait]
impl Source for LocalFolderSource {
    fn id(&self) -> &str { "LOCAL_FILES" }
    fn name(&self) -> &str { "Local CBZ / Folders" }
    fn language(&self) -> &str { "multi" }
    fn base_url(&self) -> &str { "https://localhost" }
    fn priority_rank(&self) -> u32 { 49 }
    fn tags(&self) -> &[SourceTag] {
        &[SourceTag::LocalStorage, SourceTag::Manhwa, SourceTag::Manga, SourceTag::FastCdn]
    }

    async fn popular(&self, _page: u32) -> anyhow::Result<MangaPageResult> {
        let _ = &self.root_dir;
        Ok(MangaPageResult { items: Vec::new(), has_next_page: false })
    }
    async fn latest(&self, page: u32) -> anyhow::Result<MangaPageResult> {
        self.popular(page).await
    }
    async fn search(&self, page: u32, _query: &str) -> anyhow::Result<MangaPageResult> {
        self.popular(page).await
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
