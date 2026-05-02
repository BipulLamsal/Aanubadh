use std::path::Path;
use std::sync::Arc;
use tmt::send_translation_request;
use tmt::types::request::Language;
use tokio::sync::Semaphore;

fn pdf_to_html(input_pdf: &Path, output_html: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dest_dir = output_html.parent().unwrap();
    let out_name = output_html.file_name().unwrap();

    let status = std::process::Command::new("pdf2htmlEX")
        .args([
            "--optimize-text",
            "1",
            "--dest-dir",
            dest_dir.to_str().unwrap(),
            input_pdf.to_str().unwrap(),
            out_name.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        return Err("pdf2htmlEX failed".into());
    }
    Ok(())
}

fn is_text_tag(tag: &str) -> bool {
    if let Some(pos) = tag.find("class=\"") {
        let after = &tag[pos + 7..];
        if let Some(end) = after.find('"') {
            let classes = &after[..end];
            return classes.split_whitespace().next() == Some("t");
        }
    }
    false
}

fn collect_text_ranges(html: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut depth: usize = 0;

    while i < len {
        if bytes[i] != b'<' {
            let start = i;
            while i < len && bytes[i] != b'<' {
                i += 1;
            }
            if depth == 1 {
                let text = &html[start..i];
                let decoded = html_escape::decode_html_entities(text);
                if !decoded.trim().is_empty() {
                    ranges.push((start, i));
                }
            }
            continue;
        }

        let tag_start = i;
        i += 1;
        while i < len && bytes[i] != b'>' {
            if bytes[i] == b'"' {
                i += 1;
                while i < len && bytes[i] != b'"' {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
        }
        if i < len {
            i += 1;
        }

        let tag = &html[tag_start..i];

        if tag.starts_with("</") {
            if depth > 0 {
                depth -= 1;
            }
        } else if tag.starts_with("<!--") || tag.starts_with("<!") || tag.starts_with("<?") {
            // ignore
        } else {
            let self_closing = tag.ends_with("/>");
            if depth > 0 {
                if !self_closing {
                    depth += 1;
                }
            } else if is_text_tag(tag) && !self_closing {
                depth = 1;
            }
        }
    }

    ranges
}

async fn translate_html_text(
    html: &str,
    src: Language,
    tgt: Language,
) -> Result<String, Box<dyn std::error::Error>> {
    let ranges = collect_text_ranges(html);
    tracing::info!(text_nodes = ranges.len(), "pdf text nodes found");

    if ranges.is_empty() {
        tracing::warn!("no text nodes found — returning html unchanged");
        return Ok(html.to_string());
    }

    let semaphore = Arc::new(Semaphore::new(10));
    let html_arc = Arc::new(html.to_string());
    let mut handles = Vec::new();

    for &(start, end) in &ranges {
        let sem = semaphore.clone();
        let src = src.clone();
        let tgt = tgt.clone();
        let html_ref = html_arc.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let raw = &html_ref[start..end];
            let decoded = html_escape::decode_html_entities(raw).into_owned();
            let trimmed = decoded.trim();

            if trimmed.is_empty() {
                return (start, end, raw.to_string());
            }

            match send_translation_request(trimmed, src, tgt).await {
                Ok(resp) => {
                    tracing::debug!(original = %trimmed, translated = %resp.output);
                    let escaped = html_escape::encode_text(&resp.output).into_owned();
                    (start, end, escaped)
                }
                Err(e) => {
                    tracing::warn!(err = %e, "translation failed, keeping original");
                    (start, end, raw.to_string())
                }
            }
        }));
    }

    let mut results: Vec<(usize, usize, String)> = Vec::new();
    for h in handles {
        results.push(h.await?);
    }
    results.sort_by_key(|(s, _, _)| *s);

    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    for (start, end, translated) in results {
        out.push_str(&html[cursor..start]);
        out.push_str(&translated);
        cursor = end;
    }
    out.push_str(&html[cursor..]);

    Ok(out)
}

pub async fn translate_pdf(
    input_pdf: impl AsRef<Path>,
    output_html: impl AsRef<Path>,
    src_lang: Language,
    tgt_lang: Language,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output_html.as_ref();
    let dest_dir = output_path.parent().unwrap();
    let temp_path = dest_dir.join("_pdf2html_intermediate.html");

    pdf_to_html(input_pdf.as_ref(), &temp_path)?;

    let html = std::fs::read_to_string(&temp_path)?;
    tracing::info!(bytes = html.len(), "read intermediate html");

    let translated = translate_html_text(&html, src_lang, tgt_lang).await?;
    std::fs::write(output_path, translated.as_bytes())?;

    let _ = std::fs::remove_file(&temp_path);
    Ok(())
}
