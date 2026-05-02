use csv::{ReaderBuilder, WriterBuilder};
use std::sync::Arc;
use tmt::send_translation_request;
use tmt::types::request::Language;
use tokio::sync::Semaphore;

pub async fn translate_csv(
    input_data: &[u8],
    src_lang: Language,
    tgt_lang: Language,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false) // Assuming we translate everything including headers
        .from_reader(input_data);

    let mut translated_records: Vec<Vec<String>> = Vec::new();

    for result in reader.records() {
        let record = result?;
        let mut row = Vec::new();
        for field in record.iter() {
            row.push(field.to_string());
        }
        translated_records.push(row);
    }

    let mut cells_to_translate = Vec::new();
    for (r_idx, row) in translated_records.iter().enumerate() {
        for (c_idx, cell) in row.iter().enumerate() {
            let text = cell.trim();
            if !text.is_empty() {
                cells_to_translate.push((r_idx, c_idx, text.to_string()));
            }
        }
    }

    let semaphore = Arc::new(Semaphore::new(20)); // Limit concurrency
    let mut tasks = Vec::new();

    for (r_idx, c_idx, text) in cells_to_translate {
        let sem = semaphore.clone();
        let src = src_lang.clone();
        let tgt = tgt_lang.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            match send_translation_request(&text, src, tgt).await {
                Ok(resp) => {
                    tracing::debug!(original = %text, translated = %resp.output);
                    (r_idx, c_idx, resp.output)
                }
                Err(e) => {
                    tracing::warn!(err = %e, "csv cell translation failed, keeping original");
                    (r_idx, c_idx, text)
                }
            }
        }));
    }

    for task in tasks {
        let (r_idx, c_idx, translated_text) = task.await?;
        translated_records[r_idx][c_idx] = translated_text;
    }

    let mut writer = WriterBuilder::new().from_writer(Vec::new());
    for row in translated_records {
        writer.write_record(&row)?;
    }
    
    let output_data = writer.into_inner()?;
    Ok(output_data)
}
