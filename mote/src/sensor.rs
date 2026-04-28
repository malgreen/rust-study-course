use core::fmt::Error;

use alloc::str::pattern::SearchStep;
// use esp_hal::clock::CpuClock;
// use esp_hal::gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::Blocking;
use esp_hal::i2c::master::{Config as i2cConfig, I2c, Operation};
// use esp_hal::peripherals::{GPIO, Peripherals};

use defmt::{debug, error, info, warn};
use esp_hal::peripherals::{GPIO21, GPIO22, I2C0};

// Bootloader
const _APP_VERIFY: [u8; 1] = [0xF3];
const _APP_START: [u8; 1] = [0xF4];

// Registers
const _STATUS_REG: [u8; 1] = [0x00];
const _MEAS_MODE_REG: [u8; 1] = [0x01];
const _ALG_RESULT_REG: [u8; 1] = [0x02];
const _RAW_DATA_REG: [u8; 1] = [0x03];
const _NTC_REG: [u8; 1] = [0x06];
const _HW_ID_REG: [u8; 1] = [0x20]; // Used to test if data from chip is valid. Should return 81
const _ERRPR_ID_REG: [u8; 1] = [0xE0];

// Status
#[repr(u8)]
pub enum Status {
    FwMode = 0b10000000,
    AppValid = 0b00010000,
    DataReady = 0b00001000,
    Error = 0b00000001, // If error read E0 to get code
}
#[repr(u8)]
pub enum MeasurementDriveMode {
    Mode0Idle = 0b00000000,  // Idle
    Mode11S = 0b00010000,    // Measurement every 1 second
    Mode210S = 0b00100000,   // Measurement every 10 second
    Mode360S = 0b00110000,   // Measurement every 60 second
    Mode4250MS = 0b01000000, // // Measurement every 250 ms (Only raw data)

    IRQEnable = 0b00001000, // Enable/disable interrupt
}

enum Errors {
    NoDeviveFound,
    ReadingFault,
    DataNotReady
}

pub struct CO2Sensor<'a> {
    i2c: I2c<'a, Blocking>,
    dev_addr: u8,
    interrupt_active: bool,
}

impl<'a> CO2Sensor<'a> {
    pub fn new(_i2c: I2C0<'a>, scl: GPIO22<'a>, sda: GPIO21<'a>) -> Self {
        let i2c = I2c::new(_i2c, i2cConfig::default())
            .unwrap()
            .with_scl(scl) // Does it need .upwrap()?
            .with_sda(sda);

        Self {
            i2c,
            dev_addr: 0,
            interrupt_active: false,
        }
    }

    pub fn find_dev(&mut self) -> Result<bool, Errors> // Return some correct type??
    {
        let addr_space = 128_u8;
        let mut current_addr = 0;
        let mut respons: [u8; 1] = [0u8; 1];
        // let mut success = false;

        while current_addr < addr_space {
            match self.i2c.read(current_addr, &mut respons) 
            {
                Ok(()) => {
                    info!("Device located at 0x{:02x}", current_addr);
                    self.dev_addr = current_addr;

                    return Ok((true)); // Early stopping
                },  
                Err(_) => {}
            }
            current_addr += 1;
        }
        error!("No device located");
        return Err(Errors::NoDeviveFound);
        // while current_addr < addr_space {
        //     if self.i2c.read(current_addr, &mut respons).is_ok() {
        //         info!("Device located at 0x{:02x}", current_addr);
        //         success = true;
        //         self.dev_addr = current_addr;

        //         return Ok((true)); // Early stopping
        //     }
        //     current_addr += 1;
        // }

        // Failed to locate device -> Report this somehow??
    }

