#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use crate::sensor::{CO2Sensor};

// use esp_println::print;

use esp_hal::clock::CpuClock;
// use esp_hal::delay::Delay;
// use esp_hal::peripherals::{self, GPIO, Peripherals};
// use esp_hal::gpio::{Event, Input, InputConfig, Level, Output, OutputConfig, Pull, Io};
use esp_hal::gpio::{Input, InputConfig, Pull};
// use esp_hal::i2c::master::{Config as OtherConfig, I2c, Operation};
use esp_hal::i2c::master::{Config as OtherConfig};
// use esp_hal::{handler, main};
// use esp_hal::{main, peripherals};
use esp_hal::{main};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use {esp_backtrace as _, esp_println as _};
// use critical_section::Mutex;
// use core::cell::RefCell;
// use esp_println::print;
// use esp_println::println;

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
    let config_i2c = OtherConfig::default().with_frequency(Rate::from_khz(100));
    let peripherals = esp_hal::init(config);
    
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    let mut co2_sensor: CO2Sensor = CO2Sensor::new();
    co2_sensor.find_dev();
    co2_sensor.read_status();
    co2_sensor.config_i2c(sensor::MeasurementDriveMode::Mode11S, true);
    co2_sensor.read_meas_mode();

    // Initialize bus
    // let sda = peripherals.GPIO21;
    // let scl = peripherals.GPIO22;
    // let wake_pin = Output::new(peripherals.GPIO23, Level::Low, OutputConfig::default());
    // let mut i2c = I2c::new(peripherals.I2C0, OtherConfig::default())
    //     .unwrap()
    //     .with_scl(scl)
    //     .with_sda(sda);
    
    // // Delay for waiting operations
    // let mut delay = Delay::new();

    // // Find address
    // let mut dev_addr = 0;
    // let mut read_buffer: [u8; 8] = [0u8; 8]; // Biggest read is 8 byte
    // let mut dev_found = false;
    // while dev_addr < 128_u8 {
    //     if i2c.read(dev_addr, &mut read_buffer).is_ok() {
    //         println!("Device found: 0x{:02x}", dev_addr);
    //         dev_found = true;
    //         break;
    //     };
    //     dev_addr += 1;
    // }

    // if dev_found == false {
    //     println!("No device found!!");
    //     loop {}
    // }

    // // Status
    // const FW_MODE: u8 = 0b10000000;
    // const APP_VALID: u8 = 0b00010000;
    // const DATA_READY: u8 = 0b00001000;
    // const ERROR: u8 = 0b00000001; // If error read E0 to get code

    // // Bootloader
    // let _app_verify = [0xF3];
    // let _app_start = [0xF4];

    // // Registers
    // let _status_reg = [0x00];
    // let _meas_mode_reg = [0x01];
    // let _alg_result_reg = [0x02];
    // let _raw_data_reg = [0x03];
    // let _ntc_reg = [0x06];
    // let _hw_id_reg = [0x20]; // Used to test if data from chip is valid. Should return 81
    // let _error_id_reg = [0xE0];

    // // Read initial status
    // let mut status = i2c.write_read(dev_addr, &_status_reg, &mut read_buffer);
    // println!("Status: {:08b}", read_buffer[0]);

    // if read_buffer[0] & FW_MODE != 0 {
    //     println!("\t-> Firmware is in application mode. CCS811 is ready to take ADC measurements");
    // } else {
    //     println!("\t-> Firmware is in boot mode, this allows new firmware to be loaded");
    //     if read_buffer[0] & APP_VALID != 0 {
    //         println!("\t-> Valid application firmware loaded");

    //         // Run app start.
    //         status = i2c.write(dev_addr, &_app_start);
    //         delay.delay_millis(50);
    //         status = i2c.write_read(dev_addr, &_status_reg, &mut read_buffer);
    //         println!("Status: {:08b}", read_buffer[0]);
    //         if read_buffer[0] & FW_MODE != 0 {
    //             println!(
    //                 "\t-> Firmware is in application mode. CCS811 is ready to take ADC measurements"
    //             );
    //         } else {
    //             println!("\t-> Firmware is in boot mode, this allows new firmware to be loaded");
    //         }
    //     } else {
    //         println!("\t-> No application firmware loaded");
    //     }
    // }

    // if read_buffer[0] & ERROR != 0 {
    //     status = i2c.write_read(dev_addr, &_error_id_reg, &mut read_buffer);
    //     print_error(read_buffer[0]);
    // }

    // // Measure mode setup
    // const MEAS_MODE_DRIVE_MODE_1: u8 = 0b00010000; // Measurement every 1 second
    // const MEAS_MODE_DRIVE_MODE_2: u8 = 0b00100000; // Measurement every 10 second
    // const MEAS_MODE_INTERRUPT: u8 = 0b00001000; // Interrupt on ready 
    // // let meas_mode = [MEAS_MODE_DRIVE_MODE_2 | MEAS_MODE_INTERRUPT];
    // // let meas_mode = [MEAS_MODE_DRIVE_MODE_1];
    // let meas_mode = MEAS_MODE_DRIVE_MODE_1 | MEAS_MODE_INTERRUPT;

    // // Read measurement mode
    // status = i2c.write_read(dev_addr, &_meas_mode_reg, &mut read_buffer);
    // println!("Meas mode: {:08b}", read_buffer[0]);

    // if read_buffer[0] != meas_mode {
    //     status = i2c.transaction(
    //         dev_addr,
    //         &mut [
    //             Operation::Write(&_meas_mode_reg),
    //             Operation::Write(&[meas_mode]),
    //         ],
    //     );

    //     status = i2c.write_read(dev_addr, &_meas_mode_reg, &mut read_buffer);
    //     if read_buffer[0] == meas_mode {
    //         println!("New Meas mode: {:08b}", read_buffer[0]);
    //     } else {
    //         println!("Failed to update")
    //     }
    // }
    // if read_buffer[0] & MEAS_MODE_DRIVE_MODE_1 != 0 {
    //     println!("\t-> Constant power mode, IAQ measurement every second")
    // } else if read_buffer[0] & MEAS_MODE_DRIVE_MODE_2 != 0 {
    //     println!("\t-> Pulse heating mode IAQ measurement every 10 seconds")
    // }
    // if read_buffer[0] & MEAS_MODE_INTERRUPT != 0 {
    //     println!("\t-> Interrupt generation is enabled")
    // } else {
    //     println!("\t-> Interrupt generation is disabled")
    // }

    let input_config = InputConfig::default().with_pull(Pull::Up);
    let mut interrupt_pin = Input::new(peripherals.GPIO36, input_config);



    // Read NTC to calculate temperature - need info on thermistor
    // let mut ntc_buffer: [u8;4];
    // status = i2c.write_read(dev_addr,&_ntc_reg,&mut ntc_buffer);
    // let v_r_ref = ((ntc_buffer[0]) & 0xFF) << 8 | (ntc_buffer[1]) & 0xFF;
    // let v_r_ntc = ((ntc_buffer[2]) & 0xFF) << 8 | (ntc_buffer[3]) & 0xFF;
    // let mut r_ntc = v_r_ntc*r_ref/v_r_ref;

    // critical_section::with(|cs| {
    // // Here we are listening for a low level to demonstrate
    // // that you need to stop listening for level interrupts,
    // // but usually you'd probably use `FallingEdge`.
    // interrupt_pin.listen(Event::FallingEdge);
    // BUTTON.borrow_ref_mut(cs).replace(interrupt_pin);
    // });

    // delay.delay_millis(50);
    loop {
        // print!("Wait");
        // interrupt_pin.wait_for_low();
        // println!(" done");
        if interrupt_pin.is_low() {
            // status = i2c.write_read(dev_addr, &_alg_result_reg, &mut read_buffer);
            // let eco2 = ((read_buffer[0]) as u16 & 0xFF) << 8 | (read_buffer[1]) as u16 & 0xFF;
            // let tvoc = ((read_buffer[2]) as u16 & 0xFF) << 8 | (read_buffer[3]) as u16 & 0xFF;
            // println!("eCO2: {:#?} ppm\tTVOC: {:#?}", eco2, tvoc);
            co2_sensor.read_data();
            
        }
    }

}

// fn print_error(buf: u8) {
//     print!("\t-> Error: ");
//     match buf {
//         1 => println!("WRITE_REG_INVALID"),
//         2 => println!("READ_REG_INVALID"),
//         4 => println!("MEASMODE_INVALID"),
//         8 => println!("MAX_RESISTANCE"),
//         16 => println!("HEATER_FAULT"),
//         32 => println!("HEATER_SUPPLY"),
//         _ => println!("Error {:08b}", buf),
//     }
// }


