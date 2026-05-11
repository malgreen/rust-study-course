use axum::{routing::get, routing::post, Router};

mod api;
mod web;

const LISTEN_PORT: &str = env!("LISTEN_PORT");

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{LISTEN_PORT}"))
        .await
        .unwrap();

    let router = Router::new()
        // web routes
        .route("/", get(web::index::get))
        // api routes
        .route("/api/data", post(api::data::post));

    println!("Listening on port {}", LISTEN_PORT);
    axum::serve(listener, router).await.unwrap();
}
