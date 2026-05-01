use docx_rs::{Docx, Paragraph, Run};
use pdf_oxide::PdfDocument;
use std::io::Cursor;
use std::path::Path;
use tmt::{send_translation_request, types::request::Language};

struct TextBlock {
    text: String,
    font_size: f32,
}

// After sorting words, merge any token that looks like a broken word:
// if token[i] ends without a space and token[i+1] starts with a lowercase letter
// and there is no horizontal gap between them, glue them together.
fn merge_broken_tokens(words: &mut Vec<pdf_oxide::layout::Word>) {
    let mut i = 0;
    while i + 1 < words.len() {
        let gap = words[i + 1].bbox.x - (words[i].bbox.x + words[i].bbox.width);
        let same_line = (words[i + 1].bbox.y - words[i].bbox.y).abs() < 2.0;
        // gap < 2.0 pt means they are visually touching — broken glyph
        let looks_broken = gap < 2.0
            && same_line
            && words[i + 1]
                .text
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false);

        if looks_broken {
            let next_text = words[i + 1].text.clone();
            words[i].text.push_str(&next_text);
            // widen bbox to cover both
            words[i].bbox.width = (words[i + 1].bbox.x + words[i + 1].bbox.width) - words[i].bbox.x;
            words.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

fn extract_blocks(
    doc: &PdfDocument,
    line_gap: f32,
) -> Result<Vec<TextBlock>, Box<dyn std::error::Error>> {
    let page_count = doc.page_count()?;
    let mut blocks = Vec::new();

    for page in 0..page_count {
        let words = doc.extract_words(page)?;
        if words.is_empty() {
            continue;
        }

        let mut sorted = words;
        // y=0 is BOTTOM of page so we need to sort descending so top of page comes first
        sorted.sort_by(|a, b| {
            b.bbox
                .y
                .partial_cmp(&a.bbox.y)
                .unwrap()
                .then(a.bbox.x.partial_cmp(&b.bbox.x).unwrap())
        });

        merge_broken_tokens(&mut sorted);

        let mut cur_text = sorted[0].text.clone();
        let mut cur_y = sorted[0].bbox.y;
        let mut cur_h = sorted[0].bbox.height;

        for word in sorted.iter().skip(1) {
            if (word.bbox.y - cur_y).abs() <= line_gap {
                cur_text.push(' ');
                cur_text.push_str(&word.text);
                if word.bbox.height > cur_h {
                    cur_h = word.bbox.height;
                }
            } else {
                let trimmed = cur_text.trim().to_string();
                if !trimmed.is_empty() {
                    blocks.push(TextBlock {
                        text: trimmed,
                        font_size: cur_h,
                    });
                }
                cur_text = word.text.clone();
                cur_y = word.bbox.y;
                cur_h = word.bbox.height;
            }
        }

        let trimmed = cur_text.trim().to_string();
        if !trimmed.is_empty() {
            blocks.push(TextBlock {
                text: trimmed,
                font_size: cur_h,
            });
        }

        if let Ok(tables) = doc.extract_tables(page) {
            for table in tables {
                for row in &table.rows {
                    for cell in &row.cells {
                        let trimmed = cell.text.trim().to_string();
                        if !trimmed.is_empty() {
                            blocks.push(TextBlock {
                                text: trimmed,
                                font_size: 11.0,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(blocks)
}

async fn translate_blocks(
    blocks: &[TextBlock],
    src: Language,
    tgt: Language,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(50));
    let mut handles = Vec::new();

    for block in blocks {
        let text = block.text.clone();
        let sem = semaphore.clone();
        let src = src.clone();
        let tgt = tgt.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return text.clone();
            }
            match send_translation_request(trimmed, src, tgt).await {
                Ok(resp) => resp.output,
                Err(_) => text.clone(),
            }
        }));
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await?);
    }
    Ok(results)
}

pub async fn translate_pdf(
    input_pdf: impl AsRef<Path>,
    output_docx: impl AsRef<Path>,
    src_lang: Language,
    tgt_lang: Language,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_path = &input_pdf;
    let output_path = &output_docx;

    let doc = PdfDocument::open(input_path)?;
    let blocks = extract_blocks(&doc, 4.0)?;

    let translated = translate_blocks(&blocks, src_lang, tgt_lang).await?;

    let body_size: f32 = {
        let mut sizes: Vec<f32> = blocks.iter().map(|b| b.font_size).collect();
        sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sizes.get(sizes.len() / 2).copied().unwrap_or(11.0)
    };

    let mut docx = Docx::new();
    for (block, trans_text) in blocks.iter().zip(translated.iter()) {
        if trans_text.is_empty() {
            continue;
        }
        let sz = (block.font_size * 2.0).round() as usize;
        let para = if block.font_size > body_size * 1.4 {
            Paragraph::new()
                .add_run(Run::new().add_text(trans_text).bold().size(sz))
                .style("Heading1")
        } else if block.font_size > body_size * 1.15 {
            Paragraph::new()
                .add_run(Run::new().add_text(trans_text).bold().size(sz))
                .style("Heading2")
        } else {
            Paragraph::new().add_run(Run::new().add_text(trans_text).size(sz))
        };
        docx = docx.add_paragraph(para);
    }

    let mut buf = Cursor::new(Vec::new());
    docx.build().pack(&mut buf)?;
    std::fs::write(output_path, buf.into_inner())?;
    Ok(())
}
