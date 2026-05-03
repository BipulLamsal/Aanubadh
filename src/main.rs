mod docx;
mod handler;
mod pdf;
mod csv;
mod txt;

use axum::Router;
use axum::routing::post;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("tmt".parse().unwrap()))
        .with_target(false)
        .compact()
        .init();

    // Ensure the translated_files directory exists
    std::fs::create_dir_all("translated_files").ok();

    // Determine paths for serving the frontend
    // In production (Docker), the built frontend is at ./frontend/dist
    // In development, it may or may not exist
    let frontend_dir = std::path::Path::new("frontend/dist");

    let app = Router::new()
        .route("/translate", post(handler::translate))
        // Serve translated files so Microsoft Viewer can access them
        .nest_service("/files", ServeDir::new("translated_files"))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // If the frontend dist exists, serve the SPA as a fallback
    let app = if frontend_dir.exists() {
        let index_file = frontend_dir.join("index.html");
        app.fallback_service(
            ServeDir::new(frontend_dir)
                .not_found_service(ServeFile::new(index_file)),
        )
    } else {
        tracing::warn!("frontend/dist not found, not serving SPA (dev mode)");
        app
    };

    // Use PORT env var (set by Render) or default to 1997
    let port = std::env::var("PORT").unwrap_or_else(|_| "1997".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(addr = %addr, "server starting");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
