use axum::{Router, routing::get, routing::post};

mod api;
mod web;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9876").await.unwrap();

    let router = Router::new()
        // web routes
        .route("/", get(web::index::get))
        // api routes
        .route("/api/data", post(api::data::post));

    axum::serve(listener, router).await.unwrap();
}
