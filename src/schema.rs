use crate::schema::Format::{Audio, Video};
use crate::schema::Language::{Chinese, English};
use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Language {
    #[serde(rename = "中文")]
    Chinese,
    #[serde(rename = "英文")]
    English,
}

impl FromStr for Language {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chinese" => Ok(Chinese),
            "english" => Ok(English),
            _ => Err(anyhow!("invalid language")),
        }
    }
}

impl ToString for Language {
    fn to_string(&self) -> String {
        match self {
            Chinese => "chinese".to_string(),
            English => "english".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Audio,
    Video,
}

impl FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "audio" => Ok(Audio),
            "video" => Ok(Video),
            _ => Err(anyhow!("invalid format")),
        }
    }
}

impl ToString for Format {
    fn to_string(&self) -> String {
        match self {
            Audio => "audio".to_string(),
            Video => "video".to_string(),
        }
    }
}

impl Format {
    pub fn extension(&self) -> &str {
        match self {
            Audio => "mp3",
            Video => "mp4",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct Entry {
    #[serde(flatten)]
    pub id: Id,
    pub programme_title: String,
    pub episode_title: String,
    pub episode_date: String,
    #[serde(deserialize_with = "de_from_f32")]
    pub duration_seconds: Option<u32>,
    pub og_title: String,
    pub og_description: String,
    pub cids: String,
    pub category_names: String,
    pub file_url: String,
    pub m3u8_url: Option<String>,
    pub rss_url: String,
    pub language: Language,
    pub format: Format,
}

fn de_from_f32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let f: Option<f32> = Option::deserialize(deserializer)?;
    Ok(f.map(|v| v as u32))
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Id {
    pid: u32,
    eid: u32,
}
