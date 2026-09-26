//! Register Layouts and Bit Descriptions for the STAT Register Group.
//!
//! The "main" struct here (i.e., the struct representing the overall register group) is `Status`.
//!
//! This group is entirely read-only: ADC initialization status, the OCA/OCB pin readbacks, the
//! GPIO and GPO level readbacks, and the part's derivative and revision identifiers.
//!
//! Note the GPO readbacks are laid out awkwardly: `GPO1L` through `GPO4L` live in byte 4 while
//! `GPO5L` and `GPO6L` sit in byte 3 below the `GPOxH` bits. Each is only one bit, so no field
//! spans a gap, but the declaration order below follows the wire rather than the pin numbering.
//!
//! For more info about these registers, see Table 75 on page 61 of the datasheet
//! (the register map) and Table 77 on pages 62 and 63 of the datasheet (the bit descriptions).

use adbms_register_macros::BitfieldEnumDefault;
use bitfield_struct::{bitenum, bitfield};

use super::super::commands;
use super::register_group;

/// Field types relevant to the STAT register group. See Table 77 on page 62 of the datasheet.
pub mod types {
    use super::{BitfieldEnumDefault, bitenum};

    /// Device revision identifier (REVID) values. See Table 77 on page 62 of the datasheet.
    ///
    /// This is deliberately not the type of the `revid` field, because the datasheet only assigns
    /// four of the sixteen codes and silently coercing an unknown revision into a known one would
    /// be worse than reporting it verbatim. Use `Status::revision()` to get one of these.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Revision {
        /// Rev B.
        B,
        /// Rev C.
        C,
        /// Rev D.
        D,
        /// Rev E.
        E,
    }
    impl Revision {
        /// Maps a raw four-bit `REVID` code to a revision, or `None` if the datasheet does not
        /// assign that code.
        pub const fn from_code(code: u8) -> Option<Self> {
            match code {
                0b0001 => Some(Self::B),
                0b0010 => Some(Self::C),
                0b0011 => Some(Self::D),
                0b0100 => Some(Self::E),
                _ => None,
            }
        }
    }

    /// Derivative code (DER). Two-bit field. Read only.
    ///
    /// Only `0b00` is assigned; every other code is documented as invalid, which is what the
    /// fallback covers.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Derivative {
        /// ADBMS2950B (default).
        #[default]
        Adbms2950b = 0b00,
        /// Not a valid derivative code.
        #[fallback]
        Invalid = 0b01,
    }

    /// ADC initialization status (I1CAL, I2CAL). One-bit field. Read only.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum InitializationStatus {
        /// The ADC has completed initialization.
        Complete = 1,
        /// The ADC has not yet completed initialization (default).
        #[default]
        #[fallback]
        Incomplete = 0,
    }
}

/// STAT Register Group.
///
/// See Table 75 on page 61 of the datasheet for the register map, and Table 77 on pages 62 and 63
/// for the bit descriptions.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::status::rdstat().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Status {
    // STAT0! first byte of the register group.
    /// OCA pin state (OCAP), meaningful when `OCMODE` is static or for diagnostics. `true` means
    /// the pin reads back high. Read only; the reset value is undefined.
    #[bits(1, default = false)]  pub ocap: bool,
    /// OCB pin state (OCBP), meaningful when `OCMODE` is static or for diagnostics. `true` means
    /// the pin reads back high. Read only; the reset value is undefined.
    #[bits(1, default = false)]  pub ocbp: bool,
    #[bits(6, default = 0)]      _reserved0: u8,
    // STAT1! second byte of the register group.
    /// Derivative code (DER). Two-bit field. Read only.
    #[bits(2, default = types::Derivative::DEFAULT)]           pub der: types::Derivative,
    #[bits(4, default = 0)]                                    _reserved1: u8,
    /// I1ADC initialization status (I1CAL). One-bit field. Read only.
    #[bits(1, default = types::InitializationStatus::DEFAULT)] pub i1cal: types::InitializationStatus,
    /// I2ADC initialization status (I2CAL). One-bit field. Read only.
    #[bits(1, default = types::InitializationStatus::DEFAULT)] pub i2cal: types::InitializationStatus,
    // STAT2! third byte of the register group. Entirely reserved.
    #[bits(8, default = 0)]      _reserved2: u8,
    // STAT3! fourth byte of the register group.
    /// GPO5 low-level readback (GPO5L). `false` means GPO5 reads back low. Read only.
    #[bits(1, default = true)]   pub gpo5l: bool,
    /// GPO6 low-level readback (GPO6L). `false` means GPO6 reads back low. Read only.
    #[bits(1, default = true)]   pub gpo6l: bool,
    /// GPO1 high-level readback (GPO1H). Read only.
    #[bits(1, default = true)]   pub gpo1h: bool,
    /// GPO2 high-level readback (GPO2H). Read only.
    #[bits(1, default = true)]   pub gpo2h: bool,
    /// GPO3 high-level readback (GPO3H). Read only.
    #[bits(1, default = true)]   pub gpo3h: bool,
    /// GPO4 high-level readback (GPO4H). Read only.
    #[bits(1, default = true)]   pub gpo4h: bool,
    /// GPO5 high-level readback (GPO5H). Read only.
    #[bits(1, default = true)]   pub gpo5h: bool,
    /// GPO6 high-level readback (GPO6H). Read only.
    #[bits(1, default = true)]   pub gpo6h: bool,
    // STAT4! fifth byte of the register group.
    /// GPIO1 readback (GPIO1L). `false` means GPIO1 reads back low. Read only.
    #[bits(1, default = true)]   pub gpio1l: bool,
    /// GPIO2 readback (GPIO2L). Read only.
    #[bits(1, default = true)]   pub gpio2l: bool,
    /// GPIO3 readback (GPIO3L). Read only.
    #[bits(1, default = true)]   pub gpio3l: bool,
    /// GPIO4 readback (GPIO4L). Read only.
    #[bits(1, default = true)]   pub gpio4l: bool,
    /// GPO1 low-level readback (GPO1L). Read only.
    #[bits(1, default = true)]   pub gpo1l: bool,
    /// GPO2 low-level readback (GPO2L). Read only.
    #[bits(1, default = true)]   pub gpo2l: bool,
    /// GPO3 low-level readback (GPO3L). Read only.
    #[bits(1, default = true)]   pub gpo3l: bool,
    /// GPO4 low-level readback (GPO4L). Read only.
    #[bits(1, default = true)]   pub gpo4l: bool,
    // STAT5! sixth byte of the register group.
    #[bits(4, default = 0)]      _reserved3: u8,
    /// Device revision identifier (REVID). Four-bit field. Read only.
    ///
    /// Raw code; use `revision()` to interpret it. The datasheet assigns `0b0001` through
    /// `0b0100` only.
    #[bits(4, default = 0)]      pub revid: u8,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]     _padding: u16,
}

impl Status {
    /// The device revision, or `None` if `REVID` holds a code the datasheet doesn't assign.
    pub const fn revision(&self) -> Option<types::Revision> {
        types::Revision::from_code(self.revid())
    }
}
