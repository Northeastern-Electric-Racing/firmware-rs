#![no_std]
//! Driver for the STMicroelectronics M24 series I2C EEPROMs with a two byte address.
//!
//! One [`Eeprom`] serves both flavours, the way `embassy_stm32`'s own `I2c` does:
//! async methods take the plain names, blocking ones take a `blocking_` prefix, and
//! both `embedded-storage` traits are implemented. There is no single trait that is
//! blocking and async at once - `embedded_hal::i2c::I2c` and
//! `embedded_hal_async::i2c::I2c` are separate traits in separate crates, and a type
//! that does both simply implements both - so this driver does the same.
//!
//! ```ignore
//! let mut eeprom = M24C32::new(i2c, Delay, ChipEnable::GROUNDED);
//! eeprom.write(0, b"hello").await?;       // async
//! eeprom.blocking_write(0, b"hello")?;    // blocking
//! ```
//!
//! After construction the layer that lays out records - banks, rings, wear levelling,
//! none of which belong here - can take `&mut impl Storage` and never name this crate
//! again.
//!
//! ## The family
//!
//! These are the M24 parts whose byte address fits in two bytes. Below them the
//! M24C01/C02 use a single address byte and the M24C04/C08/C16 steal their high
//! address bits from the chip enable field, so one device answers on several bus
//! addresses; above them the M24M01 needs a seventeenth address bit and steals it the
//! same way. Neither shape fits the wire format here.
//!
//! Listing all five costs nothing in a binary: a generic is only compiled for the
//! parameters it is instantiated with, so an alias nobody constructs emits no code.

#[cfg(feature = "async")]
mod async_store;
#[cfg(feature = "sync")]
mod blocking_store;

/// Largest page in the supported family (M24512). The command buffer is sized for it
/// so that the page size can stay a const generic without `generic_const_exprs`.
const MAX_PAGE: usize = 128;

/// Two address bytes plus one full page of data.
const CMD_BUF: usize = MAX_PAGE + 2;

/// How long the part may hold the bus off while it commits a page, per datasheet tW
/// (5 ms max). Polled once a millisecond, with margin.
const WRITE_CYCLE_POLLS: u32 = 10;

/// Delay between acknowledge polls, in milliseconds.
const POLL_INTERVAL_MS: u32 = 1;

/// How the part's three chip enable pins are strapped on the board.
///
/// E2/E1/E0 are the low three bits of the device select byte, which is why eight of
/// these parts can share one bus. This is a strapping, not a memory address and not a
/// bus address: it says which physical device you are talking to.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChipEnable(u8);

impl ChipEnable {
    /// All three pins tied to ground. The usual strapping where a board carries one
    /// of these parts.
    pub const GROUNDED: Self = Self(0);

    /// From the level on each pin.
    pub const fn from_pins(e2: bool, e1: bool, e0: bool) -> Self {
        Self(((e2 as u8) << 2u32) | ((e1 as u8) << 1u32) | e0 as u8)
    }

    /// From the strapping read off the schematic as a number, `0b_E2_E1_E0`. Anything
    /// above the low three bits would walk off the select field and is discarded.
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b111)
    }

    /// The seven bit address this part answers to on the bus: `1010` then E2/E1/E0.
    const fn bus_address(self) -> u8 {
        0b1010_000 | self.0
    }
}

/// A failed access.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// The access runs past the end of the array, where the device would wrap around
    /// and overwrite the start of memory instead of failing.
    OutOfBounds {
        /// First byte the caller asked for.
        offset: u32,
        /// How many bytes the caller asked for.
        len: u32,
        /// Size of the array.
        capacity: u32,
    },
    /// The part never acknowledged again after a write, so the page may or may not
    /// have committed. Whether the data landed is not knowable from here.
    WriteTimeout,
    /// The bus itself failed.
    I2c(E),
}

