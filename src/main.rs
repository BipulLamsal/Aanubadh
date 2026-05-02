mod docx;
mod handler;
mod pdf;
mod csv;
mod txt;

use axum::Router;
use axum::routing::post;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("tmt".parse().unwrap()))
        .with_target(false)
        .compact()
        .init();

    let app = Router::new()
        .route("/translate", post(handler::translate))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = "127.0.0.1:1997";
    tracing::info!(addr = %addr, "server starting");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
