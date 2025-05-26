use async_trait::async_trait;
use regex::Regex;
use reqwest;
use reqwest::header::HeaderValue;
use reqwest_cookie_store::CookieStoreMutex;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde_json::{self, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::common::anime;
use crate::common::anime::AnimeEpisode;
use crate::common::download::Downloader;
use crate::common::errors::AnimeDownloaderError;
use crate::common::quality::Quality;
use crate::ddos::ddos_guard_net;

#[derive(Debug, Clone)]
pub struct AnimePahe {
    client: ClientWithMiddleware,
    base_url: String,
    cookie_store: Arc<CookieStoreMutex>,
    headers: reqwest::header::HeaderMap,
}

impl AnimePahe {
    pub fn new() -> Result<Self, AnimeDownloaderError> {
        let cookie_store = {
            let file = match std::fs::File::open("E:\\Projects\\anime_downloader\\cookies.json")
                .map(std::io::BufReader::new)
            {
                Ok(reader) => reader,
                Err(_) => {
                    return Err(AnimeDownloaderError::ConfigError(
                        "Could not open cookies.json".to_string(),
                    ))
                }
            };
            cookie_store::serde::json::load(file).unwrap_or(cookie_store::CookieStore::new(None))
        };
        let cookie_store = reqwest_cookie_store::CookieStoreMutex::new(cookie_store);
        let cookie_store = std::sync::Arc::new(cookie_store);

        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(Duration::from_secs(1), Duration::from_secs(60))
            .build_with_max_retries(20);

        let client: ClientWithMiddleware = ClientBuilder::new(
            reqwest::ClientBuilder::new()
                // .danger_accept_invalid_certs(true)
                // .danger_accept_invalid_hostnames(true)
                .gzip(true)
                .cookie_store(true)
                .cookie_provider(std::sync::Arc::clone(&cookie_store))
                .redirect(reqwest::redirect::Policy::none())
                .use_rustls_tls()
                .build()
                .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?,
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("en-US,en;q=0.9"),
        );
        headers.insert("cache-control", HeaderValue::from_static("max-age=0"));
        headers.insert("priority", HeaderValue::from_static("u=0, i"));
        headers.insert("referer", HeaderValue::from_static("https://animepahe.ru/"));
        headers.insert(
            "sec-ch-ua",
            HeaderValue::from_static(
                "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"",
            ),
        );
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert(
            "sec-ch-ua-platform",
            HeaderValue::from_static("\"Windows\""),
        );
        headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
        headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64)) AppleWebKit/537.36 (KHTML, like Gecko)) Chrome/136.0.0.0 Safari/537.36"));

        Ok(AnimePahe {
            base_url: "https://animepahe.ru".to_string(),
            client: client.to_owned(),
            cookie_store,
            headers: headers.to_owned(),
        })
    }
}

#[async_trait]
impl anime::AnimeSite for AnimePahe {
    fn clone_object(&self) -> Box<dyn anime::AnimeSite> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "AnimePahe"
    }
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn available_qualities(&self) -> &'static [Quality] {
        &[Quality::P360, Quality::P480, Quality::P720, Quality::P1080]
    }
    async fn search(
        &mut self,
        query: String,
    ) -> Result<Vec<Box<dyn anime::AnimeSeries>>, AnimeDownloaderError> {
        println!("Start Searching for {}", query);
        let mut response = self
            .client
            .get(format!("{}/api", self.base_url))
            .query(&[("m", "search"), ("q", query.as_str())])
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            println!("Bypassing DDoS protection...");
            ddos_guard_net::bypass(&self.client, &self.cookie_store, self.base_url.as_str())
                .await?;

            // Try again after bypass
            response = self
                .client
                .get(format!("{}/api", self.base_url))
                .query(&[("m", "search"), ("q", query.as_str())])
                .headers(self.headers.clone())
                .send()
                .await
                .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        }
        println!("Got anime list");

        let animes: Value = serde_json::from_str(
            &response
                .text()
                .await
                .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?,
        )
        .map_err(|e| AnimeDownloaderError::ParsingError(e.to_string()))?;

        let anime_list = match animes["data"].as_array() {
            Some(list) => list,
            None => return Ok(vec![]), // No results found
        };

        let mut results = Vec::new();

        for anime in anime_list {
            let title = anime["title"].as_str().unwrap_or("Unknown").to_string();
            let session_id = anime["session"].as_str().unwrap_or("").to_string();

            let mut metadata = HashMap::new();
            if let Some(status) = anime["status"].as_str() {
                metadata.insert("status".to_string(), status.to_string());
            }
            if let Some(year) = anime["year"].as_str() {
                metadata.insert("year".to_string(), year.to_string());
            }
            if let Some(season) = anime["season"].as_str() {
                metadata.insert("season".to_string(), season.to_string());
            }

            results.push(Box::new(AnimePaheSeries {
                client: Arc::new(self.client.clone()),
                base_url: self.base_url.clone(),
                cookie_store: Arc::clone(&self.cookie_store),
                title,
                session_id,
                site_name: self.name().to_string(),
                episode_list: Vec::new(),
            }) as Box<dyn anime::AnimeSeries>);
        }
        let mut writer = match std::fs::File::create("cookies.json").map(std::io::BufWriter::new) {
            Ok(writer) => writer,
            Err(e) => return Err(AnimeDownloaderError::IoError(e)),
        };
        let store = self.cookie_store.lock().unwrap();
        cookie_store::serde::json::save(&store, &mut writer).unwrap();

        Ok(results)
    }

    fn search_sync(
        &mut self,
        query: String,
    ) -> Result<Vec<Box<dyn anime::AnimeSeries>>, AnimeDownloaderError> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| AnimeDownloaderError::Other(e.to_string()))?;
        runtime.block_on(async { self.search(query).await })
    }
}

