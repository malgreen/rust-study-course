use axum::{http::header, response::{Html, IntoResponse, Response}};

pub async fn get() -> Response {
    println!("GET /");
    let mut response = Html("<h1>Hello, World!</h1>\0").into_response();

    response.headers_mut().insert(header::CONNECTION, "close".parse().unwrap());

    response
}



