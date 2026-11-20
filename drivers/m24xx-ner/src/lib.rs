#![no_std]
//! Driver for the STMicroelectronics M24 series EEPROMs
//!
//! A part is reached only through a layout declared with [`eeprom_layout!`].
//!
//! Async only: a page takes up to 5 ms to commit and the part holds the bus off for
//! all of it, which is too long to block an executor.

use embedded_hal::i2c::{Error as _, ErrorKind, Operation};

/// The bus and delay traits a store is generic over, re-exported so a consumer can
/// name them without depending on `embedded-hal-async` itself.
pub use embedded_hal_async::delay::DelayNs;
pub use embedded_hal_async::i2c::I2c;

mod double_buffer;
mod raw;
mod storable;

pub use double_buffer::{DoubleBuffered, SlotState};
pub use raw::Raw;
pub use storable::{BitMask, Storable, bytes_for};

/// Declare what lives on one part, and the only way to reach one.
///
/// ```ignore
/// eeprom_layout! {
///     BmsStore for M24C32;
///
///     bad_thermistors: DoubleBuffered<BadThermistors>,
///     scratch: Raw<4>,
/// }
///
/// let mut store = BmsStore::init(i2c, Delay, ChipEnable::GROUNDED);
/// store.open().await?;
/// store.bad_thermistors().commit(&mask).await?;
/// ```
///
/// A region is `DoubleBuffered<T>` for any `T` implementing [`Storable`], or
/// `Raw<N>` pages with no scheme over them. Regions take the next free pages in declaration order,
/// so no page number is written down and two cannot collide. A value too large for
/// a page, or a layout too large for the part, fails the build.
///
/// One invocation per chip, not per part number.
pub use m24xx_ner_macros::eeprom_layout;

/// Derive [`Storable`] for a struct whose every field is itself a [`Storable`].
///
/// Use [`BitMask`] for a flag array; `[bool; N]` would cost a byte per flag.
///
/// ```ignore
/// #[derive(Clone, PartialEq, Storable)]
/// struct PackSoc {
///     percent: u16,
///     cycles:  u32,
///     bad:     BitMask<23>,
/// }
/// ```
pub use m24xx_ner_macros::Storable;

/// Not public API: what generated code needs in scope.
#[doc(hidden)]
pub mod __private {
    pub use crate::double_buffer::HEADER;
    pub use crate::{DelayNs, I2c};
}

/// Datasheet tW is 5 ms max, polled once a millisecond with margin.
const WRITE_CYCLE_POLLS: u32 = 10;
const POLL_INTERVAL_MS: u32 = 1;

/// How the part's three chip enable pins are strapped on the board.
///
/// (effectively determines I2C address)
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChipEnable(u8);

impl ChipEnable {
    /// All three pins tied to ground.
    pub const GROUNDED: Self = Self(0);

    /// From the level on each pin.
    pub const fn from_pins(e2: bool, e1: bool, e0: bool) -> Self {
        Self(((e2 as u8) << 2u32) | ((e1 as u8) << 1u32) | e0 as u8)
    }

    /// From the strapping read off the schematic as `0b_E2_E1_E0`.
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b111)
    }

    const fn bus_address(self) -> u8 {
        0b1010_000 | self.0
    }
}

/// A failed access.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// Past the end of the array, where the device wraps around and overwrites the start of memory rather than failing.
    OutOfBounds {
        /// First byte the caller asked for.
        address: u32,
        /// How many bytes the caller asked for.
        len: u32,
        /// Size of the array.
        capacity: u32,
    },
    /// The part accepted a write but never became ready again afterwards. Whether the page committed is not knowable from here.
    WriteTimeout,
    /// Nothing answered at the part's address, even after waiting out a write cycle.
    /// Check power, the chip enable strapping, and the bus wiring.
    NoDevice,
    /// The part answers on the bus but refused a write, which is what a part with its
    /// write control (WC) pin held high does: it still acknowledges its address and
    /// reads, but NACKs the data bytes of a write.
    WriteProtected,
    /// The bus itself failed.
    I2c(E),
}