#[derive(Debug, Clone)]
pub struct AnimePaheSeries {
    client: Arc<ClientWithMiddleware>,
    base_url: String,
    cookie_store: Arc<CookieStoreMutex>,
    title: String,
    session_id: String,
    site_name: String,
    episode_list: Vec<AnimePaheEpisode>,
}

#[async_trait]
impl anime::AnimeSeries for AnimePaheSeries {
    fn site_name(&self) -> &str {
        self.site_name.as_str()
    }

    fn clone_object(&self) -> Box<dyn anime::AnimeSeries> {
        Box::new(self.clone())
    }

    fn title(&self) -> &str {
        &self.title
    }

    async fn fetch_episode_list(
        &mut self,
    ) -> Result<Vec<Box<dyn anime::AnimeEpisode>>, AnimeDownloaderError> {
        let mut page: usize = 1;
        loop {
            let response = self
                .client
                .get("https://animepahe.ru/api")
                .query(&[
                    ("m", "release"),
                    ("id", self.session_id.as_str()),
                    ("sort", "episode_asc"),
                    ("page", page.to_string().as_str()),
                ])
                .send()
                .await
                .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
            let ep_list: serde_json::Value = serde_json::from_str(
                response
                    .text()
                    .await
                    .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?
                    .as_str(),
            )
            .map_err(|e| AnimeDownloaderError::ParsingError(e.to_string()))?;
            let episode_array = match ep_list["data"].as_array() {
                Some(list) => list,
                None => break,
            };
            if episode_array.is_empty() {
                break;
            }
            for ep in episode_array {
                let episode_number: String = if ep["episode2"].as_u64().unwrap() > 0 {
                    format!(
                        "{} - {}",
                        ep["episode"].as_u64().unwrap(),
                        ep["episode2"].as_u64().unwrap()
                    )
                } else {
                    ep["episode"].as_u64().unwrap().to_string()
                };
                self.episode_list.push(AnimePaheEpisode {
                    client: Arc::clone(&self.client),
                    base_url: self.base_url.clone(),
                    title: ep["title"].as_str().unwrap_or("Unknown").to_string(),
                    session_id: ep["session"].as_str().unwrap_or("").to_string(),
                    anime_id: self.session_id.clone(),
                    anime_title: self.title.clone(),
                    episode_number,
                });
            }
            let last_page = ep_list["last_page"].as_u64().unwrap_or(1) as usize;
            if page >= last_page {
                break;
            }

            page += 1;
        }

        let mut writer = match std::fs::File::create("cookies.json").map(std::io::BufWriter::new) {
            Ok(writer) => writer,
            Err(e) => return Err(AnimeDownloaderError::IoError(e)),
        };
        let store = self.cookie_store.lock().unwrap();
        cookie_store::serde::json::save(&store, &mut writer).unwrap();
        return Ok(self
            .episode_list
            .iter()
            .map(|x| x.clone_object())
            .collect::<Vec<_>>());
    }

    fn get_episode_list(&self) -> Option<Vec<Box<dyn anime::AnimeEpisode>>> {
        Some(
            self.episode_list
                .iter()
                .map(|x| x.clone_object())
                .collect::<Vec<_>>(),
        )
    }

