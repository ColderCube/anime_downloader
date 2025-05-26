
#[allow(dead_code)]
#[derive(Debug)]
pub enum AnimeDownloaderError {
    NetworkError(String),
    ParsingError(String),
    NotFoundError(String),
    QualityNotFound(String),
    EpisodeUnavailable(String),
    IoError(std::io::Error),
    ExtractorError(String),
    ConfigError(String),
    Aria2Error(String),
    UserInputError(String),
    Other(String),
}

impl From<std::io::Error> for AnimeDownloaderError {
    fn from(err: std::io::Error) -> Self {
        AnimeDownloaderError::IoError(err)
    }
}

impl From<reqwest::Error> for AnimeDownloaderError {
    fn from(err: reqwest::Error) -> Self {
        AnimeDownloaderError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for AnimeDownloaderError {
    fn from(err: serde_json::Error) -> Self {
        AnimeDownloaderError::ParsingError(err.to_string())
    }
}

impl From<aria2_rs::Error> for AnimeDownloaderError {
    fn from(err: aria2_rs::Error) -> Self {
        AnimeDownloaderError::Aria2Error(err.to_string())
    }
}

impl std::error::Error for AnimeDownloaderError {}

impl std::fmt::Display for AnimeDownloaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnimeDownloaderError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AnimeDownloaderError::ParsingError(msg) => write!(f, "Parsing error: {}", msg),
            AnimeDownloaderError::NotFoundError(msg) => write!(f, "Not found error: {}", msg),
            AnimeDownloaderError::QualityNotFound(msg) => {
                write!(f, "Quality not found error: {}", msg)
            }
            AnimeDownloaderError::EpisodeUnavailable(msg) => {
                write!(f, "Episode unavailable error: {}", msg)
            }
            AnimeDownloaderError::IoError(err) => write!(f, "IO error: {}", err),
            AnimeDownloaderError::ExtractorError(msg) => write!(f, "Extractor error: {}", msg),
            AnimeDownloaderError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            AnimeDownloaderError::Aria2Error(msg) => write!(f, "Aria2 error: {}", msg),
            AnimeDownloaderError::UserInputError(msg) => write!(f, "User input error: {}", msg),
            AnimeDownloaderError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}
