#![no_std]
//! Driver for HDC2021 temperature/humidity sensors

// NOTE: All drivers shoould only ever use embedded hal traits for standardization
// this makes them eternally portable yet still easy to use from embassy
// therefore, no import should EVER mention



use embedded_hal_async::{delay::DelayNs, i2c::I2c};




#[derive(Debug, Clone, Copy)]
pub enum Address {
    /// the ADDR is recommended not to be left 
    /// floating if the device is to be used in noisy environment.
    Low = 0x40,
    High = 0x41,
}

pub const MANUFACTURER_ID: u16 = 0x5449; 
const DRDY_POLL_ATTEMPTS: u8 = 10;
const DRDY_POLL_INTERVAL_MS: u32 = 2; // 2ms,
 
/// register addresses
mod register {
    pub const TEMP_LOW: u8 = 0x00;
    // pub const TEMP_HIGH: u8 = 0x01;
    // pub const HUMIDITY_LOW: u8 = 0x02;
    // pub const HUMIDITY_HIGH: u8 = 0x03;
    pub const DRDY_BIT: u8 = 1 << 7; // bit 7 of the status register indicates data ready
    pub const DRDY_STATUS: u8 = 0x04;
    pub const MANUFACTURER_ID_LOW: u8 = 0xFC;
    // pub const MANUFACTURER_ID_HIGH: u8 = 0xFD;
    pub const HEATER_BIT: u8 = 1 << 3; // bit 3 of the configuration register enables the heater
    pub const DEVICE_ID_LOW: u8 = 0xFE;
    pub const DEVICE_CONFIG: u8 = 0x0E;

    pub const MEASURE_TRIG: u8 = 0x01;
    pub const MEASURE_CONFIG: u8 = 0x0F; // 00: Humidity + Temperature
}

/// Errors
#[derive(Debug, defmt::Format)]
pub enum Error<E> {

    // I2C bus error
    I2c(E),
    Timeout,
}

/// Raw temperature and humidity measurement
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct Measurement {
    // Temperature in degrees celsius
    pub temperature: f32, 
    // Relative humidity as a percentage (0-100)
    pub humidity: f32,
}

/// HDC2021 on I2C bus
#[derive(Debug, Clone)]
pub struct Hdc2021<I2C> {
    i2c: I2C,
    address: Address,
}


impl<I2C, E> Hdc2021<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Creates a new driver.
    pub const fn new(i2c: I2C, address: Address) -> Self {
        Self { i2c, address }
    }



/// Read the manufacture ID of the sensor. This can be used to verify that the sensor is present and responding correctly.
/// the address register is incremented automatically by the sensor, so we can read both bytes in one transaction.

/// The manufacturer ID is a 16-bit value, with the low byte at register 0xFC and the high byte at register 0xFD. The expected value for the HDC2021 is 0x5449 (ASCII "TI").
    pub async fn manufacturer_id(&mut self) -> Result<u16, Error<E>> {
        self.read_u16(register::MANUFACTURER_ID_LOW).await
    }

// When a measurement is
// triggered, the HDC2021 switches to measurement mode that converts temperature or humidity values from
// integrated sensors through an internal ADC and stores the information in their respective data registers. 


// Measurement trigger:
// 0: No action
// 1: Start measurement
// Setting this bit to 1 to start a single measurement in one-shot
// mode or continuous measurements in continuous conversion
// mode. This bit self-clears to 0 once the measurement starts

/// The measurement is complete when the DRDY bit in the status register is set to 1. The maximum time for a
    pub async fn device_id(&mut self) -> Result<u16, Error<E>> {
        self.read_u16(register::DEVICE_ID_LOW).await
    }

    /// Trigger a measurement and wait for it to complete, then read the result. This function will block until the measurement is complete or a timeout occurs.
    pub async fn measure(&mut self, delay: &mut impl DelayNs) -> Result<Measurement, Error<E>> {
        self.trigger_measurement().await?;
       /// Wait for the measurement to complete by polling the DRDY bit in the status register. The maximum time for a measurement is 6.5ms, so we will poll for a maximum of 10 attempts with a 2ms delay between each attempt.
        for _ in 0.. DRDY_POLL_ATTEMPTS {
            delay.delay_ms(DRDY_POLL_INTERVAL_MS).await;
            if self.read_u8(register::DRDY_STATUS).await? & register::DRDY_BIT != 0 {
                return self.read_measurement().await;
            }
        }
        Err(Error::Timeout) 
    }



/// Enable or disable the internal heater. The heater can be used to prevent condensation on the sensor in high humidity environments. It is recommended to use the heater only when necessary, as it will increase power consumption and may affect measurement accuracy.
    pub async fn toggle_heater(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut config = self.read_u8(register::DEVICE_CONFIG).await?;
        if enable {
            config |= register::HEATER_BIT;
        } else {
            config &= !register::HEATER_BIT;
        }
        self.write_u8(register::DEVICE_CONFIG, config).await
    }

    async fn trigger_measurement(&mut self) -> Result<(), Error<E>> {
        self.write_u8(register::MEASURE_CONFIG, register::MEASURE_TRIG).await
    }


/// Read the temperature and humidity measurement from the sensor. This function assumes that a measurement has already been triggered and completed.
    pub async fn read_measurement(&mut self) -> Result<Measurement, Error<E>> {
        let mut buf = [0u8; 4];
        self.i2c
            .write_read(self.address as u8, &[register::TEMP_LOW], &mut buf)
            .await
            .map_err(Error::I2c)?;
        
        Ok(Measurement {
            temperature: convert_temperature(u16::from_le_bytes([buf[0], buf[1]])),
            humidity: convert_humidity(u16::from_le_bytes([buf[2], buf[3]])),
        })
    }

    /// Read a single byte from the given register
    async fn read_u8(&mut self, reg: u8) -> Result<u8, Error<E>> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.address as u8, &[reg], &mut buf)
            .await
            .map_err(Error::I2c)?;
        Ok(buf[0])
    }

    /// Read a 16-bit value from the given register (little-endian)
    async fn read_u16(&mut self, reg_low: u8) -> Result<u16, Error<E>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address as u8, &[reg_low], &mut buf)
            .await
            .map_err(Error::I2c)?;
        Ok(u16::from_le_bytes(buf))
    }


    /// Write a single byte to the given register
    async fn write_u8(&mut self, reg: u8, value: u8) -> Result<(), Error<E>> {
        self.i2c
            .write(self.address as u8, &[reg, value])
            .await
            .map_err(Error::I2c)
    }
    
}
const fn convert_temperature(raw: u16) -> f32 {
    // (temp/2^8)*165.0-40.0 -> 8-bits, for 16-bits, use 2^16 instead of 2^8
    (raw as f32 / 65536.0) * 165.0 - 40.0
}

const fn convert_humidity(raw: u16) -> f32 {
    // humidity*(100/2^8) -> 8-bits, for 16-bits, use 2^16 instead of 2^8
    (raw as f32 / 65536.0) * 100.0 
}




