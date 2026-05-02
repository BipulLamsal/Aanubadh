pub mod types;
use crate::types::request::{
    ApiError, Language, ResponseStatus, TranslationRequest, TranslationResponse,
};
use dotenvy::dotenv;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use std::{env, sync::LazyLock};

struct Config {
    token: String,
    base_url: String,
    client: reqwest::Client,
}

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    dotenv().ok();
    Config {
        token: env::var("API_TOKEN").expect("API_TOKEN must be set in .env or environment"),
        base_url: "https://tmt.ilprl.ku.edu.np/lang-translate".to_string(),
        client: reqwest::Client::new(),
    }
});

pub async fn send_translation_request(
    text: &str,
    src: Language,
    tgt: Language,
) -> Result<TranslationResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", &CONFIG.token))?,
    );

    let payload = TranslationRequest::new(text, src, tgt);
    tracing::debug!(src = ?src, tgt = ?tgt, len = text.len(), "sending translation request");

    let max_retries = 5;
    let mut attempt = 0;

    loop {
        attempt += 1;

        let response = CONFIG
            .client
            .post(&CONFIG.base_url)
            .headers(headers.clone())
            .json(&payload)
            .send()
            .await?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            if attempt > max_retries {
                tracing::error!("max retries exceeded for rate limit");
                return Err("Rate limit exceeded".into());
            }

            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or_else(|| 2u64.pow(attempt as u32)); // 2, 4, 8, 16, 32

            tracing::warn!(
                attempt = attempt,
                retry_after_secs = retry_after,
                "rate limit hit, sleeping before retry"
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(retry_after)).await;
            continue;
        }

        let body = response.text().await?;

        if status.is_success() {
            let res: TranslationResponse = serde_json::from_str(&body)?;
            if res.message_type == ResponseStatus::Success {
                return Ok(res);
            } else {
                tracing::warn!(msg = %res.message, "translation api returned failure");
                return Err(res.message.into());
            }
        } else {
            let err: ApiError = serde_json::from_str(&body).unwrap_or(ApiError {
                message: "Unknown API error".to_string(),
            });
            tracing::error!(status = %status, msg = %err.message, "translation api error");
            return Err(err.message.into());
        }
    }
}
