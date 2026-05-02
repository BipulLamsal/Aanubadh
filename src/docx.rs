use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::{BytesText, Event};
use std::borrow::Cow;
use std::fs::File;
use std::io::Cursor;
use std::io::{Read, Write};
use std::sync::Arc;
use tmt::send_translation_request;
use tokio::sync::Semaphore;
use zip::write::SimpleFileOptions;

use tmt::types::request::Language;

pub async fn process_document_xml(
    xml_bytes: &[u8],
    src: Language,
    tgt: Language,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_reader(xml_bytes);
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut in_w_t = false;
    let mut events: Vec<Event> = Vec::new();
    let mut text_indices: Vec<(usize, String)> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name().as_ref() == b"w:t" => {
                in_w_t = true;
                events.push(Event::Start(e.into_owned()));
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"w:t" => {
                in_w_t = false;
                events.push(Event::End(e.into_owned()));
            }
            Ok(Event::Text(e)) if in_w_t => {
                let raw = std::str::from_utf8(e.as_ref())?;
                let text = match quick_xml::escape::unescape(raw) {
                    Ok(Cow::Borrowed(s)) => s.to_string(),
                    Ok(Cow::Owned(s)) => s,
                    Err(_) => raw.to_string(),
                };

                if !text.trim().is_empty() {
                    text_indices.push((events.len(), text.clone()));
                }
                events.push(Event::Text(e.into_owned()));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("XML error at {}: {:?}", reader.error_position(), e).into());
            }
            Ok(e) => events.push(e.into_owned()),
        }
    }

    tracing::info!(
        text_nodes = text_indices.len(),
        "docx text nodes to translate"
    );

    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = Vec::new();

    for (idx, text) in text_indices {
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let trimmed = text.trim();
            if trimmed.is_empty() {
                return (idx, text);
            }

            let leading = &text[..text.find(trimmed).unwrap_or(0)];
            let trailing_start = text.rfind(trimmed).unwrap_or(0) + trimmed.len();
            let trailing = &text[trailing_start..];

            match send_translation_request(trimmed, src, tgt).await {
                Ok(resp) => {
                    tracing::debug!(original = %trimmed, translated = %resp.output);
                    (idx, format!("{}{}{}", leading, resp.output, trailing))
                }
                Err(e) => {
                    tracing::warn!(err = %e, text = %trimmed, "translation failed, keeping original");
                    (idx, text)
                }
            }
        }));
    }

    for handle in handles {
        let (idx, translated) = handle.await?;
        events[idx] = Event::Text(BytesText::new(&translated).into_owned());
    }

    for event in events {
        writer.write_event(event)?;
    }

    Ok(writer.into_inner().into_inner())
}

pub async fn translate_docx(
    input_path: &str,
    output_path: &str,
    src: Language,
    tgt: Language,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let output_file = File::create(output_path)?;
    let mut zip_writer = zip::ZipWriter::new(output_file);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        if entry.name() == "word/document.xml" {
            let mut xml_bytes = Vec::new();
            entry.read_to_end(&mut xml_bytes)?;
            tracing::info!(bytes = xml_bytes.len(), "processing word/document.xml");

            let new_xml = process_document_xml(&xml_bytes, src, tgt).await?;
            zip_writer.start_file("word/document.xml", options)?;
            zip_writer.write_all(&new_xml)?;
        } else {
            let name = entry.name().to_string();
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            zip_writer.start_file(name, options)?;
            zip_writer.write_all(&buf)?;
        }
    }

    zip_writer.finish()?;
    Ok(())
}
