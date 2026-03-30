#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use crate::wifi::{build_networking_stack, connect_wifi, setup_tcp, setup_wifi};
use alloc::vec::Vec;
use defmt::{error, info};
use embedded_io::{Read, Write};
use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::time::{Duration, Instant};
use esp_hal::timer::timg::TimerGroup;
use esp_println::print;
use smoltcp::wire::IpAddress;

use {esp_backtrace as _, esp_println as _};

extern crate alloc;
mod wifi;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // === device setup === //
    // TODO: configure clock
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // === networking setup === //
    let radio_controller: esp_radio::Controller<'static> = esp_radio::init().unwrap();

    let (mut wifi_controller, mut wifi_device) =
        setup_wifi(&radio_controller, peripherals.WIFI).unwrap();

    let (tcp_interface, mut tcp_sockets) = setup_tcp(&mut wifi_device);

    let net_stack = build_networking_stack(wifi_device, tcp_interface, &mut tcp_sockets);

    if let Some(e) = connect_wifi(&mut wifi_controller) {
        error!("WiFi connection error: {}", e);
        panic!();
    }

    // === get IP address for server === //
    let server_ip_parts: Vec<u8> = env!("SERVER_IP")
        .split(".")
        .map(|p| p.parse::<u8>().unwrap())
        .collect();
    assert!(
        server_ip_parts.len() == 4,
        "Server IP must be a valid IPv4 address"
    );
    let server_ip: IpAddress = IpAddress::v4(
        server_ip_parts[0],
        server_ip_parts[1],
        server_ip_parts[2],
        server_ip_parts[3],
    );
    let server_port: u16 = env!("SERVER_PORT").parse().unwrap();

    // === send http request === //
    let mut tcp_write_buffer = [0u8; 2048];
    let mut tcp_read_buffer = [0u8; 2048];
    let mut tcp_socket = net_stack.get_socket(&mut tcp_read_buffer, &mut tcp_write_buffer);
    tcp_socket.work();
    tcp_socket.open(server_ip, server_port).unwrap();
    loop {
        tcp_socket.write(b"GET / HTTP/1.0").unwrap(); // TODO
        tcp_socket.flush().unwrap();

        // === listen for http response === //
        let mut tcp_socket_buffer = [0u8; 512];
        while let Ok(len) = tcp_socket.read(&mut tcp_socket_buffer) {
            let part = unsafe { core::str::from_utf8_unchecked(&tcp_socket_buffer[..len]) };
            print!("{part}");
            tcp_socket.work();
        }

        info!("Hello world!");
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }
    // tcp_socket.disconnect();

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
