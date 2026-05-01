use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tmt::{send_translation_request, types::request::Language};

pub async fn translate_csv(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    src: Language,
    tgt: Language,
    progress: Arc<AtomicU8>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = csv::Reader::from_path(&input_path)?;
    let headers = rdr.headers()?.clone();
    let records: Vec<csv::StringRecord> = rdr.records().collect::<Result<_, _>>()?;

    let cells: Vec<(usize, usize, String)> = records
        .iter()
        .enumerate()
        .flat_map(|(r, record)| {
            record
                .iter()
                .enumerate()
                .map(move |(c, field)| (r, c, field.to_string()))
        })
        .collect();

    let mut grid: Vec<Vec<String>> = records
        .iter()
        .map(|r| r.iter().map(str::to_string).collect())
        .collect();

    const BATCH: usize = 55;
    let total_batches = cells.chunks(BATCH).len().max(1);
    let mut batches_done = 0usize;

    for chunk in cells.chunks(BATCH) {
        let handles: Vec<_> = chunk
            .iter()
            .map(|(row, col, text)| {
                let text = text.clone();
                let row = *row;
                let col = *col;
                tokio::spawn(async move {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        return Ok((row, col, text));
                    }
                    match send_translation_request(trimmed, src, tgt).await {
                        Ok(resp) => Ok((row, col, resp.output)),
                        Err(e) => Err(e),
                    }
                })
            })
            .collect();

        for handle in handles {
            let (row, col, translated) = handle.await??;
            grid[row][col] = translated;
        }

        batches_done += 1;
        progress.store((batches_done * 95 / total_batches) as u8, Ordering::Relaxed);
        sleep(Duration::from_secs(1)).await;
    }

    let mut wtr = csv::Writer::from_path(&output_path)?;
    wtr.write_record(&headers)?;
    for row in &grid {
        wtr.write_record(row)?;
    }
    wtr.flush()?;

    progress.store(100, Ordering::Relaxed);
    Ok(())
}