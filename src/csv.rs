use tmt::types::request::Language;
use tmt::translate_text_parallel;

fn needs_translation(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    
    if !trimmed.chars().any(|c| c.is_alphabetic()) {
        return false;
    }
    
    if (trimmed.starts_with("http://") || trimmed.starts_with("https://")) && !trimmed.contains(' ') {
        return false;
    }
    if trimmed.contains('@') && !trimmed.contains(' ') {
        return false;
    }
    
    true
}

pub async fn translate_csv(
    data: &[u8],
    source: Language,
    target: Language,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Translating CSV");
    
    // creating reader to parse the csv file
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data);
    let mut writer = csv::Writer::from_writer(Vec::new());

    for result in reader.records() {
        let record = result?;
        let mut translated_row = Vec::new();
        
        // iterating through every cell
        for cell in record.iter() {
            if needs_translation(cell) {
                let translated = translate_text_parallel(cell, source.clone(), target.clone()).await?;
                translated_row.push(translated);
            } else {
                translated_row.push(cell.to_string());
            }
        }
        
        writer.write_record(&translated_row)?;
    }

    writer.flush()?;

    Ok(writer.into_inner().unwrap())
}
