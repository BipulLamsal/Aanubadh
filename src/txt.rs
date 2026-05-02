use tmt::types::request::Language;
use tmt::translate_text_parallel;

/// Translate a plain-text file.
///
/// Strategy
/// --------
/// * Split on single newlines to preserve line-by-line formatting
/// * Each line is translated independently to maintain structure
/// * Empty lines are preserved
/// * The resulting bytes are UTF-8 encoded text.
pub async fn translate_txt(
    data: &[u8],
    source: Language,
    target: Language,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Translating TXT");

    let text = std::str::from_utf8(data)?;

    // Normalise line endings to \n so the rest of the logic is simple.
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");

    // Split on single newlines to preserve formatting
    let lines: Vec<&str> = normalised.split('\n').collect();

    let mut translated_lines: Vec<String> = Vec::with_capacity(lines.len());

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Preserve blank lines as-is
            translated_lines.push(String::new());
            continue;
        }

        // Translate the line
        match translate_text_parallel(trimmed, source.clone(), target.clone()).await {
            Ok(translated) => {
                translated_lines.push(translated);
            }
            Err(e) => {
                tracing::error!("Failed to translate line '{}': {}", trimmed, e);
                // Keep original line on error
                translated_lines.push(trimmed.to_string());
            }
        }
    }

    let output = translated_lines.join("\n");

    Ok(output.into_bytes())
}