    // fn episodes_iter(&self) -> impl Iterator<Item = &impl anime::AnimeEpisode> {
    //     &self.episode_list.iter()
    // }

    fn fetch_episode_list_sync(
        &mut self,
    ) -> Result<Vec<Box<dyn AnimeEpisode>>, AnimeDownloaderError> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| AnimeDownloaderError::Other(e.to_string()))?;
        runtime.block_on(async { self.fetch_episode_list().await })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AnimePaheEpisode {
    client: Arc<ClientWithMiddleware>,
    base_url: String,
    title: String,
    session_id: String,
    anime_id: String,
    anime_title: String,
    episode_number: String,
}

#[async_trait]
impl anime::AnimeEpisode for AnimePaheEpisode {
    fn series_title(&self) -> &str {
        self.title.as_str()
    }

    fn clone_object(&self) -> Box<dyn anime::AnimeEpisode> {
        Box::new(self.clone())
    }

    fn episode_number_str(&self) -> &str {
        self.episode_number.as_str()
    }

    async fn download<'a>(
        &self,
        downloader: &'a mut dyn Downloader,
    ) -> Result<(), AnimeDownloaderError> {
        let selected_quality = self.get_pahe().await?;
        let kwik_link = self.get_kwik(selected_quality.0).await?;

        // TODO: Implement the kwik link extraction as extarctor
        let (download_url, token) = self.bypass_1(&kwik_link).await?;
        let download_url = self.bypass_2(&download_url, &token, kwik_link.as_str()).await?;
        downloader.add_url_link(download_url.clone()).await?;

        Ok(())
    }
}

