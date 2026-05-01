use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use axum::extract::{Multipart, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;

use tmt::types::request::Language;

#[derive(Clone)]
pub struct AppState {
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    work_dir: PathBuf,
}

struct Job {
    progress: Arc<AtomicU8>,
    status: JobStatus,
    output_path: PathBuf,
    output_filename: String,
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum JobStatus {
    Processing,
    Done,
    Error,
}

#[derive(Serialize)]
struct TranslateResp {
    job_id: String,
}

#[derive(Serialize)]
struct ProgressResp {
    progress: u8,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn parse_lang(s: &str) -> Option<Language> {
    match s {
        "en" => Some(Language::English),
        "ne" => Some(Language::Nepali),
        "tmg" => Some(Language::Tamang),
        _ => None,
    }
}

async fn translate_handler(
    State(st): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<TranslateResp>, (StatusCode, String)> {
    let mut file_data: Option<(String, Vec<u8>)> = None;
    let mut src: Option<Language> = None;
    let mut tgt: Option<Language> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let fname = field.file_name().unwrap_or("file").to_string();
                let data = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                file_data = Some((fname, data.to_vec()));
            }
            "src_lang" => {
                let t = field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                src = parse_lang(&t);
            }
            "tgt_lang" => {
                let t = field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                tgt = parse_lang(&t);
            }
            _ => {}
        }
    }

    let (filename, data) = file_data.ok_or((StatusCode::BAD_REQUEST, "No file".into()))?;
    let src = src.ok_or((StatusCode::BAD_REQUEST, "Invalid src_lang".into()))?;
    let tgt = tgt.ok_or((StatusCode::BAD_REQUEST, "Invalid tgt_lang".into()))?;

    let ext = Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    let stem = Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_ext = if ext == "pdf" { "docx" } else { &ext };
    let out_filename = format!("{}_translated.{}", stem, out_ext);

    let job_id = Uuid::new_v4().to_string();
    let input_path = st.work_dir.join(format!("{}_in_{}", job_id, filename));
    let output_path = st.work_dir.join(format!("{}_out_{}", job_id, out_filename));

    tokio::fs::write(&input_path, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let progress = Arc::new(AtomicU8::new(0));

    st.jobs.write().await.insert(
        job_id.clone(),
        Job {
            progress: progress.clone(),
            status: JobStatus::Processing,
            output_path: output_path.clone(),
            output_filename: out_filename,
            error: None,
        },
    );

    let jobs = st.jobs.clone();
    let jid = job_id.clone();

    tokio::spawn(async move {
        let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = match ext.as_str() {
            "docx" => crate::docx::translate_docx(
                input_path.to_str().unwrap(),
                output_path.to_str().unwrap(),
                src,
                tgt,
                progress.clone(),
            )
            .await
            .map_err(|e| e.into()),
            "csv" | "tsv" => crate::csv::translate_csv(
                &input_path,
                &output_path,
                src,
                tgt,
                progress.clone(),
            )
            .await
            .map_err(|e| e.into()),
            "pdf" => crate::pdf::translate_pdf(
                &input_path,
                &output_path,
                src,
                tgt,
                progress.clone(),
            )
            .await
            .map_err(|e| e.into()),
            _ => Err("Unsupported format".into()),
        };

        let mut jobs = jobs.write().await;
        if let Some(job) = jobs.get_mut(&jid) {
            match result {
                Ok(()) => {
                    job.status = JobStatus::Done;
                    job.progress.store(100, Ordering::Relaxed);
                }
                Err(e) => {
                    job.status = JobStatus::Error;
                    job.error = Some(e.to_string());
                }
            }
        }
        let _ = tokio::fs::remove_file(&input_path).await;
    });

    Ok(Json(TranslateResp { job_id }))
}

async fn progress_handler(
    State(st): State<AppState>,
    AxumPath(job_id): AxumPath<String>,
) -> Result<Json<ProgressResp>, StatusCode> {
    let jobs = st.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ProgressResp {
        progress: job.progress.load(Ordering::Relaxed),
        status: match job.status {
            JobStatus::Processing => "processing",
            JobStatus::Done => "done",
            JobStatus::Error => "error",
        },
        error: job.error.clone(),
    }))
}

async fn download_handler(
    State(st): State<AppState>,
    AxumPath(job_id): AxumPath<String>,
) -> Result<([(axum::http::header::HeaderName, String); 2], Vec<u8>), StatusCode> {
    let jobs = st.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    if job.status != JobStatus::Done {
        return Err(StatusCode::CONFLICT);
    }

    let data = tokio::fs::read(&job.output_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let filename = job.output_filename.clone();

    Ok((
        [
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
            (
                axum::http::header::CONTENT_TYPE,
                "application/octet-stream".to_string(),
            ),
        ],
        data,
    ))
}

pub fn create_router(frontend_dir: &str) -> Router {
    let work_dir = PathBuf::from(".work");
    std::fs::create_dir_all(&work_dir).expect("Failed to create .work dir");

    let state = AppState {
        jobs: Arc::new(RwLock::new(HashMap::new())),
        work_dir,
    };

    Router::new()
        .route("/api/translate", post(translate_handler))
        .route("/api/progress/{id}", get(progress_handler))
        .route("/api/download/{id}", get(download_handler))
        .fallback_service(ServeDir::new(frontend_dir))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
