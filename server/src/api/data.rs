use axum::Json;
use futures::stream;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DataRequest {
    room: String,
    eco2: u16,
    tvoc: u16,
}

pub async fn post(Json(body): Json<DataRequest>) {
    println!("POST /api/data");

    let influx_client = Client::new(
        env!("INFLUXDB_URL"),
        env!("INFLUXDB_ORG"),
        env!("INFLUXDB_TOKEN"),
    );

    influx_client
        .write(
            env!("INFLUXDB_BUCKET"),
            stream::iter(vec![
                DataPoint::builder("co2")
                    .tag("room", body.room)
                    .field("eco2", body.eco2 as i64)
                    .field("tvoc", body.tvoc as i64)
                    .build()
                    .unwrap(),
            ]),
        )
        .await
        .unwrap_or_else(|e| {
            println!("ERROR /api/data - {}", e);
        })
}
