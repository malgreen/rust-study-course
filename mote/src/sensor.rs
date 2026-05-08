use esp_hal::Blocking;
use esp_hal::i2c::master::{Config as i2cConfig, I2c, Operation};
use defmt::{debug, error, info, warn, Format};
use esp_hal::peripherals::{GPIO21, GPIO22, I2C0};
use crate::Rate;
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
const _ERROR_ID_REG: [u8; 1] = [0xE0];

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
#[derive(defmt::Format)]
pub enum Errors {
    NoDeviceFound,
    ReadingFault,
    DataNotReady,
    NoApplicationFound,
    WriteRegInvalid,
    ReadRegInvalid,
    MeasmodeInvalid,
    MaxResistance,
    HeaterFault,
    HeaterSupply,
    ConfigError,
    MultipleErrors,
}

pub struct CO2Sensor<'a> {
    i2c: I2c<'a, Blocking>,
    dev_addr: u8,
    interrupt_active: bool,
}

impl<'a> CO2Sensor<'a> {
    /// Construct a CO2 sensor 
    pub fn new(_i2c: I2C0<'a>, scl: GPIO22<'a>, sda: GPIO21<'a>) -> CO2Sensor<'a> {
        let mut i2c = I2c::new(_i2c, i2cConfig::default())
            .unwrap()
            .with_scl(scl) // Does it need .upwrap()?
            .with_sda(sda);
    
        i2c.apply_config(&i2cConfig::default().with_frequency(Rate::from_khz(100)));

        Self {
            i2c,
            dev_addr: 0,
            interrupt_active: false,
        }
    }

    /// Find a i2c device on the bus. Check for response
    /// 
    /// Datasheet highlights 0x5A and 0x5B.
    /// 
    /// Returns current address -> The address which was confirmed or returns error
    pub fn find_dev(&mut self) -> Result<u8, Errors> // Return some correct type??
    {
        let addr_space = 91_u8; // All: 128_u8 
        let mut current_addr = 90_u8; // All: 0, Skip early addresses and only search addresses of interest
        let mut respons: [u8; 1] = [0u8; 1];

        while current_addr < addr_space {
            match self.i2c.read(current_addr, &mut respons) {
                Ok(()) => {
                    self.dev_addr = current_addr;

                    return Ok(current_addr); // Early stopping
                }
                Err(_) => {}
            }
            current_addr += 1;
        }
        error!("No device located");
        return Err(Errors::NoDeviceFound);
    }

    /// Reads status register. On error the error register is also read (get_error)
    /// 
    /// Will start the application if firmware is loaded but not running.
    /// 
    /// Program does not support transfering the firmware.
    pub fn read_status(&mut self) -> Result<(), Errors> {
        let mut status: [u8; 1] = [0u8; 1];

        if let Err(_) = self
            .i2c
            .write_read(self.dev_addr, &_STATUS_REG, &mut status)
        {
            return Err(Errors::ReadingFault);
        }

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
                return Err(Errors::NoApplicationFound);
            }
        }
        if status[0] & Status::Error as u8 != 0 {
            return self.get_error();
        }

        Ok(())
    }

    /// Starts the application if the chip is in bootloader mode.
    /// 
    /// Requires that a firmware is loaded already.
    pub fn app_start(&mut self) -> Result<(), Errors> {
        if let Err(_) = self.i2c.write(self.dev_addr, &_APP_START) {
            return Err(Errors::ReadingFault);
        }
        self.read_status()
    }

    /// Configuration of the chip in terms of reading interval and wether its runs with interrupt.
    pub fn meas_setup(
        &mut self,
        mode: MeasurementDriveMode,
        interrupt: bool,
    ) -> Result<(), Errors> {
        let mut config_read: [u8; 1] = [0u8; 1];
        if let Err(_) = self
            .i2c
            .write_read(self.dev_addr, &_MEAS_MODE_REG, &mut config_read)
        {
            return Err(Errors::ReadingFault);
        }

        let mut configuration: u8 = mode as u8;

        // Check for active pin configuration??

        if interrupt {
            configuration |= MeasurementDriveMode::IRQEnable as u8;
        }
        debug!("Config: {:08b}", configuration);

        if config_read[0] != configuration {
            if let Err(_) = self.i2c.transaction(
                self.dev_addr,
                &mut [
                    Operation::Write(&_MEAS_MODE_REG),
                    Operation::Write(&[configuration as u8]),
                ],
            ) {
                return Err(Errors::WriteRegInvalid);
            }
        }

        if let Err(_) = self
            .i2c
            .write_read(self.dev_addr, &_MEAS_MODE_REG, &mut config_read)
        {
            return Err(Errors::ReadingFault);
        }
        if config_read[0] & configuration == 0 {
            error!("Failed to configure");
            return Err(Errors::ConfigError);
        }
        Ok(())
    }

    /// Read sensor data from chip
    /// 
    /// Activated on interrupt or checking wether the data is ready.
    pub fn read_data(&mut self) -> Result<(u16, u16), Errors> {
        let mut sensor_data: [u8; 8] = [0u8; 8];

        if let Err(_) = self
            .i2c
            .write_read(self.dev_addr, &_ALG_RESULT_REG, &mut sensor_data)
        {
            return Err(Errors::ReadingFault);
        }

        if sensor_data[4] & (Status::Error as u8) != 0 {
            if let Err(e) = self.get_error() {
                return Err(e);
            }
        }
        if sensor_data[4] & (Status::DataReady as u8) == 0 {
            debug!("Data not ready yet?");

            return Err(Errors::DataNotReady);
        }

        let eco2 = ((sensor_data[0]) as u16 & 0xFF) << 8 | (sensor_data[1]) as u16 & 0xFF;
        let tvoc = ((sensor_data[2]) as u16 & 0xFF) << 8 | (sensor_data[3]) as u16 & 0xFF;

        // Some reads in the beginning says 0. Skip those
        if eco2 != 0
        {
            return Ok((eco2, tvoc));
        }
        return Err(Errors::DataNotReady);
    }

    pub fn read_meas_mode(&mut self) -> Result<(), Errors> {
        // Read measurement mode
        let mut status: [u8; 1] = [0u8; 1];

        if let Err(_) = self
            .i2c
            .write_read(self.dev_addr, &_MEAS_MODE_REG, &mut status)
        {
            return Err(Errors::ReadingFault);
        }
        debug!("Meas mode: {:08b}", status[0]);

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
        Ok(())
    }

    fn get_error(&mut self) -> Result<(), Errors> {
        let mut error_id: [u8; 1] = [0u8;1];

        if let Err(_) = self
            .i2c
            .write_read(self.dev_addr, &_ERROR_ID_REG, &mut error_id)
        {
            return Err(Errors::ReadingFault);
        }

        match error_id[0] {
            // Possible wrong since errors might occur at the same time.
            1 => return Err(Errors::WriteRegInvalid),
            2 => return Err(Errors::ReadRegInvalid),
            4 => return Err(Errors::MeasmodeInvalid),
            8 => return Err(Errors::MaxResistance),
            16 => return Err(Errors::HeaterFault),
            32 => return Err(Errors::HeaterSupply),
            _ => {
                error!("Error {:08b}", error_id);
                Err(Errors::MultipleErrors)
            }
        }
    }
}
