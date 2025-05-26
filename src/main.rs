use std::num::ParseIntError;
use std::io::Write;
use std::env;

mod common;
mod ddos;
mod download;
mod sites;

use common::anime::AnimeSite;
use common::download::Downloader;
use common::errors::AnimeDownloaderError;
use common::utils;
use download::aria2c::Aria2cDownoader;
use sites::animepahe::AnimePahe;

#[tokio::main]
async fn main() -> Result<(), AnimeDownloaderError> {
    let mut path = env::current_dir()?;
    clearscreen::clear().map_err(|e| AnimeDownloaderError::Other(e.to_string()))?;
    print!("Enter the anime name : ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    let mut anime_pahe = AnimePahe::new()?;
    // let animes = anime_pahe.search_sync("tokyo ghoul".to_string())?;
    let animes = anime_pahe.search(input.trim().to_string()).await?;
    for anime in animes.iter().enumerate() {
        println!("{}: {}", anime.0 + 1, anime.1.title());
    }
    let mut input = String::new();
    print!("Enter anime number : ");
    std::io::stdout().flush()?;
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    let num: u32 = input
        .trim()
        .parse()
        .map_err(|e: ParseIntError| AnimeDownloaderError::ParsingError(e.to_string()))?;
    let mut animes = animes[num as usize - 1].clone_object();
    path.push(utils::filenamify(animes.title()));
    if !path.exists() {
        std::fs::create_dir(path.clone())?;
    }
    // let episodes = animes.fetch_episode_list_sync()?;
    let episodes = animes.fetch_episode_list().await?;
    println!("It has {} episodes", animes.episode_count().unwrap_or(0));
    let mut downloader = Aria2cDownoader::new().await?;
    let dpath = path.display().to_string();
    downloader.download_path = Some(dpath);
    for episode in episodes.iter().enumerate() {
        episode.1.download(&mut downloader).await?;
    }
    downloader.download().await?;

    Ok(())
}
