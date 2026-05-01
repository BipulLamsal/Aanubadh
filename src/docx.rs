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

// translate all w:t nodes inside a word/document.xml byte slice
// shared by both the .docx path and the pdf→docx path
pub async fn process_document_xml(
    xml_bytes: &[u8],
    src: Language,
    tgt: Language,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_reader(xml_bytes);
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut w_t = false;

    // we store the index of the word for events
    let mut events: Vec<Event<'static>> = Vec::new();
    let mut text_indices: Vec<(usize, String)> = Vec::new();

    loop {
        // we are only looking for w:t elements only
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name().as_ref() == b"w:t" => {
                w_t = true;
                events.push(Event::Start(e.into_owned()));
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"w:t" => {
                w_t = false;
                events.push(Event::End(e.into_owned()));
            }
            Ok(Event::Text(e)) if w_t => {
                let raw_str = std::str::from_utf8(&e)?;
                let text = match quick_xml::escape::unescape(raw_str) {
                    Ok(Cow::Borrowed(s)) => s.to_string(),
                    Ok(Cow::Owned(s)) => s,
                    Err(_) => raw_str.to_string(),
                };
                if !text.trim().is_empty() {
                    text_indices.push((events.len(), text));
                }
                events.push(Event::Text(e.into_owned()));
            }

            Ok(Event::Eof) => break,

            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(e) => {
                events.push(e.into_owned());
            }
        }
    }

    let semaphore = Arc::new(Semaphore::new(50));
    let mut handles = Vec::new();

    for (idx, text) in text_indices {
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _sem = sem.acquire().await.unwrap();

            let trimmed = text.trim();
            if trimmed.is_empty() {
                return (idx, text);
            }

            let start_spaces = &text[..text.find(trimmed).unwrap_or(0)];
            let end_spaces_idx = text.rfind(trimmed).unwrap_or(0) + trimmed.len();
            let end_spaces = &text[end_spaces_idx..];

            let response = match send_translation_request(trimmed, src, tgt).await {
                Ok(resp) => format!("{}{}{}", start_spaces, resp.output, end_spaces),
                Err(_) => text,
            };
            (idx, response)
        }));
    }

    for handle in handles {
        let (idx, translated_text) = handle.await?;
        // writing translated text to events
        events[idx] = Event::Text(BytesText::new(&translated_text).into_owned());
    }

    // writing serially stored events again to xml
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
        let mut file = archive.by_index(i)?;
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        if file.name() == "word/document.xml" {
            let mut xml_bytes = Vec::new();
            file.read_to_end(&mut xml_bytes)?;

            let new_xml_bytes = process_document_xml(&xml_bytes, src, tgt).await?;

            zip_writer.start_file("word/document.xml", options)?;
            zip_writer.write_all(&new_xml_bytes)?;
        } else {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            zip_writer.start_file(file.name(), options)?;
            zip_writer.write_all(&buffer)?;
        }
    }

    zip_writer.finish()?;
    Ok(())
}
