use axum::{Router, routing::get, routing::post};
use local_ip_address::local_ip;

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

    println!("Serving on {}:{}", local_ip().unwrap(), LISTEN_PORT);
    axum::serve(listener, router).await.unwrap();
}
