#![no_std]
//! Driver for LSM6DSV IMU

// NOTE: All drivers shoould only ever use embedded hal traits for standardization
// this makes them eternally portable yet still easy to use from embassy
// therefore, no import should EVER mention

// found from c files for this driver (imp for sensor fusion: Embedded sensor fusion low-power algorithm)

use embedded_hal_async::{delay::DelayNs, spi::SpiDevice};
use lsm6dsv_rs::asynchronous::driver::{self as st, Lsm6dsv as StLsm6dsv};
use lsm6dsv_rs::asynchronous::register::main::{GyFullScale, Odr, Reset, XlFullScale};
use st_mems_bus::asynchronous::spi::SpiBus;

/// wait after a software reset before talking to the sensor again, Turn-on time - gyroscope - 30ms from datasheet
const RESET_WAIT_MS: u32 = 30;

/// Errors
#[derive(Debug, defmt::Format)]
pub enum Error {
    /// SPI bus error
    Bus,
    /// The WHO_AM_I register didn't match the LSM6DSV. Usually wiring, SPI mode, or chip select.
    WrongId(u8),
}

/// accelerometer full scale range (linear acceleration measurement range)
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum AccelRange {
    G2,
    G4,
    G8,
    G16,
}

/// gyroscope full scale range (125/250/500/1000/2000/4000 dps full scale)
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum GyroRange {
    Dps125,
    Dps250,
    Dps500,
    Dps1000,
    Dps2000,
    Dps4000,
}

/// Output data rate, used for both the accelerometer and the gyroscope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum DataRate {
    Hz1_875,
    Hz7_5,
    Hz15,
    Hz30,
    Hz60,
    Hz120,
    Hz240,
    Hz480,
    Hz960,
    Hz1920,
    Hz3840,
    Hz7680,
}

/// config
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct Config {
    pub accel_range: AccelRange,
    pub gyro_range: GyroRange,
    pub data_rate: DataRate,
}

impl Default for Config {
    /// same as C: 125dps, 2g, 15Hz??
    fn default() -> Self {
        Self {
            accel_range: AccelRange::G2,
            gyro_range: GyroRange::Dps125, // Dps2000
            data_rate: DataRate::Hz15,     // Hz120
        }
    }
}

