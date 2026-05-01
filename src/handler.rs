use axum::body::Body;
use axum::extract::Multipart;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use std::path::PathBuf;
use tmt::types::request::Language;

use crate::docx;
use crate::pdf;

fn parse_language(s: &str) -> Result<Language, Response> {
    match s {
        "en" => Ok(Language::English),
        "ne" => Ok(Language::Nepali),
        "tmg" => Ok(Language::Tamang),
        _ => {
            tracing::warn!(lang = %s, "unknown language code");
            Err(error_response(
                StatusCode::BAD_REQUEST,
                &format!("unknown language: {}", s),
            ))
        }
    }
}

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
        Some(s) => match parse_language(s) {
            Ok(l) => l,
            Err(r) => return r,
        },
        None => return error_response(StatusCode::BAD_REQUEST, "missing src field"),
    };
    let tgt = match tgt_str.as_deref() {
        Some(s) => match parse_language(s) {
            Ok(l) => l,
            Err(r) => return r,
        },
        None => return error_response(StatusCode::BAD_REQUEST, "missing tgt field"),
    };

    let extension = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    tracing::info!(file = %file_name, ext = %extension, "received translation request");

    let tmp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(err = %e, "failed to create temp dir");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    };
    let input_path = tmp_dir.path().join(&file_name);
    if let Err(e) = std::fs::write(&input_path, &file_data) {
        tracing::error!(err = %e, "failed to write uploaded file");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }

    let result = match extension.as_str() {
        "docx" => {
            let output_path = tmp_dir.path().join("translated.docx");
            handle_docx(input_path, output_path, src, tgt, file_name).await
        }
        "pdf" => {
            let output_path = tmp_dir.path().join("translated.html");
            handle_pdf(input_path, output_path, src, tgt, file_name).await
        }
        _ => {
            tracing::warn!(ext = %extension, "unsupported file type");
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
    input_path: PathBuf,
    output_path: PathBuf,
    src: Language,
    tgt: Language,
    original_name: String,
) -> Result<Response, String> {
    docx::translate_docx(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        src,
        tgt,
    )
    .await
    .map_err(|e| {
        tracing::error!(err = %e, "docx translation failed");
        e.to_string()
    })?;

    tracing::info!(file = %original_name, "docx translation complete");
    let bytes = std::fs::read(&output_path).map_err(|e| {
        tracing::error!(err = %e, "failed to read translated docx");
        e.to_string()
    })?;

    Ok(file_response(
        bytes,
        &format!("translated_{}", original_name),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ))
}

async fn handle_pdf(
    input_path: PathBuf,
    output_path: PathBuf,
    src: Language,
    tgt: Language,
    original_name: String,
) -> Result<Response, String> {
    pdf::translate_pdf(&input_path, &output_path, src, tgt)
        .await
        .map_err(|e| {
            tracing::error!(err = %e, "pdf translation failed");
            e.to_string()
        })?;

    tracing::info!(file = %original_name, "pdf translation complete");
    let bytes = std::fs::read(&output_path).map_err(|e| {
        tracing::error!(err = %e, "failed to read translated html");
        e.to_string()
    })?;

    Ok(file_response(
        bytes,
        &original_name.replace(".pdf", ".html"),
        "text/html; charset=utf-8",
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
