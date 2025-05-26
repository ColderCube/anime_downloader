use super::errors::AnimeDownloaderError;
use async_trait::async_trait;

#[allow(dead_code)]
#[async_trait]
pub trait Downloader: Send + Sync {
    fn name(&self) -> &'static str;
    // async fn add_download(
    //     &self,
    //     url: &str,
    //     options: HashMap<String, String>,
    // ) -> Result<(), AnimeDownloaderError>;
    async fn add_url_link(&mut self, url: String) -> Result<(), AnimeDownloaderError>;
    async fn download(&mut self) -> Result<(), AnimeDownloaderError>;
}
