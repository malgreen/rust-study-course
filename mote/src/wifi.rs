use core::{
    error::{self, Error},
    fmt,
};

use alloc::boxed::Box;
use defmt::Format;
use esp_hal::peripherals::{Peripherals, WIFI};
use esp_radio::{InitializationError, wifi::WifiError};

//
pub fn init(wifi_peripheral: WIFI) -> Result<&'static str, EspWifiError> {
    let radio_controller = esp_radio::init()?;

    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(&radio_controller, wifi_peripheral, Default::default())?;

    return Ok("asd");
}

// error stuff
#[derive(Format)]
pub enum EspWifiError {
    WifiError(WifiError),
    InitializationError(InitializationError),
}
// we need to implement From<T> because it allows use to use the `?` operator,
impl From<WifiError> for EspWifiError {
    fn from(value: WifiError) -> Self {
        EspWifiError::WifiError(value)
    }
}

impl From<InitializationError> for EspWifiError {
    fn from(value: InitializationError) -> Self {
        EspWifiError::InitializationError(value)
    }
}
