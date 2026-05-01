mod csv;
mod docx;
mod pdf;
mod server;

#[tokio::main]
async fn main() {
    let app = server::create_router("./frontend");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Server running at http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}
