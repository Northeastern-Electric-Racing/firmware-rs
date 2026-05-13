#![no_std]

// declare all files in this project except main
pub mod can_handler;
pub mod controllers;
pub mod readers;

// include below any shared types or structs across the project
// make sure to define these in a workspace crate if they are shared across multiple projects

// dont import anything in a lib.rs file, instead use fully resolved definitions
pub type SharedI2c3 = embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    embassy_stm32::i2c::I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::Master>,
>;
#[derive(Clone, defmt::Format)]
pub enum DeviceLocation {
    FrontLeft,
    BackLeft,
    BackRight,
    FrontRight,
}

impl From<(bool, bool, bool)> for DeviceLocation {
    fn from(value: (bool, bool, bool)) -> Self {
        if value.0 && value.1 {
            DeviceLocation::FrontLeft
        } else if value.0 && !value.1 {
            DeviceLocation::FrontRight
        } else if !value.1 && value.2 {
            DeviceLocation::BackLeft
        } else if !value.0 && !value.2 {
            DeviceLocation::BackRight
        } else {
            DeviceLocation::FrontLeft
        }
    }
}

impl DeviceLocation {
    fn get_can_id(&self, base_id: &embassy_stm32::can::Id) -> embassy_stm32::can::StandardId {
        let id = match base_id {
            embassy_stm32::can::Id::Standard(id) => id,
            embassy_stm32::can::Id::Extended(id) => &id.standard_id(),
        };
        defmt::unwrap!(embassy_stm32::can::StandardId::new(match self {
            DeviceLocation::FrontLeft => id.as_raw(),
            DeviceLocation::BackLeft => id.as_raw() + 0x40,
            DeviceLocation::BackRight => id.as_raw() + 0x60,
            DeviceLocation::FrontRight => id.as_raw() + 0x20,
        }))
    }
}
