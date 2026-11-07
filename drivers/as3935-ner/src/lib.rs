//! an SPI driver for an as3935 sensor found on the lightning board
#![no_std]

use embedded_hal_async::spi::SpiDevice;

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
        todo!("need to implement!")
    }

    async fn write_register(&mut self, addr: u8, value: u8) -> Result<(), Error<SPI::Error>> {
        todo!("need to implement!")
    }
}

pub enum Error<E> {
    /// SPI hardware failed error
    Spi(E),
}

