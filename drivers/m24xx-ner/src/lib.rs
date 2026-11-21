#![no_std]
//! Driver for the STMicroelectronics M24 series EEPROMs
//!
//! A part is read and written as whole pages through [`PagedMemory`]. A layout
//! declared with [`macro@eeprom_layout`] takes one over by value, so once it owns the
//! part there is no hand written page left to collide with the ones it manages.
//!
//! Async only: a page takes up to 5 ms to commit and the part holds the bus off for
//! all of it, which is too long to block an executor.
//!
//! Everything below is contract and nothing else: [`PagedMemory`] and [`Error`] between
//! the two halves - one module is the M24 part and the bus, the other the layout sitting
//! on it - and [`EepromLayout`], which is what the macro fills in.

// So the layout macro's `::m24xx_ner` paths resolve inside this crate too.
extern crate self as m24xx_ner;

pub use embedded_hal_async::delay::DelayNs;
pub use embedded_hal_async::i2c::I2c;

mod device;
mod schemes;
mod storable;
mod store;

pub use device::{AddressBits, Eeprom, M24C32, M24C64, M24128, M24256, M24512};
pub use schemes::{DoubleBuffered, MemoryScheme, PageSelector, Ring, RingBuffered, Unchecked};
pub use storable::{BitMask, Storable, bytes_for};
pub use store::{Access, Commit, Field};

/// Declare what lives on one part, and the only way to reach one.
///
/// ```ignore
/// use m24xx_ner::EepromLayout;
///
/// #[eeprom_layout(M24C32)]
/// struct Memory {
///     /// Kept across a commit that loses power.
///     boots: DoubleBuffered<u32>,
///     /// A page some other tool reads.
///     scratch: Unchecked<[u8; 32]>,
/// }
///
/// let eeprom = M24C32::new(i2c, Delay, AddressBits::GROUNDED);
/// let mut memory = Memory::attach(eeprom).await?;
///
/// let boots = memory.boots().read().await?.copied().unwrap_or(0);
/// memory.boots().commit(&(boots + 1)).await?;
/// ```
///
/// A field's type is its [`MemoryScheme`], and fields take the next free pages in
/// declaration order, so no page number is written down and two cannot collide. A field
/// too large for a page, or a layout too large for the part, fails the build.
///
/// Nothing spans pages, since a page is the most the part commits in one write cycle.
/// Data needing more room is several fields, split where its owner decides a tear is
/// survivable rather than wherever the page boundary falls.
///
/// Each field gets an accessor of the same name handing out an [`Access`], and keeps
/// its doc comment.
pub use m24xx_ner_macros::eeprom_layout;

/// Derive [`Storable`] for a struct whose every field is itself [`Storable`].
///
/// Use [`BitMask`] for a flag array; `[bool; N]` would cost a byte per flag.
///
/// ```ignore
/// #[derive(Clone, PartialEq, Storable)]
/// struct Record {
///     count: u16,
///     total: u32,
///     flags: BitMask<23>,
/// }
/// ```
pub use m24xx_ner_macros::Storable;

/// Not public API: the surface generated code is allowed to name, so a rename
/// elsewhere in this crate breaks here and not inside somebody's expanded layout.
#[doc(hidden)]
pub mod __private {
    pub use crate::store::fits;
    pub use crate::{DelayNs, I2c};
}

/// A failed access.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// The part accepted a write but never became ready again. Whether the page
    /// committed is not knowable from here.
    WriteTimeout,
    /// Nothing answered at the part's address. Check power, the chip enable strapping
    /// and the bus wiring.
    NoDevice,
    /// The part refused the data bytes, which is what holding write control (WC) high
    /// does.
    WriteProtected,
    /// The bus itself failed.
    I2c(E),
    /// What came back did not decode as what went out. The page written was not the
    /// live one, so the previous value still is.
    Verify,
    /// More bytes than a page holds, which no part commits in one write cycle.
    TooLong {
        /// How many bytes were asked for.
        len: usize,
    },
    /// A page past the end of the array.
    NoSuchPage {
        /// Page the caller asked for.
        page: usize,
    },
}

/// Memory addressed as whole pages, which is the only way an [`Access`] reaches it.
#[allow(async_fn_in_trait)]
pub trait PagedMemory {
    type BusError;

    async fn read_page(&mut self, page: usize, buf: &mut [u8])
    -> Result<(), Error<Self::BusError>>;

    /// Returns once the page has committed.
    async fn write_page(&mut self, page: usize, data: &[u8]) -> Result<(), Error<Self::BusError>>;
}

/// What [`macro@eeprom_layout`] builds: the fields of one part, and the only way to reach them.
///
/// An accessor per field also comes with the layout.
#[allow(async_fn_in_trait)]
pub trait EepromLayout: Sized {
    /// The part this layout was declared for.
    type Part: PagedMemory;

    /// Pages the layout occupies, from the start of the array.
    const PAGES: usize;

    /// Take `part` over, after which nothing else can write to it, and read every
    /// field off it. Needs the executor, so call it from a task, not from `main`.
    async fn attach(part: Self::Part)
    -> Result<Self, Error<<Self::Part as PagedMemory>::BusError>>;
}
