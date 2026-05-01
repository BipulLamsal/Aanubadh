mod docx;
mod handler;
mod pdf;

use axum::Router;
use axum::routing::post;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("tmt=info".parse().unwrap()))
        .with_target(false)
        .compact()
        .init();

    let app = Router::new().route("/translate", post(handler::translate));

    let addr = "0.0.0.0:1997";
    tracing::info!(addr = %addr, "server starting");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
