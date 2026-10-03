#![no_std]
//! Driver for LIS2MDL magnetometer

// NOTE: All drivers shoould only ever use embedded hal traits for standardization
// this makes them eternally portable yet still easy to use from embassy
// therefore, no import should EVER mention

use embedded_hal_async::{delay::DelayNs, spi::SpiDevice};
use lis2mdl_rs::asynchronous::driver::{self as st, Lis2mdl as StLis2mdl};
use lis2mdl_rs::asynchronous::register::OnState;
use lis2mdl_rs::asynchronous::register::main::{Md, Odr, Sim};
use st_mems_bus::asynchronous::spi::SpiBus;
 
/// wait after a software reset before talking to the sensor again, in ms (as done is C)
const RESET_WAIT_MS: u32 = 10;
 
/// Errors
#[derive(Debug, defmt::Format)]
pub enum Error {
    /// SPI bus error 
    Bus,
    /// The WHO_AM_I register didn't match the LIS2MDL. Usually wiring, SPI mode, or chip select.
    WrongId(u8),
}
 
///  magnetometer reading
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct MagneticField {
    /// in milligauss
    pub x_mgauss: f32,
    pub y_mgauss: f32,
    pub z_mgauss: f32,
}
 
/// LIS2MDL on SPI bus
pub struct Lis2mdl<SPI: SpiDevice, D: DelayNs> {
    sensor: StLis2mdl<SpiBus<SPI>, D, OnState>,
}
 
impl<SPI: SpiDevice, D: DelayNs> Lis2mdl<SPI, D> {
    /// to create a new driver
    pub fn new(spi: SPI, delay: D) -> Self {
        Self {
            sensor: StLis2mdl::new_spi(spi, delay),
        }
    }
 
    /// Sets the sensor up the same way the C firmware did: check the ID, reset, continuous
    /// mode at 50 Hz, temperature compensation on, block data update on.
    ///
    /// Also switches the sensor to 4-wire SPI first. It powers up in 3-wire mode, where it
    /// answers on the MOSI line, so on a board with a separate MISO line no read works until
    /// this is done.
    pub async fn init(&mut self) -> Result<(), Error> {
        // Writes only use MOSI, so this works even while the sensor is still in 3-wire mode
        self.set_4wire().await?;
 
        let id = self.sensor.device_id_get().await.map_err(|_| Error::Bus)?;
        if id != st::ID {
            return Err(Error::WrongId(id));
        }
 
        // `sw_reset()` is the non-deprecated reset, but it polls for completion with a read,
        // and the reset may put the sensor back into 3-wire mode, where that read can fail.
        // So reset, wait a fixed time like the C firmware, then switch back to 4-wire.
        #[allow(deprecated)]
        self.sensor.reset_set(1).await.map_err(|_| Error::Bus)?;
        self.sensor.tim.delay_ms(RESET_WAIT_MS).await;
        self.set_4wire().await?;
 
        self.sensor.operating_mode_set(Md::ContinuousMode).await.map_err(|_| Error::Bus)?;
        self.sensor.data_rate_set(Odr::_50hz).await.map_err(|_| Error::Bus)?;
        // Keeps readings stable as the board heats up.
        self.sensor.offset_temp_comp_set(1).await.map_err(|_| Error::Bus)?;
        // Output registers don't change halfway through a read.
        self.sensor.block_data_update_set(1).await.map_err(|_| Error::Bus)?;
 
        Ok(())
    }
 
    /// reads the magnetic field. Returns `Ok(None)` if there's no new sample since the last read
    pub async fn read(&mut self) -> Result<Option<MagneticField>, Error> {
        if self.sensor.mag_data_ready_get().await.map_err(|_| Error::Bus)? == 0 {
            return Ok(None);
        }
 
        let raw = self.sensor.magnetic_raw_get().await.map_err(|_| Error::Bus)?;
        Ok(Some(MagneticField {
            x_mgauss: st::from_lsb_to_mgauss(raw[0]),
            y_mgauss: st::from_lsb_to_mgauss(raw[1]),
            z_mgauss: st::from_lsb_to_mgauss(raw[2]),
        }))
    }
 
    /// Direct access to ST's driver, for settings this wrapper doesn't expose.
    pub fn inner(&mut self) -> &mut StLis2mdl<SpiBus<SPI>, D, OnState> {
        &mut self.sensor
    }
 
    async fn set_4wire(&mut self) -> Result<(), Error> {
        self.sensor.spi_mode_set(Sim::Spi4Wire).await.map_err(|_| Error::Bus)
    }
}