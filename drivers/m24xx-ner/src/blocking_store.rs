//! The blocking half of [`Eeprom`], over `embedded-hal`.
//!
//! The same sequence as [`crate::async_store`] without the awaits. The two cannot
//! share one body: `embedded_hal::i2c::I2c` and `embedded_hal_async::i2c::I2c` are
//! separate traits whose methods differ in return type, and stable Rust has no way to
//! be generic over that.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{Error as _, ErrorKind, I2c};

use crate::{Eeprom, Error, POLL_INTERVAL_MS, PageChunks, WRITE_CYCLE_POLLS, memory_address};

impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE: u32> Eeprom<I2C, D, CAPACITY, PAGE> {
    /// Blocking [`read`](Eeprom::read).
    pub fn blocking_read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error<I2C::Error>> {
        Self::check_bounds(offset, buf.len())?;
        if buf.is_empty() {
            return Ok(());
        }
        self.i2c
            .write_read(self.address, &memory_address(offset), buf)
            .map_err(Error::I2c)
    }

    /// Blocking [`write`](Eeprom::write).
    pub fn blocking_write(&mut self, offset: u32, data: &[u8]) -> Result<(), Error<I2C::Error>> {
        Self::check_bounds(offset, data.len())?;
        for (addr, run) in PageChunks::new(offset, data.len() as u32, PAGE) {
            self.blocking_write_page(addr, &data[run])?;
        }
        Ok(())
    }

    /// Send one page, then hold until the part has finished committing it.
    fn blocking_write_page(&mut self, offset: u32, data: &[u8]) -> Result<(), Error<I2C::Error>> {
        let len = self.stage(offset, data);

        // A NACK means an earlier write is still committing and the part has taken
        // itself off the bus, so retrying doubles as the wait for it. Any other
        // failure is a real bus fault and retrying would only hide it.
        let mut polls = 0u32;
        while let Err(err) = self.i2c.write(self.address, &self.cmd[..len]) {
            if !matches!(err.kind(), ErrorKind::NoAcknowledge(_)) {
                return Err(Error::I2c(err));
            }
            if polls >= WRITE_CYCLE_POLLS {
                return Err(Error::WriteTimeout);
            }
            polls += 1;
            self.delay.delay_ms(POLL_INTERVAL_MS);
        }

        self.blocking_wait_ready()
    }

    /// Poll until the part acknowledges again, meaning the write cycle is over.
    fn blocking_wait_ready(&mut self) -> Result<(), Error<I2C::Error>> {
        // A one byte read, rather than a zero length write: an address-only
        // transaction is not portable across HALs, and the byte is discarded anyway.
        let mut scratch = [0u8; 1];
        for _ in 0..WRITE_CYCLE_POLLS {
            self.delay.delay_ms(POLL_INTERVAL_MS);
            match self.i2c.read(self.address, &mut scratch) {
                Ok(()) => return Ok(()),
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => (),
                Err(err) => return Err(Error::I2c(err)),
            }
        }
        Err(Error::WriteTimeout)
    }
}

#[cfg(feature = "storage")]
impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE: u32> embedded_storage::ReadStorage
    for Eeprom<I2C, D, CAPACITY, PAGE>
{
    type Error = Error<I2C::Error>;

    fn capacity(&self) -> usize {
        CAPACITY as usize
    }

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.blocking_read(offset, bytes)
    }
}

#[cfg(feature = "storage")]
impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE: u32> embedded_storage::Storage
    for Eeprom<I2C, D, CAPACITY, PAGE>
{
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.blocking_write(offset, bytes)
    }
}
