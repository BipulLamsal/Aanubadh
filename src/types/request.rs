use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ne")]
    Nepali,
    #[serde(rename = "tmg")]
    Tamang,
}

impl FromStr for Language {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "en" => Ok(Language::English),
            "ne" => Ok(Language::Nepali),
            "tmg" => Ok(Language::Tamang),
            _ => Err(format!("unknown language: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ResponseStatus {
    Success,
    Fail,
}

#[derive(Debug, Serialize)]
pub struct TranslationRequest {
    pub text: String,
    pub src_lang: Language,
    pub tgt_lang: Language,
}

#[derive(Debug, Deserialize)]
pub struct TranslationResponse {
    pub message_type: ResponseStatus,
    pub message: String,
    pub src_lang: String,
    pub input: String,
    pub target_lang: String,
    pub output: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub message: String,
}

impl TranslationRequest {
    pub fn new(text: impl Into<String>, src: Language, tgt: Language) -> Self {
        Self {
            text: text.into(),
            src_lang: src,
            tgt_lang: tgt,
        }
    }
}
