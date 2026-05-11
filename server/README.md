# Server

This package serves a website and API endpoints.
It uses [`axum`](https://docs.rs/axum/latest/axum/) for the HTTP routing, and [`maud`](https://maud.lambda.xyz/) for the HTML rendering.

## Setup

1. flash Raspberry Pi OS Lite (w. SSH access)
1. ssh into rpi with `ssh pi@co2pi.local` and password `co2pass`
1. install docker and docker-desktop with:

    ```sh
    sudo apt install docker.io docker-compose
    ```

1. copy InfluxDB files with:

    ```sh
    scp compose.yml pi@co2pi.local:/home/pi/compose.yml
    scp influxdb.env pi@co2pi.local:/home/pi/influxdb.env
    ```

1. run InfluxDB with

    ```sh
    sudo docker compose up -d
    ```

1. go to `co2pi.local:8086` in a browser and setup InfluxDB. '`co2`' should be used as bucket name, otherwise change `.cargo/config.toml`.

1. enable hotspot with:

    ```sh
    sudo nmcli device wifi hotspot ssid co2wifi password co2password ifname wlan0 
    ```

1. install `Cross` with:

    ```sh
    cargo install cross --git https://github.com/cross-rs/cross
    ```

## Build

1. `cross build --target aarch64-unknown-linux-musl`
1. `scp target/aarch64-unknown-linux-musl/debug/server pi@co2pi.local:/home/pi/co2server`

or just: `make pi`
