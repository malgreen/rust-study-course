#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use crate::sensor::CO2Sensor;

use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::i2c::master::Config as OtherConfig;
use esp_hal::main;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use {esp_backtrace as _, esp_println as _};
use defmt::{info, error};

// Interrupt
// use core::cell::{Cell, RefCell};
// use critical_section::Mutex;
// use esp_hal::gpio::{Event, PullUp, GPIO36};
// use esp_hal::IO;

// static BUTTON: Mutex<RefCell<Option<GPIO36<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));

extern crate alloc;

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
    // generator version: 1.2.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    // let config_i2c = OtherConfig::default().with_frequency(Rate::from_khz(100));
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    let mut co2_sensor: CO2Sensor =
        CO2Sensor::new(peripherals.I2C0, peripherals.GPIO22, peripherals.GPIO21);
    match co2_sensor.find_dev() {
        Ok(addr) => info!("Device found at 0x{:02x}", addr),
        Err(e) => {
            loop {}
        }
    }

    if let Err(e) = co2_sensor.read_status() {
        error!("Failed to read status: {:?}", e);
        loop {}
    }

    if let Err(e) = co2_sensor.meas_setup(sensor::MeasurementDriveMode::Mode11S, true) {
        error!("Failed to set meas mode: {:?}", e);
        loop {}
    }
    if let Err(e) = co2_sensor.read_meas_mode() {
        error!("Failed to read meas mode: {:?}", e);
        loop {}
    }

    let input_config = InputConfig::default().with_pull(Pull::Up);
    let mut interrupt_pin = Input::new(peripherals.GPIO36, input_config);



    loop {
        if interrupt_pin.is_low() {
            match co2_sensor.read_data() {
                Ok((eco2, tvoc)) => info!("eCO2: {} ppm\tTVOC: {}ppb", eco2, tvoc),
                Err(e) => error!("Failed to read data: {:?}", e),
            }
        }
    }

}
