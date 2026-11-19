//! Async `embedded-hal` SPI driver for the ams AS3935 Franklin lightning sensor
//! (used on the NER lightning board).
//!
//! ### SPI setup (done by the application, not this driver)
//! - SPI mode 1 (CPOL = 0, CPHA = 1)
//! - SCLK at most 2 MHz
//! - Chip select is handled by the [`SpiDevice`] implementation
//!
//! The sensor's IRQ pin is not handled here: the application waits for it to go
//! high, then calls [`As3935::interrupt`] to find out why it fired.
#![no_std]

mod chip;
mod types;
pub use types::{Afe, Distance, Interrupt};

use crate::chip::registers::Register;
use embedded_hal_async::spi::SpiDevice;

/// Register addresses are 6 bits (B13–B8 of the SPI frame).
const ADDRESS_MASK: u8 = 0x3F;
/// Value written to a direct-command register (PresetDefault, CalibRco) to trigger it.
const DIRECT_COMMAND: u8 = 0x96;

/// An AS3935 lightning sensor connected over SPI.
pub struct As3935<SPI> {
    spi: SPI,
}

impl<SPI> As3935<SPI>
// this is a trait bound, its a requirement on what types are allowed
// "this impl only applies when SPI is a type that implements SpiDevice"
where
    SPI: SpiDevice,
{
    /// Creates the driver. Doesn't talk to the chip yet.
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// Reads one whole byte from a register on the chip.
    /// Sends [read command + address, dummy byte]; the chip replies with the value in the second byte.
    async fn read_register(&mut self, reg: Register) -> Result<u8, Error<SPI::Error>> {
        let command_byte = (reg as u8 & ADDRESS_MASK) | 0x40;
        let mut buf: [u8; 2] = [command_byte, 0x00];

        SpiDevice::transfer_in_place(&mut self.spi, &mut buf)
            .await
            .map_err(Error::Spi)?;
        Ok(buf[1])
    }

    /// Writes one whole byte to a register on the chip, replacing all 8 bits.
    /// Sends [write command + address, value] in a single SPI transfer.
    async fn write_register(&mut self, reg: Register, value: u8) -> Result<(), Error<SPI::Error>> {
        let command_byte = (reg as u8 & ADDRESS_MASK) | 0x00; // write mode will always be 0x00 so the OR does nothing
        let buf: [u8; 2] = [command_byte, value];

        SpiDevice::write(&mut self.spi, &buf)
            .await
            .map_err(Error::Spi)?;
        Ok(())
    }

    /// Resets every register on the chip to its factory default value.
    /// Works by writing the magic value 0x96 to the PresetDefault register (a "direct command")
    pub async fn preset_default(&mut self) -> Result<(), Error<SPI::Error>> {
        self.write_register(Register::PresetDefault, DIRECT_COMMAND)
            .await
    }

    /// Reads a group of bits (a "field") out of one register, e.g. bits `[6:4]`.
    /// Reads the whole byte, slides the field down to the bottom (>> shift), then masks off the rest.
    ///
    /// `shift` is the field's lowest bit number, `width` is how many bits it has (1 to 7).
    async fn read_field(
        &mut self,
        reg: Register,
        shift: u8,
        width: u8,
    ) -> Result<u8, Error<SPI::Error>> {
        let register = self.read_register(reg).await?; // reads the passed reg argument
        let slide_right = register >> shift;
        let mask = (1 << width) - 1;
        let covered = slide_right & mask;

        Ok(covered)
    }

    /// Changes a group of bits inside one register without touching the other bits.
    /// Reads the byte, erases just the field's bits, puts the new value in their place, writes it back.
    async fn write_field(
        &mut self,
        reg: Register,
        shift: u8,
        width: u8,
        value: u8,
    ) -> Result<(), Error<SPI::Error>> {
        let register = self.read_register(reg).await?;
        let mask = (1 << width) - 1;
        let erase = !(mask << shift);
        let erased_register = register & erase;
        let value_in_position = (value & mask) << shift;
        let combined_register = erased_register | value_in_position;

        self.write_register(reg, combined_register).await
    }

    /// Calibrates the chip's internal RC oscillators.
    /// Works by writing the magic value 0x96 to the CalibRco register (a "direct command").
    pub async fn calibrate_rco(&mut self) -> Result<(), Error<SPI::Error>> {
        self.write_register(Register::CalibRco, DIRECT_COMMAND)
            .await
    }

    /// Reads why the IRQ pin fired.
    ///
    /// Call this after the IRQ pin goes high. Returns [`Error::UnknownValue`] if the
    /// chip reports a value the datasheet doesn't define.
    pub async fn interrupt(&mut self) -> Result<Interrupt, Error<SPI::Error>> {
        let raw_number = self.read_field(Register::Frequency, 0, 4).await?;

        match raw_number {
            0 => Ok(Interrupt::None),
            0b0001 => Ok(Interrupt::NoiseTooHigh),
            0b0100 => Ok(Interrupt::Disturber),
            0b1000 => Ok(Interrupt::Lightning),
            other => Err(Error::UnknownValue(other)),
        }
    }

    /// Reads the estimated distance to the front of the storm.
    ///
    /// Only meaningful after [`interrupt`](Self::interrupt) returns [`Interrupt::Lightning`].
    pub async fn distance(&mut self) -> Result<Distance, Error<SPI::Error>> {
        let raw_distance = self.read_field(Register::DistanceEstimation, 0, 6).await?;

        match raw_distance {
            0b000001 => Ok(Distance::Overhead),
            0b111111 => Ok(Distance::OutOfRange),
            km => Ok(Distance::Km(km)),
        }
    }

    /// Sets the analog front-end gain for indoor or outdoor use.
    /// Only changes bits `[5:1]` of the Config register.
    pub async fn set_afe(&mut self, setting: Afe) -> Result<(), Error<SPI::Error>> {
        self.write_field(Register::Config, 1, 5, setting as u8)
            .await
    }

    /// Reads the energy of the last lightning strike.
    ///
    /// This is a raw 21-bit number with no physical unit; bigger means a stronger strike.
    /// Only meaningful after [`interrupt`](Self::interrupt) returns [`Interrupt::Lightning`].
    pub async fn energy(&mut self) -> Result<u32, Error<SPI::Error>> {
        let energy_lsb = self.read_register(Register::EnergyLsb).await?;
        let energy_mmsb = self.read_field(Register::EnergyMmsb, 0, 5).await?;
        let energy_msb = self.read_register(Register::EnergyMsb).await?;

        let energy_lsb = energy_lsb as u32;
        let energy_msb = energy_msb as u32;
        let energy_mmsb = energy_mmsb as u32;

        let shift_mmsb = energy_mmsb << 16_u32;
        let shift_msb = energy_msb << 8_u32;

        Ok(shift_mmsb | shift_msb | energy_lsb)
    }
}

/// Errors returned by the driver.
#[derive(Debug)]
pub enum Error<E> {
    /// The SPI transfer failed. Holds the SPI implementation's own error.
    Spi(E),
    /// The chip returned a value the datasheet doesn't define. Holds the raw value so it can be logged.
    UnknownValue(u8),
}
