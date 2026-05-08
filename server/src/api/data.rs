// use axum::{
//     extract::Json,
//     // routing::post,
//     handler::Handler,
//     Router,
// };
// use serde::Deserialize;

// #[derive(Deserialize)]
// struct DataBody {
//     email: String,
//     password: String,
// }

// pub async fn post(Json(body): Json<DataBody>) {
pub async fn post(body: String) {
    println!("posted data: {}", body);
}
