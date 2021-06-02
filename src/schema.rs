use serde::Deserialize;
use std::str::FromStr;
use crate::schema::Format::{Audio, Video};
use crate::schema::Language::{Chinese, English};
use anyhow::anyhow;

#[derive(Deserialize, Debug, Copy, Clone)]
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

#[derive(Deserialize, Debug, Copy, Clone)]
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

#[derive(Deserialize, Clone, Debug)]
pub struct Entry {
    pub pid: u32,
    pub eid: u32,
    pub programme_title: String,
    pub episode_title: String,
    pub episode_date: String,
    pub duration_seconds: Option<f32>,
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