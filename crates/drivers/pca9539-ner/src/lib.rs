#![no_std]
use embedded_hal_async::i2c::I2c;

/// Defines errors
#[derive(Debug, Copy, Clone)]
pub enum Error<E> {
    /// Underlying bus error
    BusError(E),
}

impl<E> From<E> for Error<E> {
    fn from(error: E) -> Self {
        Error::BusError(error)
    }
}

// /// Pin modes.
// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
// pub enum Direction {
//     /// Represents input mode.
//     Input = 1,
//     /// Represents output mode.
//     Output = 0,
// }

// /// Pin levels.
// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
// pub enum Level {
//     /// High level
//     High = 1,
//     /// Low level
//     Low = 0,
// }

/// Pin names
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Pin {
    #[default]
    P00 = 0,
    P01 = 1,
    P02 = 2,
    P03 = 3,
    P04 = 4,
    P05 = 5,
    P06 = 6,
    P07 = 7,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Bank {
    Bank0 = 0,
    Bank1 = 1,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum RegisterType {
    /// 1 = high
    InputLevel = 0,
    /// 1 = high, default 1
    OutputLevel = 2,
    /// 1 = inverted, default 0
    PolarityInverted = 4,
    /// 1 = input, default 1
    Direction = 6,
}

/// PCA9539/TCA9539 is a 16-pin I2C I/O Expander with I2C Interface.
#[derive(Clone, Copy, Debug)]
pub struct Pca9539<I2C> {
    i2c: I2C,
    address: u8,
}

/// Pca9539 GPIO expander (or TCA9539)
impl<I2C: I2c, E> Pca9539<I2C>
where
    I2C: I2c<Error = E>,
{
    const DEFAULT_ADDRESS: u8 = 0x74;

    /// Creates an expander with the default configuration.
    pub fn new_default(i2c: I2C) -> Result<Self, Error<E>> {
        Self::new(i2c, Self::DEFAULT_ADDRESS)
    }

    /// Creates an expander with specific address.
    pub fn new(i2c: I2C, address: u8) -> Result<Self, Error<E>> {
        Ok(Self { i2c, address })
    }

    /// Return the I2C address
    pub fn address(&self) -> u8 {
        self.address
    }

    // base functions

    /// Read an 8 bit register
    pub async fn read(&mut self, addr: u8) -> Result<u8, E> {
        let mut data = [0u8];
        self.i2c
            .write_read(self.address, &[addr], &mut data)
            .await?;
        Ok(data[0])
    }

    /// Write an 8 bit register
    pub async fn write(&mut self, addr: u8, data: u8) -> Result<(), E> {
        self.i2c.write(self.address, &[addr, data]).await
    }

    // register abstractions

    pub async fn write_register(
        &mut self,
        reg: RegisterType,
        bank: Bank,
        data: u8,
    ) -> Result<(), E> {
        self.write(reg as u8 + bank as u8, data).await
    }

    pub async fn read_register(&mut self, reg: RegisterType, bank: Bank) -> Result<u8, E> {
        self.read(reg as u8 + bank as u8).await
    }

    // helper functions

    pub async fn write_pin(
        &mut self,
        reg: RegisterType,
        bank: Bank,
        pin: Pin,
        state: bool,
    ) -> Result<(), E> {
        let old_state = self.read_register(reg, bank).await?;
        let new_state = (old_state & !(1u8 << pin as u8)) | ((state as u8) << pin as u8);

        self.write_register(reg, bank, new_state).await
    }

    pub async fn read_pin(&mut self, reg: RegisterType, bank: Bank, pin: Pin) -> Result<bool, E> {
        let data = self.read_register(reg, bank).await?;
        Ok((data & (1 << pin as u32)) > 0)
    }
}
