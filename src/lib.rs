pub mod error;
pub mod parser;
pub mod types;

use std::{env, sync::LazyLock};

use dotenvy::dotenv;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};

use crate::types::request::{
    ApiError, Language, ResponseStatus, TranslationRequest, TranslationResponse,
};

struct Config {
    token: String,
    base_url: String,
}

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    dotenv().ok();
    Config {
        token: env::var("API_TOKEN").expect("API_TOKEN must be set in .env or environment"),
        base_url: "https://tmt.ilprl.ku.edu.np/lang-translate".to_string(),
    }
});

pub async fn send_translation_request(
    text: &str,
    src: Language,
    tgt: Language,
) -> Result<TranslationResponse, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", &CONFIG.token))?,
    );

    let payload = TranslationRequest::new(text, src, tgt);

    let response = client
        .post(&CONFIG.base_url)
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    if status.is_success() {
        let res_data: TranslationResponse = serde_json::from_str(&response_text)?;

        if res_data.message_type == ResponseStatus::Success {
            Ok(res_data)
        } else {
            Err(res_data.message.into())
        }
    } else {
        let err_data: ApiError = serde_json::from_str(&response_text).unwrap_or(ApiError {
            message: "Unknown API error".to_string(),
        });

        Err(err_data.message.into())
    }
}
