use scraper::{Html, Selector};
use std::path::Path;
use std::sync::Arc;
use tmt::send_translation_request;
use tmt::types::request::Language;
use tokio::sync::Semaphore;

fn pdf_to_html(input_pdf: &Path, output_html: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = std::process::Command::new("pdf2htmlEX")
        .args([input_pdf.to_str().unwrap(), output_html.to_str().unwrap()])
        .status()?;
    if !status.success() {
        return Err("pdf2htmlEX failed".into());
    }
    Ok(())
}

async fn translate_html_text(
    html: &str,
    src: Language,
    tgt: Language,
) -> Result<String, Box<dyn std::error::Error>> {
    // Html and Selector are not Send, so we drop them before any .await
    let texts: Vec<String> = {
        let document = Html::parse_document(html);
        let selector = Selector::parse("div.t").unwrap();
        document
            .select(&selector)
            .map(|el| el.text().collect::<String>())
            .filter(|t| !t.trim().is_empty())
            .collect()
    };

    let semaphore = Arc::new(Semaphore::new(50));
    let handles: Vec<_> = texts
        .into_iter()
        .map(|text| {
            let sem = semaphore.clone();
            let src = src.clone();
            let tgt = tgt.clone();
            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                match send_translation_request(text.trim(), src, tgt).await {
                    Ok(resp) => (text, resp.output),
                    Err(_) => (text.clone(), text),
                }
            })
        })
        .collect();

    let mut output_html = html.to_string();
    for handle in handles {
        let (original, translated) = handle.await?;
        output_html = output_html.replacen(&original, &translated, 1);
    }
    Ok(output_html)
}

pub async fn translate_pdf(
    input_pdf: impl AsRef<Path>,
    output_html: impl AsRef<Path>,
    src_lang: Language,
    tgt_lang: Language,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_html = tempfile::Builder::new().suffix(".html").tempfile()?;
    let temp_path = temp_html.path().to_path_buf();

    pdf_to_html(input_pdf.as_ref(), &temp_path)?;
    let html = std::fs::read_to_string(&temp_path)?;
    let translated = translate_html_text(&html, src_lang, tgt_lang).await?;
    std::fs::write(output_html.as_ref(), translated.as_bytes())?;

    Ok(())
}
