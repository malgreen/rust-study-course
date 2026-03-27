use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9876").await.unwrap();

    let router = Router::new()
        .route("/", get("Hello!"))
        .route("/goodbye", get("Goodbye!"));

    axum::serve(listener, router).await.unwrap();
}