impl AnimePaheEpisode {
    async fn get_pahe(&self) -> Result<(String, Quality), AnimeDownloaderError> {
        let response = self
            .client
            .get(format!(
                "https://animepahe.ru/play/{}/{}",
                self.anime_id, self.session_id
            ))
            .send()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let text = response
            .text()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let document = scraper::Html::parse_document(text.as_str());
        let selector = scraper::Selector::parse("div#pickDownload>a").unwrap();
        let re = Regex::new(r"\b(\d{3,4}p)\b").unwrap();
        let mut qualities = document
            .select(&selector)
            .filter_map(|x| {
                let href = x.attr("href")?;

                let quality_text = x
                    .children()
                    .filter_map(|child| match child.value() {
                        scraper::Node::Text(text) => Some(text.text.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let quality_match = re.find(&quality_text)?;
                let quality = Quality::from_str(quality_match.as_str());

                Some((href.to_string(), quality))
            })
            .collect::<Vec<(String, Quality)>>();
        if qualities.is_empty() {
            return Err(AnimeDownloaderError::QualityNotFound(format!(
                "No qualities found for episode {}",
                self.episode_number
            )));
        }
        qualities.sort_by(|a, b| b.1.cmp(&a.1));

        // TODO: Implement quality sorting with fallback

        let selected_quality: &(String, Quality) = qualities
            .iter()
            .find(|(_, quality)| quality == &Quality::P1080)
            .or_else(|| qualities.first())
            .ok_or(AnimeDownloaderError::QualityNotFound(format!(
                "No suitable qualities found for episode {}",
                self.episode_number
            )))?;
        Ok(selected_quality.clone())
    }

    async fn get_kwik(&self, link: String) -> Result<String, AnimeDownloaderError> {
        let response = self
            .client
            .get(link)
            .send()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let text = response
            .text()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let document = scraper::Html::parse_document(text.as_str());
        let selector = scraper::Selector::parse("head > script:nth-child(24)").unwrap();
        let js_pahe = match document.select(&selector).next() {
            Some(x) => x,
            None => {
                return Err(AnimeDownloaderError::ExtractorError(
                    "Could not find the script tag".to_string(),
                ))
            }
        };
        let js_pahe = js_pahe.inner_html();

        // Fix: Store the owned string first, then get a reference to it
        let js_pahe_owned = js_pahe.to_owned();
        let js_pahe_str = js_pahe_owned.as_str();

        let re = Regex::new(r"https:\/\/kwik\.si\/f\/[a-zA-Z0-9]+").unwrap();
        let kwik_link = match re.find(js_pahe_str) {
            Some(x) => x.as_str().to_owned().to_string(),
            None => {
                return Err(AnimeDownloaderError::ExtractorError(
                    "Could not find the kwik link".to_string(),
                ))
            }
        };
        Ok(kwik_link)
    }

    async fn bypass_1(&self, link: &String) -> Result<(String, String), AnimeDownloaderError> {
        let response = self
            .client
            .get(link)
            .send()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let html = response
            .text()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let re = Regex::new(r###"\("(\w+)",\d+,"(\w+)",(\d+),(\d+),\d+\)"###).unwrap();
        let cap = match re.captures(&html) {
            Some(c) => c,
            None => {
                return Err(AnimeDownloaderError::ParsingError(
                    "Could not find encryption parameters".to_string(),
                ))
            }
        };
        let decrypted = self.decrypt(&cap[1], &cap[2], &cap[3], &cap[4]);

        let re_action = Regex::new(r###"action="(.+?)""###).unwrap();
        let download_url = match re_action.captures(&decrypted) {
            Some(c) => c[1].to_string(),
            None => {
                return Err(AnimeDownloaderError::ParsingError(
                    "Could not find form action URL".to_string(),
                ))
            }
        };

        let re_token = Regex::new(r###"value="(.+?)""###).unwrap();
        let token = match re_token.captures(&decrypted) {
            Some(c) => c[1].to_string(),
            None => {
                return Err(AnimeDownloaderError::ParsingError(
                    "Could not find form token".to_string(),
                ))
            }
        };
        Ok((download_url, token))
    }

    async fn bypass_2(
        &self,
        link: &String,
        token: &String,
        referer: &str,
    ) -> Result<String, AnimeDownloaderError> {
        let mut params = std::collections::HashMap::new();

        params.insert("_token", token);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("en-US,en;q=0.9"),
        );
        headers.insert("cache-control", HeaderValue::from_static("max-age=0"));
        headers.insert(
            "content-type",
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        headers.insert("origin", HeaderValue::from_static("https://kwik.si"));
        headers.insert(
            "referer",
            HeaderValue::from_str(referer)
                .map_err(|e| AnimeDownloaderError::ParsingError(e.to_string()))?,
        );
        headers.insert(
            "sec-ch-ua",
            HeaderValue::from_static(
                "\"Google Chrome\";v=\"135\", \"Not-A.Brand\";v=\"8\", \"Chromium\";v=\"135\"",
            ),
        );
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert(
            "sec-ch-ua-platform",
            HeaderValue::from_static("\"Windows\""),
        );
        headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
        headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36"));

        let response = self
            .client
            .post(link)
            .form(&params)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;
        let download_url = match response.headers().get(reqwest::header::LOCATION) {
            Some(location) => location
                .to_str()
                .map_err(|_| {
                    AnimeDownloaderError::ParsingError("Invalid Location header".to_string())
                })?
                .to_string(),
            None => {
                return Err(AnimeDownloaderError::ExtractorError(
                    "No redirect URL found".to_string(),
                ))
            }
        };
        Ok(download_url)
    }

    fn get_string(&self, content: &str, s1: u32, s2: usize) -> String {
        let slice_2 = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ+/";
        let slice_2 = &slice_2[0..s2];

        let mut acc = 0u32;
        for (n, ch) in content.chars().rev().enumerate() {
            let val = ch.to_digit(10).unwrap_or(0);
            acc += val * s1.pow(n as u32);
        }

        if acc == 0 {
            return "0".to_string();
        }

        let mut k = String::new();
        let s2_u32 = s2 as u32;

        while acc > 0 {
            let index = (acc % s2_u32) as usize;
            k.insert(0, slice_2.chars().nth(index).unwrap());
            acc /= s2_u32;
        }

        k
    }

    fn decrypt(&self, full_string: &str, key: &str, v1: &str, v2: &str) -> String {
        let v1: i32 = v1.parse().unwrap();
        let v2: usize = v2.parse().unwrap();
        let mut r = String::new();
        let mut i = 0;

        let chars: Vec<char> = full_string.chars().collect();
        let key_chars: Vec<char> = key.chars().collect();
        let key_v2 = key_chars[v2];

        while i < chars.len() {
            let mut s = String::new();
            while chars[i] != key_v2 {
                s.push(chars[i]);
                i += 1;
            }

            for (j, &kch) in key_chars.iter().enumerate() {
                s = s.replace(kch, &j.to_string());
            }

            let ch_val = self.get_string(&s, v2 as u32, 10).parse::<i32>().unwrap() - v1;
            r.push(char::from_u32(ch_val as u32).unwrap());

            i += 1;
        }

        r
    }
}

impl std::fmt::Display for AnimePaheEpisode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.title, self.episode_number)
    }
}