/// A failed access through a layout.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StoreError<E> {
    /// The part underneath failed.
    Storage(E),
    /// What came back did not decode as what went out. The live record was not the
    /// one written to, so the previous value is still current.
    Verify,
    /// Past the end of a [`Raw`] region.
    OutOfRegion {
        /// Offset within the region that was asked for.
        offset: u32,
        /// How many bytes were asked for.
        len: u32,
        /// Size of the region in bytes.
        region: u32,
    },
}

/// An M24 series EEPROM, owned by a layout.
///
/// `CAPACITY` is in bytes, like every address here; `PAGE_SIZE` is in bytes but typed
/// `usize` so a buffer can be exactly one page wide. Name it through a part alias in an
/// [`eeprom_layout!`] declaration; there is no way to build one directly.
pub struct Eeprom<I2C, D, const CAPACITY: u32, const PAGE_SIZE: usize> {
    i2c: I2C,
    delay: D,
    bus_address: u8,
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

impl<I2C, D, const CAPACITY: u32, const PAGE_SIZE: usize> Eeprom<I2C, D, CAPACITY, PAGE_SIZE> {
    /// Bind the driver to the part strapped to `chip_enable`.
    ///
    /// Not public API
    #[doc(hidden)]
    pub const fn __new(i2c: I2C, delay: D, chip_enable: ChipEnable) -> Self {
        Self {
            i2c,
            delay,
            bus_address: chip_enable.bus_address(),
        }
    }

    /// How many pages the array holds, which bounds any page indexed layout.
    pub const fn pages() -> usize {
        CAPACITY as usize / PAGE_SIZE
    }

    /// Bytes in one page, the unit everything atomic is counted in.
    pub const fn page_size() -> usize {
        PAGE_SIZE
    }