    // Should return sometihng about the status??
    pub fn read_status(&mut self)  {
        let mut status: [u8; 1] = [0u8; 1];

        self.i2c
            .write_read(self.dev_addr, &_STATUS_REG, &mut status);

        if status[0] & Status::FwMode as u8 != 0 {
            info!("\t-> Firmware is in application mode. CCS811 is ready to take ADC measurements");
        } else {
            info!("\t-> Firmware is in boot mode, this allows new firmware to be loaded");
            if status[0] & Status::AppValid as u8 != 0 {
                info!("\t-> Valid application firmware loaded");

                // Run app start.
                self.app_start();
            } else {
                error!("\t-> No application firmware loaded");
                panic!("Shit fuck");
            }
        }
        if status[0] & Status::Error as u8 != 0 {
            self.i2c
                .write_read(self.dev_addr, &_ERRPR_ID_REG, &mut status);
            print_error(status[0]);
        }
    }

    pub fn app_start(&mut self) {
        self.i2c.write(self.dev_addr, &_APP_START);

        // Confirm??
        // delay.delay_millis(50);

        self.read_status();
    }

    pub fn config_i2c(&mut self, mode: MeasurementDriveMode, interrupt: bool) {
        let mut config_read: [u8; 1] = [0u8; 1];
        self.i2c
            .write_read(self.dev_addr, &_MEAS_MODE_REG, &mut config_read);

        let mut configuration: u8 = mode as u8;

        // Check for active pin configuration??

        if interrupt {
            configuration |= MeasurementDriveMode::IRQEnable as u8;
        }
        debug!("Config: {:08b}", configuration);

        if config_read[0] & configuration != 0 {
            self.i2c.transaction(
                self.dev_addr,
                &mut [
                    Operation::Write(&_MEAS_MODE_REG),
                    Operation::Write(&[configuration as u8]),
                ],
            );
        }

        self.i2c
            .write_read(self.dev_addr, &_MEAS_MODE_REG, &mut config_read);
        if config_read[0] & configuration == 0 {
            error!("Failed to configure");
        }
    }

    // Handler function for reading data.
    pub fn read_data(&mut self) {
        let mut sensor_data: [u8; 8] = [0u8; 8];

        self.i2c
            .write_read(self.dev_addr, &_ALG_RESULT_REG, &mut sensor_data);

        if sensor_data[4] & Status::Error != 0 {
            print_error(sensor_data[4]);
            // Return error?
        }
        if sensor_data[4] & Status::DataReady != 0 {
            debug!("Data to ready yet?");
            // Return early
        }

        let eco2 = ((sensor_data[0]) as u16 & 0xFF) << 8 | (sensor_data[1]) as u16 & 0xFF;
        let tvoc = ((sensor_data[2]) as u16 & 0xFF) << 8 | (sensor_data[3]) as u16 & 0xFF;

        info!("eCO2: {:#?} ppm\tTVOC: {:#?}", eco2, tvoc);
    }

    pub fn read_meas_mode(&mut self) {
        // Read measurement mode
        let mut status: [u8; 1] = [0u8; 1];

        self.i2c
            .write_read(self.dev_addr, &_MEAS_MODE_REG, &mut status);
        info!("Meas mode: {:08b}", status[0]);

        if status[0] & MeasurementDriveMode::Mode0Idle as u8 != 0 {
            info!("\t-> Idle")
        } else if status[0] & MeasurementDriveMode::Mode11S as u8 != 0 {
            info!("\t-> Constant power mode, IAQ measurement every second")
        } else if status[0] & MeasurementDriveMode::Mode210S as u8 != 0 {
            info!("\t-> Pulse heating mode IAQ measurement every 10 seconds")
        } else if status[0] & MeasurementDriveMode::Mode360S as u8 != 0 {
            info!("\t-> Low power pulse heating mode IAQ measurement every 60 seconds")
        }
        if status[0] & MeasurementDriveMode::IRQEnable as u8 != 0 {
            info!("\t-> Interrupt generation is enabled")
        } else {
            warn!("\t-> Interrupt generation is disabled")
        }
    }
}

fn print_error(id: u8) {
    match id {
        1 => error!("WRITE_REG_INVALID"),
        2 => error!("READ_REG_INVALID"),
        4 => error!("MEASMODE_INVALID"),
        8 => error!("MAX_RESISTANCE"),
        16 => error!("HEATER_FAULT"),
        32 => error!("HEATER_SUPPLY"),
        _ => error!("Error {:08b}", id),
    }
}