/// An M24 series EEPROM.
///
/// Reach for a part alias - [`M24C32`] and friends - rather than naming the capacity
/// and page size by hand.
pub struct Eeprom<I2C, D, const CAPACITY: u32, const PAGE: u32> {
    i2c: I2C,
    delay: D,
    /// Which device on the bus, already resolved to its seven bit address.
    address: u8,
    /// Address bytes and payload, staged here so a page write goes out as one
    /// uninterrupted bus write.
    cmd: [u8; CMD_BUF],
}

/// M24C32: 4 KB, 32 byte pages.
pub type M24C32<I2C, D> = Eeprom<I2C, D, 4096, 32>;
/// M24C64: 8 KB, 32 byte pages.
pub type M24C64<I2C, D> = Eeprom<I2C, D, 8192, 32>;
/// M24128: 16 KB, 64 byte pages.
pub type M24128<I2C, D> = Eeprom<I2C, D, 16_384, 64>;
/// M24256: 32 KB, 64 byte pages.
pub type M24256<I2C, D> = Eeprom<I2C, D, 32_768, 64>;
/// M24512: 64 KB, 128 byte pages.
pub type M24512<I2C, D> = Eeprom<I2C, D, 65_536, 128>;

impl<I2C, D, const CAPACITY: u32, const PAGE: u32> Eeprom<I2C, D, CAPACITY, PAGE> {
    /// Bind the driver to the part strapped to `chip_enable`.
    ///
    /// The delay is held rather than passed per call, so that the `embedded-storage`
    /// traits - which take no delay - fit this type directly.
    pub const fn new(i2c: I2C, delay: D, chip_enable: ChipEnable) -> Self {
        Self {
            i2c,
            delay,
            address: chip_enable.bus_address(),
            cmd: [0u8; CMD_BUF],
        }
    }

    /// Size of the array in bytes.
    pub const fn capacity(&self) -> u32 {
        CAPACITY
    }

    /// Give the bus and the delay back.
    pub fn release(self) -> (I2C, D) {
        (self.i2c, self.delay)
    }

    /// Reject an access that would run off the end of the array, since the hardware
    /// wraps around to the start rather than refusing.
    fn check_bounds<E>(offset: u32, len: usize) -> Result<(), Error<E>> {
        let len = len as u32;
        if offset.checked_add(len).is_none_or(|end| end > CAPACITY) {
            return Err(Error::OutOfBounds {
                offset,
                len,
                capacity: CAPACITY,
            });
        }
        Ok(())
    }

    /// Stage a page write: the two address bytes, then the payload. Returns how many
    /// bytes of `cmd` to put on the bus.
    fn stage(&mut self, offset: u32, data: &[u8]) -> usize {
        debug_assert!(data.len() <= PAGE as usize);
        self.cmd[..2].copy_from_slice(&memory_address(offset));
        self.cmd[2..data.len() + 2].copy_from_slice(data);
        data.len() + 2
    }
}

/// A byte offset within the array, as the two bytes that carry it on the wire.
///
/// This is where in memory you are reading or writing, sent after the device select
/// byte. Nothing to do with [`ChipEnable`], which picks the device itself.
const fn memory_address(offset: u32) -> [u8; 2] {
    [(offset >> 8u32) as u8, offset as u8]
}

/// Splits an access into the runs that fall inside a single page.
///
/// A write that crosses a page boundary does not continue into the next page - the
/// address wraps back to the start of the page it began in, quietly corrupting what
/// was there. Reads have no such limit and need no splitting.
struct PageChunks {
    next: u32,
    end: u32,
    base: u32,
    page: u32,
}

impl PageChunks {
    const fn new(offset: u32, len: u32, page: u32) -> Self {
        Self {
            next: offset,
            end: offset + len,
            base: offset,
            page,
        }
    }
}

impl Iterator for PageChunks {
    /// The device address this run starts at, and where it sits in the caller's buffer.
    type Item = (u32, core::ops::Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }
        let page_end = (self.next / self.page + 1) * self.page;
        let run_end = page_end.min(self.end);
        let item = (
            self.next,
            (self.next - self.base) as usize..(run_end - self.base) as usize,
        );
        self.next = run_end;
        Some(item)
    }
}
