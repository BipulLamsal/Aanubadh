pub mod types;
use crate::types::request::{Language, ResponseStatus, TranslationRequest, TranslationResponse};
use dotenvy::dotenv;
use futures::stream::{self, StreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use std::{env, sync::LazyLock};
use unicode_segmentation::UnicodeSegmentation;

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

fn is_abbreviation(s: &str) -> bool {
    let s = s.trim_end();
    if !s.ends_with('.') {
        return false;
    }

    let last_word = s.split_whitespace().last().unwrap_or("");
    let lower = last_word.to_lowercase();

    let abbreviations = [
        "mr.", "mrs.", "ms.", "dr.", "prof.", "sr.", "jr.", "inc.", "ltd.", "co.", "corp.", "vs.",
        "v.", "etc.", "e.g.", "i.e.", "al.", "st.", "rd.", "ave.", "blvd.", "jan.", "feb.", "mar.",
        "apr.", "aug.", "sept.", "oct.", "nov.", "dec.", "rs.", "no.",
    ];

    if abbreviations.contains(&lower.as_str()) {
        return true;
    }

    if last_word.len() == 2 && last_word.ends_with('.') {
        if let Some(c) = last_word.chars().next() {
            if c.is_alphabetic() {
                return true;
            }
        }
    }

    false
}

pub fn split_sentences(text: &str) -> Vec<String> {
    let raw_sentences: Vec<&str> = text.unicode_sentences().collect();
    let mut merged = Vec::new();
    let mut current = String::new();

    for s in raw_sentences {
        current.push_str(s);

        let is_abbr = is_abbreviation(&current);

        let trimmed_no_nl = current.trim_end();
        let lacks_punct = if current.ends_with('\n')
            && !current.ends_with("\n\n")
            && !current.ends_with("\r\n\r\n")
        {
            if let Some(c) = trimmed_no_nl.chars().last() {
                !matches!(c, '.' | '?' | '!' | ':' | ';' | '।')
            } else {
                false
            }
        } else {
            false
        };

        if !is_abbr && !lacks_punct {
            merged.push(current.clone());
            current.clear();
        }
    }

    if !current.is_empty() {
        merged.push(current);
    }

    merged
}

fn fix_punctuation(original: &str, translated: &str, target: &Language) -> String {
    let mut out = translated.trim().to_string();
    if out.is_empty() {
        return out;
    }

    let orig_trimmed = original.trim();
    if !orig_trimmed.contains(' ') && orig_trimmed.len() > 1 {
        let words: Vec<&str> = out.split_whitespace().collect();
        if words.len() == 2 && words[0] == words[1] {
            out = words[0].to_string();
        }
    }

    let is_nepali_or_tamang = matches!(target, Language::Nepali | Language::Tamang);

    if is_nepali_or_tamang {
        if original.trim_end().ends_with('.') {
            if out.ends_with('.') {
                out.pop();
                out.push('।');
            } else if !out.ends_with('।') && !out.ends_with('?') && !out.ends_with('!') {
                out.push('।');
            }
        } else if original.trim_end().ends_with('?') && !out.ends_with('?') {
            out.push('?');
        } else if original.trim_end().ends_with('!') && !out.ends_with('!') {
            out.push('!');
        }
    } else {
        if original.trim_end().ends_with('.')
            && !out.ends_with('.')
            && !out.ends_with('?')
            && !out.ends_with('!')
        {
            out.push('.');
        } else if original.trim_end().ends_with('?') && !out.ends_with('?') {
            out.push('?');
        } else if original.trim_end().ends_with('!') && !out.ends_with('!') {
            out.push('!');
        }
    }

    out
}

pub async fn translate_sentence(
    text: &str,
    src: Language,
    tgt: Language,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", &CONFIG.token))?,
    );

    let payload = TranslationRequest::new(text, src.clone(), tgt.clone());

    let mut attempts = 0;
    let max_attempts = 10;

    loop {
        let response = CONFIG
            .client
            .post(&CONFIG.base_url)
            .headers(headers.clone())
            .json(&payload)
            .send()
            .await?;

        let status_code = response.status();

        if status_code.as_u16() == 429 {
            attempts += 1;
            tracing::warn!(
                "Rate limited (attempt {}/{}), waiting {} seconds before retry",
                attempts,
                max_attempts,
                2u64.pow(attempts)
            );
            if attempts >= max_attempts {
                tracing::error!(
                    "Max retry attempts reached for sentence '{}', returning original",
                    text
                );
                return Ok(text.to_string());
            }
            let wait_secs = 2u64.pow(attempts);
            tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
            continue;
        }

        if !status_code.is_success() {
            tracing::error!(
                "Translation API returned error status {}: returning original text",
                status_code
            );
            return Ok(text.to_string());
        }

        let body = response.text().await?;
        let res: TranslationResponse = serde_json::from_str(&body)?;

        if res.message_type == ResponseStatus::Success {
            let raw_output = res.output;
            return Ok(fix_punctuation(text, &raw_output, &tgt));
        } else {
            tracing::error!(
                "Translation response status not successful for '{}': {:?}",
                text,
                res.message_type
            );
            return Ok(text.to_string());
        }
    }
}

pub async fn translate_text_parallel(
    text: &str,
    source: Language,
    target: Language,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if text.trim().is_empty() {
        return Ok(text.to_string());
    }

    let sentences = split_sentences(text);

    let translated_sentences = stream::iter(sentences)
        .map(|sentence| {
            let source = source.clone();
            let target = target.clone();

            async move {
                let trimmed = sentence.trim_end();
                if trimmed.is_empty() {
                    return sentence;
                }

                let trailing_ws = &sentence[trimmed.len()..];

                match translate_sentence(trimmed, source, target).await {
                    Ok(translated) => format!("{}{}", translated, trailing_ws),
                    Err(e) => {
                        tracing::error!("Translation failed for sentence '{}': {}", trimmed, e);
                        sentence
                    }
                }
            }
        })
        .buffered(50)
        .collect::<Vec<String>>()
        .await;

    Ok(translated_sentences.join(""))
}