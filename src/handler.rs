use axum::body::Body;
use axum::extract::Multipart;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;

use tmt::types::request::Language;

use crate::docx;
use crate::pdf;
use crate::csv;
use crate::txt;

use std::time::{SystemTime, UNIX_EPOCH};

pub async fn translate(mut multipart: Multipart) -> Response {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut src_str: Option<String> = None;
    let mut tgt_str: Option<String> = None;

    loop {
        match multipart.next_field().await {
            Err(e) => {
                tracing::error!(err = %e, "failed to read multipart field");
                return error_response(StatusCode::BAD_REQUEST, &e.to_string());
            }
            Ok(None) => break,
            Ok(Some(field)) => {
                let name = field.name().unwrap_or_default().to_string();
                match name.as_str() {
                    "file" => {
                        file_name = field.file_name().map(str::to_string);
                        match field.bytes().await {
                            Ok(b) => file_data = Some(b.to_vec()),
                            Err(e) => {
                                tracing::error!(err = %e, "failed to read file bytes");
                                return error_response(StatusCode::BAD_REQUEST, &e.to_string());
                            }
                        }
                    }
                    "src" => match field.text().await {
                        Ok(v) => src_str = Some(v.trim().to_string()),
                        Err(e) => return error_response(StatusCode::BAD_REQUEST, &e.to_string()),
                    },
                    "tgt" => match field.text().await {
                        Ok(v) => tgt_str = Some(v.trim().to_string()),
                        Err(e) => return error_response(StatusCode::BAD_REQUEST, &e.to_string()),
                    },
                    _ => {}
                }
            }
        }
    }

    let file_data = match file_data {
        Some(d) => d,
        None => return error_response(StatusCode::BAD_REQUEST, "missing file field"),
    };
    let file_name = match file_name {
        Some(n) => n,
        None => return error_response(StatusCode::BAD_REQUEST, "missing file name"),
    };
    let src = match src_str.as_deref() {
        Some(s) => match s.parse::<Language>() {
            Ok(l) => l,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &e),
        },
        None => return error_response(StatusCode::BAD_REQUEST, "missing src field"),
    };

    let tgt = match tgt_str.as_deref() {
        Some(s) => match s.parse::<Language>() {
            Ok(l) => l,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &e),
        },
        None => return error_response(StatusCode::BAD_REQUEST, "missing tgt field"),
    };

    let extension = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    tracing::info!(file = %file_name, ext = %extension, "received translation request");

    let result = match extension.as_str() {
        "docx" => handle_docx(&file_data, src, tgt, file_name).await,
        "pdf" => handle_pdf(&file_data, src, tgt, file_name).await,
        "csv" => handle_csv(&file_data, src, tgt, file_name).await,
        "txt" => handle_txt(&file_data, src, tgt, file_name).await,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("unsupported file type: {}", extension),
            );
        }
    };

    match result {
        Ok(r) => r,
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn handle_docx(
    file_data: &[u8],
    src: Language,
    tgt: Language,
    original_name: String,
) -> Result<Response, String> {
    // processing document
    let output_data = docx::process_docx_translation(file_data, src, tgt)
        .await
        .map_err(|e| e.to_string())?;

    // Build response directly
    let response = file_response(
        output_data,
        &format!("translated_{}", original_name),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );

    Ok(response)
}

fn cleanup_old_files(dir: &str, max_age_secs: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = SystemTime::now().duration_since(modified) {
                    if age.as_secs() > max_age_secs {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}

async fn handle_pdf(
    file_data: &[u8],
    src: Language,
    tgt: Language,
    original_name: String,
) -> Result<Response, String> {
    // temporary directory to hold files
    let tmp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input_path = tmp_dir.path().join("in.pdf");
    let output_path = tmp_dir.path().join("out.pdf");

    std::fs::write(&input_path, file_data).map_err(|e| e.to_string())?;

    pdf::translate_pdf(&input_path, &output_path, src, tgt)
        .await
        .map_err(|e| e.to_string())?;

    let bytes = std::fs::read(&output_path).map_err(|e| e.to_string())?;

    Ok(file_response(
        bytes,
        &original_name.replace(".pdf", "_translated.pdf"),
        "application/pdf",
    ))
}

async fn handle_csv(
    file_data: &[u8],
    src: Language,
    tgt: Language,
    original_name: String,
) -> Result<Response, String> {
    // csv file processing
    let output_data = csv::translate_csv(file_data, src, tgt)
        .await
        .map_err(|e| e.to_string())?;

    Ok(file_response(
        output_data,
        &original_name.replace(".csv", "_translated.csv"),
        "text/csv; charset=utf-8",
    ))
}

async fn handle_txt(
    file_data: &[u8],
    src: Language,
    tgt: Language,
    original_name: String,
) -> Result<Response, String> {
    // txt file processing
    let output_data = txt::translate_txt(file_data, src, tgt)
        .await
        .map_err(|e| e.to_string())?;

    Ok(file_response(
        output_data,
        &original_name.replace(".txt", "_translated.txt"),
        "text/plain; charset=utf-8",
    ))
}

fn file_response(bytes: Vec<u8>, filename: &str, content_type: &str) -> Response {
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(content_type).unwrap(),
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename)).unwrap(),
        )
        .body(Body::from(bytes))
        .unwrap()
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    Response::builder()
        .status(status)
        .body(Body::from(msg.to_string()))
        .unwrap()
}
