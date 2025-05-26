use std::fmt::{Debug, Display};

use async_trait::async_trait;

use super::download;
use super::errors::AnimeDownloaderError;
use super::quality::Quality;

#[allow(dead_code)]
#[async_trait]
pub trait AnimeSite: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn base_url(&self) -> &str;
    fn available_qualities(&self) -> &'static [Quality];
    async fn search(
        &mut self,
        query: String,
    ) -> Result<Vec<Box<dyn AnimeSeries>>, AnimeDownloaderError>;
    fn search_sync(
        &mut self,
        query: String,
    ) -> Result<Vec<Box<dyn AnimeSeries>>, AnimeDownloaderError>;
    fn clone_object(&self) -> Box<dyn AnimeSite>;
}

#[allow(dead_code)]
#[async_trait]
pub trait AnimeSeries: Send + Sync + Debug {
    fn site_name(&self) -> &str;
    fn title(&self) -> &str;
    async fn fetch_episode_list(
        &mut self,
    ) -> Result<Vec<Box<dyn AnimeEpisode>>, AnimeDownloaderError>;
    fn fetch_episode_list_sync(
        &mut self,
    ) -> Result<Vec<Box<dyn AnimeEpisode>>, AnimeDownloaderError>;
    fn get_episode_list(&self) -> Option<Vec<Box<dyn AnimeEpisode>>>;
    // fn episodes_iter(&self) -> impl Iterator<Item = &impl AnimeEpisode>;
    fn clone_object(&self) -> Box<dyn AnimeSeries>;
    fn episode_count(&self) -> Option<usize> {
        self.get_episode_list().map(|list| list.len())
    }
}

#[allow(dead_code)]
#[async_trait]
pub trait AnimeEpisode: Send + Sync + Debug + Display {
    fn series_title(&self) -> &str;
    fn episode_number_str(&self) -> &str;
    fn pretty_title(&self) -> String {
        format!(
            "{} - Episode {}",
            self.series_title(),
            self.episode_number_str()
        )
    }
    async fn download<'a>(
        &self,
        downloader: &'a mut dyn download::Downloader,
    ) -> Result<(), AnimeDownloaderError>;
    fn clone_object(&self) -> Box<dyn AnimeEpisode>;
}
