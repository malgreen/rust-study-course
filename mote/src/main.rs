#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use crate::http::http_loop;
use crate::wifi::{assign_ip_address, build_networking_stack, connect_wifi, setup_tcp, setup_wifi};
use defmt::error;
use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::timer::timg::TimerGroup;

use {esp_backtrace as _, esp_println as _};

extern crate alloc;
mod http;
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
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // === networking setup === //
    let radio_controller: esp_radio::Controller<'static> = esp_radio::init().unwrap_or_else(|e| {
        error!("ESP Radio initialization error: {}", e);
        panic!();
    });

    let (mut wifi_controller, mut wifi_device) = setup_wifi(&radio_controller, peripherals.WIFI)
        .unwrap_or_else(|e| {
            error!("WiFi initialization error: {}", e);
            panic!();
        });

    let (tcp_interface, mut tcp_sockets) = setup_tcp(&mut wifi_device);

    let mut net_stack = build_networking_stack(wifi_device, tcp_interface, &mut tcp_sockets);

    connect_wifi(&mut wifi_controller).unwrap_or_else(|e| {
        error!("WiFi connection error: {}", e);
        panic!();
    });

    assign_ip_address(&mut net_stack);

    let mut tcp_write_buffer = [0u8; 2048];
    let mut tcp_read_buffer = [0u8; 2048];
    let mut tcp_socket = net_stack.get_socket(&mut tcp_read_buffer, &mut tcp_write_buffer);

    // === main loops === //
    http_loop(&mut tcp_socket);

    // TODO: why is this necessary?
    loop {}
}
