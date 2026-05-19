//! Typed register layouts for the ADBMS6830.
//!
//! Each register is one 6-byte payload per IC. We back the type with a `u64`
//! via the `bitfield-struct` crate (the smallest primitive that covers 48
//! bits) and bridge to the chain transport via `From<u64>` / `Into<u64>`
//! plus the [`AdbmsReadableRegister`] / [`AdbmsWritableRegister`] tags in
//! [`crate::registers`].
//!
//! `bitfield-struct` lays out fields LSB-first by default, so the first
//! declared field occupies bit 0 of the backing `u64`. Combined with
//! `to_le_bytes`, this puts byte 0 of the wire payload at the LSB side of
//! the integer, which matches the ADBMS6830 datasheet ordering. The top 16
//! bits of the `u64` are an `__: u16` padding slot and never reach the wire.

use bitfield_struct::bitfield;

use crate::registers::{AdbmsReadableRegister, AdbmsWritableRegister};

/// Configuration Register Group A (`CFGAR0..CFGAR5`).
///
/// Wire layout (bit 7 = MSB of each byte):
/// ```text
/// byte 0:  refon   | -- | -- | -- | -- | cth[2] | cth[1] | cth[0]
/// byte 1:  flag_d[7:0]
/// byte 2:  soakon  | owrng | owa[2] | owa[1] | owa[0] | -- | -- | --
/// byte 3:  gpo[7:0]
/// byte 4:  -- | -- | -- | -- | -- | -- | gpo[9] | gpo[8]
/// byte 5:  -- | -- | snap | mute_st | comm_bk | fc[2] | fc[1] | fc[0]
/// ```
///
/// Build with [`ConfigA::new`] + chained `with_*` setters, or one of the
/// curated presets / helpers in the `impl ConfigA` block below.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct ConfigA {
    // ---- byte 0 ----
    #[bits(3)]
    pub cth: u8,
    #[bits(4)]
    __: u8,
    pub refon: bool,
    // ---- byte 1 ----
    pub flag_d: u8,
    // ---- byte 2 ----
    #[bits(3)]
    __: u8,
    #[bits(3)]
    pub owa: u8,
    pub owrng: bool,
    pub soakon: bool,
    // ---- bytes 3 & 4: gpo straddles the boundary (low 8 in byte 3, high 2
    // in byte 4 bits 1:0) ----
    #[bits(10)]
    pub gpo: u16,
    #[bits(6)]
    __: u8,
    // ---- byte 5 ----
    #[bits(3)]
    pub fc: IirCorner,
    /// test 1234
    pub comm_bk: bool,
    pub mute_st: bool,
    pub snap: bool,
    #[bits(2)]
    __: u8,
    // ---- bytes 6 & 7 of the u64 are not on the wire ----
    #[bits(16)]
    __: u16,
}

// `bitfield-struct` auto-generates `From<u64>` for `ConfigA` and
// `From<ConfigA>` for `u64`, so the trait bounds are already satisfied —
// we only need to tag the register with its command codes.
impl AdbmsWritableRegister for ConfigA {
    const WRITE_CMD: [u8; 2] = [0x00, 0x01];
}
impl AdbmsReadableRegister for ConfigA {
    const READ_CMD: [u8; 2] = [0x00, 0x02];
}

// ---- hand-written ergonomics: validated field enums, curated presets ----

/// IIR filter corner frequency (3-bit `fc` field of [`ConfigA`]).
///
/// Lives inside the bitfield directly — `bitfield-struct` consumes the
/// `const from_bits` / `into_bits` pair below to round-trip the 3-bit code
/// without the call site ever seeing a raw `u8`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IirCorner {
    Disabled = 0b000,
    Hz4400 = 0b001,
    Hz2200 = 0b010,
    Hz1100 = 0b011,
    Hz550 = 0b100,
    Hz275 = 0b101,
    Hz138 = 0b110,
    Hz69 = 0b111,
}

impl IirCorner {
    const fn into_bits(self) -> u8 {
        self as u8
    }
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Disabled,
            1 => Self::Hz4400,
            2 => Self::Hz2200,
            3 => Self::Hz1100,
            4 => Self::Hz550,
            5 => Self::Hz275,
            6 => Self::Hz138,
            // bitfield-struct masks the input to the declared bit width, so
            // 7 is the only remaining value.
            _ => Self::Hz69,
        }
    }
}

impl ConfigA {
    /// Power-on style preset: reference always on, IIR disabled, no GPO
    /// drive, all flags cleared. Suitable as the very first config after
    /// chain wakeup; tighten from here.
    pub const fn boot_default() -> Self {
        Self::new().with_refon(true)
    }
}
