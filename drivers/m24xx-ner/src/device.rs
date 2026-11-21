use crate::{DelayNs, Error, I2c, PagedMemory};
use embedded_hal::i2c::{Error as _, ErrorKind, Operation};

/// Datasheet tW, the longest a write cycle can take.
const WRITE_CYCLE_MS: u32 = 5;
const POLL_INTERVAL_MS: u32 = 1;
/// Twice tW, so a part still busy at the last poll is a part that is not merely busy.
const WRITE_CYCLE_POLLS: u32 = 2u32 * WRITE_CYCLE_MS / POLL_INTERVAL_MS;

/// Every M24 answers to this, above the chip enable bits.
const DEVICE_TYPE: u8 = 0b1010;
/// Chip enable pins E2, E1 and E0.
const CHIP_ENABLE_BITS: u32 = 3;
/// Bits of array address an M24 takes, after the bus address. [`Eeprom::pages`] is
/// where a part too large for that fails the build.
const ADDRESS_WIDTH: u32 = 16;

/// An array address as the two bytes that carry it, big endian.
const fn address_bytes(address: u32) -> [u8; 2] {
    (address as u16).to_be_bytes()
}

/// The low three bits of the part's I2C address, from the chip enable pins E2..E0.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AddressBits(u8);

impl AddressBits {
    /// All three chip enable pins tied to ground.
    pub const GROUNDED: Self = Self(0);

    /// From the level on each chip enable pin.
    pub const fn from_pins(e2: bool, e1: bool, e0: bool) -> Self {
        Self(((e2 as u8) << 2u32) | ((e1 as u8) << 1u32) | e0 as u8)
    }

    const fn bus_address(self) -> u8 {
        (DEVICE_TYPE << CHIP_ENABLE_BITS) | self.0
    }
}

/// An M24 series EEPROM, read and written a page at a time through [`PagedMemory`].
/// `CAPACITY` and `PAGE_SIZE` are in bytes; the aliases below are how one is named.
pub struct Eeprom<I2C, D, const CAPACITY: u32, const PAGE_SIZE: usize> {
    i2c: I2C,
    delay: D,
    bus_address: u8,
}

/// Bytes in a part of `kbit` kilobits.
const fn kbits(kbit: u32) -> u32 {
    kbit * 1024u32 / 8u32
}

/// M24C32, in 32 byte pages.
pub type M24C32<I2C, D> = Eeprom<I2C, D, { kbits(32) }, 32>;
/// M24C64, in 32 byte pages.
pub type M24C64<I2C, D> = Eeprom<I2C, D, { kbits(64) }, 32>;
/// M24128, in 64 byte pages.
pub type M24128<I2C, D> = Eeprom<I2C, D, { kbits(128) }, 64>;
/// M24256, in 64 byte pages.
pub type M24256<I2C, D> = Eeprom<I2C, D, { kbits(256) }, 64>;
/// M24512, in 128 byte pages.
pub type M24512<I2C, D> = Eeprom<I2C, D, { kbits(512) }, 128>;

impl<I2C, D, const CAPACITY: u32, const PAGE_SIZE: usize> Eeprom<I2C, D, CAPACITY, PAGE_SIZE> {
    /// The part strapped to `address`. Talks to nothing until it is asked to.
    pub const fn new(i2c: I2C, delay: D, address: AddressBits) -> Self {
        Self {
            i2c,
            delay,
            bus_address: address.bus_address(),
        }
    }

    /// How many pages the array holds, which bounds any layout.
    pub const fn pages() -> usize {
        assert!(
            CAPACITY <= 1u32 << ADDRESS_WIDTH,
            "an M24 address goes out as two bytes"
        );
        CAPACITY as usize / PAGE_SIZE
    }

    /// Bytes in one page, the unit everything atomic is counted in.
    pub const fn page_size() -> usize {
        PAGE_SIZE
    }

    /// The one place a page index becomes an address, and the only bounds check.
    const fn address<E>(page: usize, len: usize) -> Result<u32, Error<E>> {
        if len > PAGE_SIZE {
            return Err(Error::TooLong { len });
        }
        if page >= Self::pages() {
            return Err(Error::NoSuchPage { page });
        }
        Ok((page * PAGE_SIZE) as u32)
    }
}

impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE_SIZE: usize> PagedMemory
    for Eeprom<I2C, D, CAPACITY, PAGE_SIZE>
{
    type BusError = I2C::Error;

    async fn read_page(
        &mut self,
        page: usize,
        buf: &mut [u8],
    ) -> Result<(), Error<Self::BusError>> {
        let addr_bytes = address_bytes(Self::address(page, buf.len())?);
        if buf.is_empty() {
            return Ok(());
        }

        // Another store on the same part may be mid write cycle, during which this one
        // is NACKed rather than answered. Same retry as a write.
        let mut polls = 0u32;
        loop {
            match self
                .i2c
                .write_read(self.bus_address, &addr_bytes, buf)
                .await
            {
                Ok(()) => return Ok(()),
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => {
                    if polls >= WRITE_CYCLE_POLLS {
                        return Err(Error::NoDevice);
                    }
                    polls += 1;
                    self.delay.delay_ms(POLL_INTERVAL_MS).await;
                }
                Err(err) => return Err(Error::I2c(err)),
            }
        }
    }

    async fn write_page(&mut self, page: usize, data: &[u8]) -> Result<(), Error<Self::BusError>> {
        let addr_bytes = address_bytes(Self::address(page, data.len())?);

        let mut polls = 0u32;
        loop {
            // Adjacent writes are sent with no stop or repeated start between them, so
            // this is one bus write without staging both into a buffer.
            let mut ops = [Operation::Write(&addr_bytes), Operation::Write(data)];
            match self.i2c.transaction(self.bus_address, &mut ops).await {
                Ok(()) => break,
                // A NACK means an earlier write is still committing and the part has
                // taken itself off the bus, so retrying doubles as the wait for it.
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => {
                    if polls >= WRITE_CYCLE_POLLS {
                        let mut scratch = [0u8; 1];
                        let probe = self.i2c.read(self.bus_address, &mut scratch).await;
                        return Err(classify_refused_write(probe));
                    }
                    polls += 1;
                    self.delay.delay_ms(POLL_INTERVAL_MS).await;
                }
                Err(err) => return Err(Error::I2c(err)),
            }
        }

        // A one byte read rather than a zero length write: an address-only transaction
        // is not portable across HALs, and the byte is discarded.
        let mut scratch = [0u8; 1];
        for _ in 0..WRITE_CYCLE_POLLS {
            self.delay.delay_ms(POLL_INTERVAL_MS).await;
            match self.i2c.read(self.bus_address, &mut scratch).await {
                Ok(()) => return Ok(()),
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => (),
                Err(err) => return Err(Error::I2c(err)),
            }
        }
        Err(Error::WriteTimeout)
    }
}

/// A write protected part still acknowledges reads, an absent one acknowledges
/// nothing. Asks the part because most HALs report a NACK's source as unknown.
fn classify_refused_write<E: embedded_hal::i2c::Error>(probe: Result<(), E>) -> Error<E> {
    match probe {
        Ok(()) => Error::WriteProtected,
        Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => Error::NoDevice,
        Err(err) => Error::I2c(err),
    }
}
