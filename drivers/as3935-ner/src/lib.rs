//! an SPI driver for an as3935 sensor found on the lightning board
#![no_std]

use embedded_hal_async::spi::SpiDevice;

/// Register addresses are 6 bits (B13–B8 of the SPI frame).
const ADDRESS_MASK: u8 = 0x3F;

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

    async fn read_register(&mut self, addr: u8) -> Result<u8, Error<SPI::Error>> {
        let command_byte = (addr & ADDRESS_MASK) | 0x40;
        let mut buf: [u8; 2] = [command_byte, 0x00];

        SpiDevice::transfer_in_place(&mut self.spi, &mut buf).await.map_err(Error::Spi)?;
        Ok(buf[1])
    }

    async fn write_register(&mut self, addr: u8, value: u8) -> Result<(), Error<SPI::Error>> {
        let command_byte = (addr & ADDRESS_MASK) | 0x00; // write mode will always be 0x00 so the OR does nothing
        let buf: [u8; 2] = [command_byte, value];

        SpiDevice::write(&mut self.spi, &buf).await.map_err(Error::Spi)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error<E> {
    /// SPI hardware failed error
    Spi(E),
}