    fn check_bounds<E>(address: u32, len: usize) -> Result<(), Error<E>> {
        let len = len as u32;
        if address.checked_add(len).is_none_or(|end| end > CAPACITY) {
            return Err(Error::OutOfBounds {
                address,
                len,
                capacity: CAPACITY,
            });
        }
        Ok(())
    }
}

impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE_SIZE: usize>
    Eeprom<I2C, D, CAPACITY, PAGE_SIZE>
{
    /// Read `buf.len()` bytes from `address`.
    ///
    /// Sequential reads roll over at the end of the array rather than at a page
    /// boundary, so any length goes out as one transaction.
    pub(crate) async fn read(
        &mut self,
        address: u32,
        buf: &mut [u8],
    ) -> Result<(), Error<I2C::Error>> {
        Self::check_bounds(address, buf.len())?;
        if buf.is_empty() {
            return Ok(());
        }
        let addr_bytes = address_bytes(address);
        // Another store on the same part may be mid write cycle, during which this
        // one is NACKed rather than answered. Same retry as a write.
        let mut polls = 0u32;
        loop {
            match self
                .i2c
                .write_read(self.bus_address, &addr_bytes, buf)
                .await
            {
                Ok(()) => return Ok(()),
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => {
                    // Still NACKed after longer than a write cycle can last, so the
                    // part is not busy: nothing is answering at this address.
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

    /// Write `data` at `address`, splitting at page boundaries. Returns once every page has committed.
    pub(crate) async fn write(
        &mut self,
        address: u32,
        data: &[u8],
    ) -> Result<(), Error<I2C::Error>> {
        Self::check_bounds(address, data.len())?;
        for (addr, range) in page_writes(address, data.len(), PAGE_SIZE) {
            self.write_page(addr, &data[range]).await?;
        }
        Ok(())
    }

    async fn write_page(&mut self, address: u32, data: &[u8]) -> Result<(), Error<I2C::Error>> {
        debug_assert!(data.len() <= PAGE_SIZE);
        let addr_bytes = address_bytes(address);

        let mut polls = 0u32;
        loop {
            // Adjacent writes are sent with no stop or repeated start between them,
            // so this is one bus write without staging both into a buffer.
            let mut ops = [Operation::Write(&addr_bytes), Operation::Write(data)];
            match self.i2c.transaction(self.bus_address, &mut ops).await {
                Ok(()) => break,
                // A NACK means an earlier write is still committing and the part has
                // taken itself off the bus, so retrying doubles as the wait for it.
                Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => {
                    if polls >= WRITE_CYCLE_POLLS {
                        // NACKed for longer than a write cycle can last, so the part is
                        // not merely busy. Probe it with a one byte read to find out why.
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

        self.wait_ready().await
    }

    async fn wait_ready(&mut self) -> Result<(), Error<I2C::Error>> {
        // A one byte read rather than a zero length write: an address-only
        // transaction is not portable across HALs, and the byte is discarded.
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

impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE_SIZE: usize>
    embedded_storage_async::ReadStorage for Eeprom<I2C, D, CAPACITY, PAGE_SIZE>
{
    type Error = Error<I2C::Error>;

    fn capacity(&self) -> usize {
        CAPACITY as usize
    }

    async fn read(&mut self, address: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        Self::read(self, address, bytes).await
    }
}

impl<I2C: I2c, D: DelayNs, const CAPACITY: u32, const PAGE_SIZE: usize>
    embedded_storage_async::Storage for Eeprom<I2C, D, CAPACITY, PAGE_SIZE>
{
    async fn write(&mut self, address: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        Self::write(self, address, bytes).await
    }
}

/// Why a write was refused, given the result of a one byte read probe sent after the
/// write's retries ran out: a write protected part still acknowledges reads, an absent
/// one acknowledges nothing.
///
/// Relies on the part's behaviour rather than on the HAL naming the NACK's source,
/// which most HALs report as unknown.
fn classify_refused_write<E: embedded_hal::i2c::Error>(probe: Result<(), E>) -> Error<E> {
    match probe {
        Ok(()) => Error::WriteProtected,
        Err(err) if matches!(err.kind(), ErrorKind::NoAcknowledge(_)) => Error::NoDevice,
        Err(err) => Error::I2c(err),
    }
}

/// An address within the array, as the two bytes that carry it on the wire. Nothing
/// to do with [`ChipEnable`], which picks the device.
const fn address_bytes(address: u32) -> [u8; 2] {
    [(address >> 8u32) as u8, address as u8]
}

/// The page writes one access becomes: each item is a device address and the slice
/// of the caller's buffer that goes there.
///
/// A write crossing a page boundary does not continue into the next page: the
/// address wraps back to the start of the page it began in, corrupting what was
/// there. Reads have no such limit, so only writes are split.
const fn page_writes(address: u32, len: usize, page_size: usize) -> PageWrites {
    PageWrites {
        next: address,
        end: address + len as u32,
        base: address,
        page_size: page_size as u32,
    }
}

struct PageWrites {
    next: u32,
    end: u32,
    base: u32,
    page_size: u32,
}

impl Iterator for PageWrites {
    type Item = (u32, core::ops::Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }
        let page_end = (self.next / self.page_size + 1) * self.page_size;
        let run_end = page_end.min(self.end);
        let item = (
            self.next,
            (self.next - self.base) as usize..(run_end - self.base) as usize,
        );
        self.next = run_end;
        Some(item)
    }
}

const MAGIC: u32 = 0x1234_5678;

/// CRC-32/ISO-HDLC over `parts` in order, seeded with [`MAGIC`].
///
/// A caller folds in whatever else it wants bound to the value - a slot index, say -
/// by passing it as a leading part. None of it is stored.
pub(crate) fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = crc32_update(0xFFFF_FFFFu32, &MAGIC.to_le_bytes());
    let mut i = 0usize;
    while i < parts.len() {
        crc = crc32_update(crc, parts[i]);
        i += 1usize;
    }
    crc ^ 0xFFFF_FFFFu32
}

/// CRC-32/ISO-HDLC
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256usize {
        let mut crc = i as u32;
        let mut bit = 0u32;
        while bit < 8u32 {
            crc = if crc & 1u32 == 0u32 {
                crc >> 1u32
            } else {
                (crc >> 1u32) ^ 0xEDB8_8320u32
            };
            bit += 1u32;
        }
        table[i] = crc;
        i += 1usize;
    }
    table
};

/// Seed with `!0` and invert the result; feed the return value back in to carry on
/// over several slices.
const fn crc32_update(crc: u32, data: &[u8]) -> u32 {
    let mut crc = crc;
    let mut i = 0usize;
    while i < data.len() {
        let index = ((crc ^ data[i] as u32) & 0xFFu32) as usize;
        crc = (crc >> 8u32) ^ CRC32_TABLE[index];
        i += 1usize;
    }
    crc
}
