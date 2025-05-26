use std::borrow::Cow;
use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use aria2_rs::options::TaskOptions;
use aria2_rs::status::{StatusKey, TaskStatus};
use aria2_rs::{self, ConnectionMeta};
use async_trait::async_trait;
use clearscreen;

use crate::common::download::Downloader;
use crate::common::errors::AnimeDownloaderError;
use crate::common::utils;

pub struct Aria2cDownoader {
    aria2c_bin: Arc<Mutex<Child>>,
    aria2c_client: Arc<aria2_rs::Client>,
    links: Arc<Mutex<Vec<String>>>,
    gid_completed: std::collections::HashMap<String, bool>,
    pub download_path: Option<String>,
}

impl Aria2cDownoader {
    pub async fn new() -> Result<Self, AnimeDownloaderError> {
        let aria2c_bin = Command::new("aria2c")
            .arg("--enable-rpc")
            .arg("--rpc-listen-all")
            .arg("--rpc-allow-origin-all")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| AnimeDownloaderError::Other(format!("Failed to start aria2c: {}", e)))?;

        let aria2c_client = aria2_rs::Client::connect(
            ConnectionMeta {
                url: "ws://localhost:6800/jsonrpc".to_string(),
                token: None,
            },
            20,
        )
        .await?;

        Ok(Self {
            aria2c_bin: Arc::new(Mutex::new(aria2c_bin)),
            aria2c_client: Arc::new(aria2c_client),
            links: Arc::new(Mutex::new(Vec::new())),
            gid_completed: std::collections::HashMap::new(),
            download_path: None,
        })
    }
}

#[async_trait]
impl Downloader for Aria2cDownoader {
    fn name(&self) -> &'static str {
        "aria2c"
    }

    async fn add_url_link(&mut self, url: String) -> Result<(), AnimeDownloaderError> {
        self.links
            .lock()
            .map_err(|e| AnimeDownloaderError::Aria2Error(format!("Failed to lock links: {}", e)))?
            .push(url);
        Ok(())
    }

    async fn download(&mut self) -> Result<(), AnimeDownloaderError> {
        let cloned_vec = {
            let guard = self
                .links
                .lock()
                .map_err(|e| {
                    AnimeDownloaderError::Aria2Error(format!("Failed to lock links: {}", e))
                })
                .unwrap();
            guard.clone()
        };
        let options = TaskOptions {
            dir: if self.download_path.is_some() {
                Some(self.download_path.clone().unwrap().as_str().into())
            } else {
                Some(env::current_dir()?.display().to_string().into())
            },
            // split: Some(2),
            // max_connection_per_server: Some(2),
            r#continue: Some(true),
            ..TaskOptions::default()
        };
        for link in cloned_vec {
            // options.dir = Some("E:\\Projects\\anime_downloader".to_string().into());
            let add_download_call = aria2_rs::call::AddUriCall {
                uris: vec![link].into(),
                options: Some(options.clone()),
            };
            let gid = self.aria2c_client.call(&add_download_call).await?;
            println!("Download started with GID: {:?}", &gid);
            self.gid_completed.insert(gid.clone().0.to_string(), false);
        }
        let gid_cloned = self.gid_completed.clone();
        let gids = gid_cloned.keys().cloned().collect::<Vec<_>>();
        loop {
            clearscreen::clear().map_err(|e| AnimeDownloaderError::Other(e.to_string()))?;
            for gid in gids.iter() {
                let status = self
                    .aria2c_client
                    .call(&aria2_rs::call::TellStatusCall {
                        gid: Cow::from(gid.as_str()),
                        keys: vec![
                            StatusKey::Status,
                            StatusKey::TotalLength,
                            StatusKey::CompletedLength,
                            StatusKey::DownloadSpeed,
                        ]
                        .into(),
                    })
                    .await?;
                if status
                    .status
                    .map(|a| a == TaskStatus::Complete || a == TaskStatus::Error)
                    .unwrap_or(false)
                {
                    println!("Download complete!");
                    self.gid_completed.insert(gid.to_owned().clone(), true);
                    continue;
                }
                let status_str = serde_json::to_value(status.status.unwrap())
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string();
                let download_speed = status.download_speed.unwrap();

                let t = status.total_length.unwrap_or(1) as f64;
                if t == 0.0 {
                    println!("Download progress: {} : 0%", status_str);
                    continue;
                }
                println!(
                    "Download progress: {} : {} : {:.2}%",
                    status_str,
                    utils::bytes_to_human_readable(download_speed),
                    (status.completed_length.unwrap_or(0) as f64 / t) * 100_f64
                );
            }
            std::thread::sleep(std::time::Duration::from_secs(8));
            // todo: check if it works
            if self.gid_completed.values().all(|&elem| elem) {
                println!("All downloads completed!");
                break;
            }
        }
        Ok(())
    }
}

impl Drop for Aria2cDownoader {
    fn drop(&mut self) {
        if let Err(e) = self
            .aria2c_bin
            .lock()
            .map_err(|e| AnimeDownloaderError::Aria2Error(e.to_string()))
            .unwrap()
            .kill()
        {
            eprintln!("Failed to kill aria2c process: {}", e);
        }
    }
}