/// A value on each of the three axes
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct Axes {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// one IMU reading
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct ImuReading {
    /// Acceleration (in milli-g)
    pub accel_mg: Axes,
    /// Angular rate (in millidegrees per second)
    pub gyro_mdps: Axes,
}

/// LSM6DSV on an SPI bus.
pub struct Lsm6dsv<SPI: SpiDevice, D: DelayNs> {
    // there's also a third parameter bank-state parameter register::MainBank
    sensor: StLsm6dsv<SpiBus<SPI>, D>,
    // Remembered so raw readings are converted using the right scale
    accel_range: AccelRange,
    gyro_range: GyroRange,
}

impl<SPI: SpiDevice, D: DelayNs> Lsm6dsv<SPI, D> {
    /// to create a new driver, must include the chip select (i.e., an `ExclusiveDevice`, should be in main.rs)
    pub fn new(spi: SPI, delay: D) -> Self {
        let defaults = Config::default();
        Self {
            sensor: StLsm6dsv::new_spi(spi, delay),
            accel_range: defaults.accel_range,
            gyro_range: defaults.gyro_range,
        }
    }

    /// Sets the sensor up like `init_imu()` in c: check the ID, reset, block data
    /// update on, then apply `config` (ranges and data rate)
    pub async fn init(&mut self) -> Result<(), Error> {
        let id = self.sensor.device_id_get().await.map_err(|_| Error::Bus)?;
        if id != st::ID {
            return Err(Error::WrongId(id));
        }

        self.sensor
            .reset_set(Reset::GlobalRst)
            .await
            .map_err(|_| Error::Bus)?;
        self.sensor.tim.delay_ms(RESET_WAIT_MS).await;

        // Output registers don't update until both halves of a reading have been read.
        self.sensor
            .block_data_update_set(1)
            .await
            .map_err(|_| Error::Bus)?;

        self.set_accel_range(config.accel_range).await?;
        self.set_gyro_range(config.gyro_range).await?;
        self.set_data_rate(config.data_rate).await?;

        Ok(())
    }

    /// Sets the accelerometer full-scale range
    pub async fn set_accel_range(&mut self, range: AccelRange) -> Result<(), Error> {
        // these variant names were in lsm6dsv16x-rs (XlFullScale)
        let fs = match range {
            AccelRange::G2 => XlFullScale::_2g,
            AccelRange::G4 => XlFullScale::_4g,
            AccelRange::G8 => XlFullScale::_8g,
            AccelRange::G16 => XlFullScale::_16g,
        };
        self.sensor
            .xl_full_scale_set(fs)
            .await
            .map_err(|_| Error::Bus)?;
        self.accel_range = range;
        Ok(())
    }

    /// Sets the gyroscope full-scale range
    pub async fn set_gyro_range(&mut self, range: GyroRange) -> Result<(), Error> {
        // these variant names were in lsm6dsv16x-rs (GyFullScale)
        let fs = match range {
            GyroRange::Dps125 => GyFullScale::_125dps,
            GyroRange::Dps250 => GyFullScale::_250dps,
            GyroRange::Dps500 => GyFullScale::_500dps,
            GyroRange::Dps1000 => GyFullScale::_1000dps,
            GyroRange::Dps2000 => GyFullScale::_2000dps,
            GyroRange::Dps4000 => GyFullScale::_4000dps,
        };
        self.sensor
            .gy_full_scale_set(fs)
            .await
            .map_err(|_| Error::Bus)?;
        self.gyro_range = range;
        Ok(())
    }

    /// Sets the output data rate of both the accelerometer and the gyroscope
    pub async fn set_data_rate(&mut self, rate: DataRate) -> Result<(), Error> {
        // these variant names were in lsm6dsv16x-rs (Odr)
        let odr = match range {
            DataRate::Hz1_875 => Odr::_1_875hz,
            DataRate::Hz7_5 => Odr::_7_5hz,
            DataRate::Hz15 => Odr::_15hz,
            DataRate::Hz30 => Odr::_30hz,
            DataRate::Hz60 => Odr::_60hz,
            DataRate::Hz120 => Odr::_120hz,
            DataRate::Hz240 => Odr::_240hz,
            DataRate::Hz480 => Odr::_480hz,
            DataRate::Hz960 => Odr::_960hz,
            DataRate::Hz1920 => Odr::_1920hz,
            DataRate::Hz3840 => Odr::_3840hz,
            DataRate::Hz7680 => Odr::_7680hz,
        };
        self.sensor
            .xl_data_rate_set(odr)
            .await
            .map_err(|_| Error::Bus)?;
        self.sensor
            .gy_data_rate_set(odr)
            .await
            .map_err(|_| Error::Bus)?;
        Ok(())
    }

    /// Reads acceleration and angular rate on all three axes
    pub async fn read(&mut self) -> Result<ImuReading, Error> {
        let a = self
            .sensor
            .acceleration_raw_get()
            .await
            .map_err(|_| Error::Bus)?;
        let g = self
            .sensor
            .angular_rate_raw_get()
            .await
            .map_err(|_| Error::Bus)?;

        let to_mg = match self.accel_range {
            AccelRange::G2 => st::from_fs2_to_mg,
            AccelRange::G4 => st::from_fs4_to_mg,
            AccelRange::G8 => st::from_fs8_to_mg,
            AccelRange::G16 => st::from_fs16_to_mg,
        };
        let to_mdps = match self.gyro_range {
            GyroRange::Dps125 => st::from_fs125_to_mdps,
            GyroRange::Dps250 => st::from_fs250_to_mdps,
            GyroRange::Dps500 => st::from_fs500_to_mdps,
            GyroRange::Dps1000 => st::from_fs1000_to_mdps,
            GyroRange::Dps2000 => st::from_fs2000_to_mdps,
            GyroRange::Dps4000 => st::from_fs4000_to_mdps,
        };

        Ok(ImuReading {
            accel_mg: Axes {
                x: to_mg(a[0]),
                y: to_mg(a[1]),
                z: to_mg(a[2]),
            },
            gyro_mdps: Axes {
                x: to_mdps(g[0]),
                y: to_mdps(g[1]),
                z: to_mdps(g[2]),
            },
        })
    }

    /// Direct access to ST's driver, for settings this wrapper doesn't expose [same as lis2mdl]
    pub fn inner(&mut self) -> &mut StLsm6dsv<SpiBus<SPI>, D> {
        &mut self.sensor
    }
}
