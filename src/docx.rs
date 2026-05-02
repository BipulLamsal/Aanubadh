use regex::Regex;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;

use tmt::types::request::Language;
use tmt::{translate_sentence, translate_text_parallel};

fn find_bytes(haystack: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + start)
}

fn find_next_wt(bytes: &[u8], start: usize) -> Option<usize> {
    let p1 = find_bytes(bytes, start, b"<w:t>");
    let p2 = find_bytes(bytes, start, b"<w:t ");
    match (p1, p2) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

fn xml_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn xml_encode(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

struct TextNode {
    open_tag: String,
    text: String,
    close_tag: String,
}

enum XmlToken {
    Literal(String),
    Text(TextNode),
}

fn parse_wt_tokens(xml: &str) -> Vec<XmlToken> {
    let bytes = xml.as_bytes();
    let close = b"</w:t>";
    let mut tokens = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        let open_start = match find_next_wt(bytes, pos) {
            Some(idx) => idx,
            None => {
                tokens.push(XmlToken::Literal(xml[pos..].to_string()));
                break;
            }
        };

        if open_start > pos {
            tokens.push(XmlToken::Literal(xml[pos..open_start].to_string()));
        }

        let tag_end = match bytes[open_start..].iter().position(|&b| b == b'>') {
            Some(p) => open_start + p + 1,
            None => {
                tokens.push(XmlToken::Literal(xml[open_start..].to_string()));
                break;
            }
        };

        let open_tag = xml[open_start..tag_end].to_string();
        pos = tag_end;

        let close_start = match find_bytes(bytes, pos, close) {
            Some(idx) => idx,
            None => {
                tokens.push(XmlToken::Literal(xml[open_start..].to_string()));
                break;
            }
        };

        let raw_text = &xml[pos..close_start];
        pos = close_start + close.len();

        tokens.push(XmlToken::Text(TextNode {
            open_tag,
            text: xml_decode(raw_text),
            close_tag: "</w:t>".to_string(),
        }));
    }

    tokens
}

async fn translate_all_wt_nodes(
    xml: &str,
    source: Language,
    target: Language,
    from_pdf: bool,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let tokens = parse_wt_tokens(xml);
    let mut out = String::with_capacity(xml.len() + xml.len() / 4);

    for token in tokens {
        match token {
            XmlToken::Literal(s) => out.push_str(&s),
            XmlToken::Text(node) => {
                let translated = if node.text.trim().is_empty() {
                    node.text
                } else if from_pdf {
                    translate_sentence(&node.text, source.clone(), target.clone())
                        .await
                        .unwrap_or(node.text)
                } else {
                    translate_text_parallel(&node.text, source.clone(), target.clone())
                        .await
                        .unwrap_or(node.text)
                };
                out.push_str(&node.open_tag);
                out.push_str(&xml_encode(&translated));
                out.push_str(&node.close_tag);
            }
        }
    }

    Ok(out)
}

fn reduce_xml_font_sizes(xml: &str) -> String {
    let re = Regex::new(r#"(w:sz(?:Cs)?\s+w:val=")(\d+)""#).unwrap();
    re.replace_all(xml, |caps: &regex::Captures| {
        let prefix = &caps[1];
        if let Ok(val) = caps[2].parse::<i32>() {
            let new_val = (val as f32 * 0.82).round() as i32;
            format!("{}{}\"", prefix, new_val)
        } else {
            caps[0].to_string()
        }
    })
    .to_string()
}

pub async fn translate_docx_via_xml(
    data: &[u8],
    source: Language,
    target: Language,
    from_pdf: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data))?;

    let mut entries: Vec<(String, Vec<u8>, bool)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let is_dir = entry.is_dir();
        let mut content = Vec::new();
        if !is_dir {
            entry.read_to_end(&mut content)?;
        }
        entries.push((name, content, is_dir));
    }
    drop(archive);

    let targets = [
        "word/document.xml",
        "word/header1.xml",
        "word/header2.xml",
        "word/header3.xml",
        "word/footer1.xml",
        "word/footer2.xml",
        "word/footer3.xml",
    ];

    let mut new_entries: Vec<(String, Vec<u8>, bool)> = Vec::new();
    for (name, content, is_dir) in entries {
        if !is_dir && targets.contains(&name.as_str()) {
            // translating specific target files
            let xml = String::from_utf8_lossy(&content).to_string();
            let mut translated_xml =
                translate_all_wt_nodes(&xml, source.clone(), target.clone(), from_pdf).await?;

            if matches!(target, Language::Nepali | Language::Tamang) {
                // reducing font size so the text fits
                translated_xml = reduce_xml_font_sizes(&translated_xml);
            }

            new_entries.push((name, translated_xml.into_bytes(), false));
        } else {
            new_entries.push((name, content, is_dir));
        }
    }

    let mut out_buf = Cursor::new(Vec::new());
    {
        // zipping everything back together
        let mut writer = zip::ZipWriter::new(&mut out_buf);
        let file_opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let dir_opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, content, is_dir) in new_entries {
            if is_dir {
                writer.add_directory(&name, dir_opts)?;
            } else {
                writer.start_file(&name, file_opts)?;
                writer.write_all(&content)?;
            }
        }
        writer.finish()?;
    }
    Ok(out_buf.into_inner())
}

pub async fn process_docx_translation(
    data: &[u8],
    source: Language,
    target: Language,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Translating DOCX");
    translate_docx_via_xml(data, source, target, false).await
}
