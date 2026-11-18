//! an SPI driver for an as3935 sensor found on the lightning board
#![no_std]

mod chip;

use crate::chip::registers::Register;
use embedded_hal_async::spi::SpiDevice;

/// Register addresses are 6 bits (B13–B8 of the SPI frame).
const ADDRESS_MASK: u8 = 0x3F;
const DIRECT_COMMAND: u8 = 0x96;

pub struct As3935<SPI> {
    spi: SPI
}

impl<SPI> As3935<SPI>
    // / this is a trait bound, its a requirement on what types are allowed
    // / "this impl only applies when SPI is a type that implements SpiDevice"
    where SPI: SpiDevice,
{
    pub fn new(spi: SPI) -> Self {
        Self {spi}
    }

    async fn read_register(&mut self, reg: Register) -> Result<u8, Error<SPI::Error>> {
        let command_byte = (reg as u8 & ADDRESS_MASK) | 0x40;
        let mut buf: [u8; 2] = [command_byte, 0x00];

        SpiDevice::transfer_in_place(&mut self.spi, &mut buf).await.map_err(Error::Spi)?;
        Ok(buf[1])
    }

    async fn write_register(&mut self, reg: Register, value: u8) -> Result<(), Error<SPI::Error>> {
        let command_byte = (reg as u8 & ADDRESS_MASK) | 0x00; // write mode will always be 0x00 so the OR does nothing
        let buf: [u8; 2] = [command_byte, value];

        SpiDevice::write(&mut self.spi, &buf).await.map_err(Error::Spi)?;
        Ok(())
    }
    /// calls write_register with an address that sets every register to their defaults and returns
    /// something idk what it returns
    pub async fn preset_default(&mut self) -> Result<(), Error<SPI::Error>> {

        self.write_register(Register::PresetDefault, DIRECT_COMMAND).await
    }

    async fn read_field(&mut self, reg: Register, shift: u8, width: u8) -> Result<u8, Error<SPI::Error>> {
        let register = self.read_register(reg).await?; // / reads the passed reg argument
        let slide_right = register >> shift;
        let mask = (1 << width) - 1;
        let covered = slide_right & mask;

        Ok(covered)
    }

    async fn write_field(&mut self, reg: Register, shift: u8, width: u8, value: u8) -> Result<(), Error<SPI::Error>> {
        let register = self.read_register(reg).await?;
        let mask = (1 << width) - 1;
        let erase = !(mask << shift);
        let erased_register = register & erase;
        let value_in_position = (value & mask) << shift;
        let combined_register = erased_register | value_in_position;

        self.write_register(reg, combined_register).await
    }
}

#[derive(Debug)]
pub enum Error<E> {
    /// SPI hardware failed error
    Spi(E),
}