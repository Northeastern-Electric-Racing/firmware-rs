//! The async half of [`Eeprom`], over `embedded-hal-async`.

use embedded_hal::i2c::{Error as _, ErrorKind};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;

use crate::{Eeprom, Error, POLL_INTERVAL_MS, PageChunks, WRITE_CYCLE_POLLS, memory_address};

impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE: u32> Eeprom<I2C, D, CAPACITY, PAGE> {
    /// Read `buf.len()` bytes starting at `offset`.
    ///
    /// Sequential reads roll over at the end of the array rather than at a page
    /// boundary, so any length goes out as a single transaction.
    pub async fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error<I2C::Error>> {
        Self::check_bounds(offset, buf.len())?;
        if buf.is_empty() {
            return Ok(());
        }
        self.i2c
            .write_read(self.address, &memory_address(offset), buf)
            .await
            .map_err(Error::I2c)
    }

    /// Write `data` starting at `offset`, splitting at page boundaries.
    ///
    /// Returns once every page has committed, so a read may follow immediately.
    pub async fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Error<I2C::Error>> {
        Self::check_bounds(offset, data.len())?;
        for (addr, run) in PageChunks::new(offset, data.len() as u32, PAGE) {
            self.write_page(addr, &data[run]).await?;
        }
        Ok(())
    }

    /// Send one page, then hold until the part has finished committing it.
    async fn write_page(&mut self, offset: u32, data: &[u8]) -> Result<(), Error<I2C::Error>> {
        let len = self.stage(offset, data);

        // A NACK means an earlier write is still committing and the part has taken
        // itself off the bus, so retrying doubles as the wait for it. Any other
        // failure is a real bus fault and retrying would only hide it.
        let mut polls = 0u32;
        while let Err(err) = self.i2c.write(self.address, &self.cmd[..len]).await {
            if !matches!(err.kind(), ErrorKind::NoAcknowledge(_)) {
                return Err(Error::I2c(err));
            }
            if polls >= WRITE_CYCLE_POLLS {
                return Err(Error::WriteTimeout);
            }
            polls += 1;
            self.delay.delay_ms(POLL_INTERVAL_MS).await;
        }

        self.wait_ready().await
    }

    /// Poll until the part acknowledges again, meaning the write cycle is over.
    async fn wait_ready(&mut self) -> Result<(), Error<I2C::Error>> {
        // A one byte read, rather than a zero length write: an address-only
        // transaction is not portable across HALs, and the byte is discarded anyway.
        let mut scratch = [0u8; 1];
        for _ in 0..WRITE_CYCLE_POLLS {
            self.delay.delay_ms(POLL_INTERVAL_MS).await;
            match self.i2c.read(self.address, &mut scratch).await {
                Ok(()) => return Ok(()),
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => (),
                Err(err) => return Err(Error::I2c(err)),
            }
        }
        Err(Error::WriteTimeout)
    }
}

#[cfg(feature = "storage")]
impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE: u32> embedded_storage_async::ReadStorage
    for Eeprom<I2C, D, CAPACITY, PAGE>
{
    type Error = Error<I2C::Error>;

    fn capacity(&self) -> usize {
        CAPACITY as usize
    }

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        Self::read(self, offset, bytes).await
    }
}

#[cfg(feature = "storage")]
impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE: u32> embedded_storage_async::Storage
    for Eeprom<I2C, D, CAPACITY, PAGE>
{
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        Self::write(self, offset, bytes).await
    }
}
