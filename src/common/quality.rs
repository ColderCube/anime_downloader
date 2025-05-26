#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Quality {
    P360,
    P480,
    P540,
    P720,
    P1080,
    P1440,
    P2160, // 4K
    Other(String),
}

#[allow(dead_code)]
impl Quality {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "360p" => Quality::P360,
            "480p" => Quality::P480,
            "540p" => Quality::P540,
            "720p" => Quality::P720,
            "1080p" => Quality::P1080,
            "1440p" => Quality::P1440,
            "2160p" | "4k" => Quality::P2160,
            _ => Quality::Other(s.to_string()),
        }
    }

    pub fn to_string_p(&self) -> String {
        match self {
            Quality::P360 => "360p".to_string(),
            Quality::P480 => "480p".to_string(),
            Quality::P540 => "540p".to_string(),
            Quality::P720 => "720p".to_string(),
            Quality::P1080 => "1080p".to_string(),
            Quality::P1440 => "1440p".to_string(),
            Quality::P2160 => "2160p".to_string(),
            Quality::Other(s) => s.clone(),
        }
    }
}
