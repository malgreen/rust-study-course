#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use crate::sensor::CO2Sensor;
use crate::wifi::{assign_ip_address, build_networking_stack, connect_wifi, setup_tcp, setup_wifi};

use alloc::format;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::main;
use esp_hal::time::{Duration, Instant, Rate};
use esp_hal::timer::timg::TimerGroup;

use defmt::{error, info};
use {esp_backtrace as _, esp_println as _};

// Interrupt
// use core::cell::{Cell, RefCell};
// use critical_section::Mutex;
// use esp_hal::gpio::{Event, PullUp, GPIO36};
// use esp_hal::IO;

// static BUTTON: Mutex<RefCell<Option<GPIO36<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));

extern crate alloc;
mod http;
mod wifi;

mod sensor;

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
    // let config_i2c = OtherConfig::default().with_frequency(Rate::from_khz(100));
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

    loop {
        match connect_wifi(&mut wifi_controller) {
            Ok(_) => break,
            Err(e) => {
                error!("WiFi connection error: {} - retrying in 1 second...", e);
                let timeout = Instant::now() + Duration::from_secs(1);
                while Instant::now() < timeout {}
                continue;
            }
        }
    }
    assign_ip_address(&mut net_stack);

    let mut tcp_write_buffer = [0u8; 2048];
    let mut tcp_read_buffer = [0u8; 2048];
    let mut tcp_socket = net_stack.get_socket(&mut tcp_read_buffer, &mut tcp_write_buffer);

    // === sensor setup === //
    let mut co2_sensor: CO2Sensor =
        CO2Sensor::new(peripherals.I2C0, peripherals.GPIO22, peripherals.GPIO21);
    match co2_sensor.find_dev() {
        Ok(addr) => info!("Device found at 0x{:02X}", addr),
        Err(e) => {
            loop {}
        }
    }

    if let Err(e) = co2_sensor.read_status() {
        error!("Failed to read status: {:?}", e);
        loop {}
    }

    co2_sensor.enable_irq(true);

    if let Err(e) = co2_sensor.meas_setup(sensor::MeasurementDriveMode::Mode11S) {
        error!("Failed to set meas mode: {:?}", e);
        loop {}
    }
    if let Err(e) = co2_sensor.read_meas_mode() {
        error!("Failed to read meas mode: {:?}", e);
        loop {}
    }

    let input_config = InputConfig::default().with_pull(Pull::Up);
    let mut interrupt_pin = Input::new(peripherals.GPIO36, input_config);

    let room = env!("MOTE_ROOM");

    // === main loop === //
    loop {
        while !interrupt_pin.is_low() {}

        let (eco2, tvoc) = match co2_sensor.read_data() {
            Ok((eco2, tvoc)) => (eco2, tvoc),
            Err(e) => {
                error!("Failed to read data: {:?}", e);
                continue;
            }
        };


        let body = format!("{{\r\n\
            \"room\": \"{room}\",\r\n\
            \"eco2\": {eco2},\r\n\
            \"tvoc\": {tvoc}\r\n\
        }}");

        match http::send_post(&mut tcp_socket, body.as_str()) {
            Ok(_) => {},
            Err(e) => {
                error!("HTTP Post Request Failed: {:?}", e);
                continue
            },
        }
    }

}
