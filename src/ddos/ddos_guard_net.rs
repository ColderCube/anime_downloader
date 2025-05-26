use crate::common::errors::AnimeDownloaderError;
use reqwest::{self, header::HeaderValue};
use reqwest_cookie_store::CookieStoreMutex;
use reqwest_middleware::ClientWithMiddleware;

pub async fn bypass(
    client: &ClientWithMiddleware,
    cookies: &CookieStoreMutex,
    base_url: &str,
) -> Result<(), AnimeDownloaderError> {
    let body = match std::fs::read_to_string("data.json") {
        Ok(content) => content,
        Err(_) => {
            return Err(AnimeDownloaderError::ConfigError(
                "data.json not found".to_string(),
            ))
        }
    };

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

    let response = client
        .get("https://check.ddos-guard.net/check.js")
        .headers(headers.clone())
        .send()
        .await
        .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;

    if response.status() != reqwest::StatusCode::OK {
        return Err(AnimeDownloaderError::NetworkError(
            "Failed to bypass protection".to_string(),
        ));
    }

    let resp_text = response
        .text()
        .await
        .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;

    let links = resp_text.split("\'").collect::<Vec<&str>>();

    client
        .get(links[3])
        .headers(headers.clone())
        .send()
        .await
        .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;

    client
        .get(format!("{}{}", base_url, links[1]))
        .headers(headers.clone())
        .send()
        .await
        .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;

    client
        .post(format!("{}/.well-known/ddos-guard/mark/", base_url))
        .headers(headers.clone())
        .body(body)
        .send()
        .await
        .map_err(|e| AnimeDownloaderError::NetworkError(e.to_string()))?;

    let mut writer = match std::fs::File::create("cookies.json").map(std::io::BufWriter::new) {
        Ok(writer) => writer,
        Err(e) => return Err(AnimeDownloaderError::IoError(e)),
    };
    let store = cookies.lock().unwrap();
    cookie_store::serde::json::save(&store, &mut writer).unwrap();

    Ok(())
}
