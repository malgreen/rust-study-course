# Server

This package serves a website and API endpoints.
It uses [`axum`](https://docs.rs/axum/latest/axum/) for the HTTP routing, and [`maud`](https://maud.lambda.xyz/) for the HTML rendering.

## Setup

1. flash Raspberry Pi OS Lite (w. SSH access)
1. ssh into rpi with `ssh pi@co2pi.local` and password `co2pass`
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
