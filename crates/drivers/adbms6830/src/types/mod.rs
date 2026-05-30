//! Typed register layouts for the ADBMS6830.

//  Each on-the-wire register group is one 6-byte payload per IC. The
//  types in here back that payload with a `u64` via the `bitfield-struct`
//  crate (smallest primitive that covers 48 bits) and bridge to the
//  chain transport via `From<u64>` / `Into<u64>` plus the
//  [`AdbmsReadableRegister`] / [`AdbmsWritableRegister`] tags in
//  [`crate::registers`].
//
//  `bitfield-struct` lays out fields LSB-first by default, so the first
//  declared field occupies bit 0 of the backing `u64`. Combined with
//  `to_le_bytes`, this puts byte 0 of the wire payload at the LSB side of
//  the integer, which matches the ADBMS6830 datasheet ordering. The top
//  16 bits of each `u64` are anonymous (`__`) padding and never reach
//  the wire.
//
//  [`AdbmsReadableRegister`]: crate::registers::AdbmsReadableRegister
//  [`AdbmsWritableRegister`]: crate::registers::AdbmsWritableRegister

pub mod aux;
pub mod comm;
pub mod config;
pub mod misc;
pub mod pwm;
pub mod status;
pub mod voltage;

// Re-export every register type and shared scalar so callers don't need
// to know which sub-module a given register lives in.
pub use aux::*;
pub use comm::*;
pub use config::*;
pub use misc::*;
pub use pwm::*;
pub use status::*;
pub use voltage::*;

/// Cell-format voltage measurement from a variety of sources.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct AdbmsCellVoltage(pub u16);

impl AdbmsCellVoltage {
    pub const fn raw(self) -> u16 {
        self.0
    }
    /// `0x8000` is what the chip returns after power-up and after a `CLR*`
    /// command — i.e. "no valid measurement yet."
    pub const fn is_cleared(self) -> bool {
        self.0 == 0x8000
    }
    pub fn volts(self) -> f32 {
        (self.0 as i16) as f32 * 0.00015 + 1.5
    }

    pub const fn into_bits(self) -> u16 {
        self.0
    }
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

/// 12-bit signed comparison threshold.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Threshold(pub u16);

impl Threshold {
    pub const fn raw(self) -> u16 {
        self.0 & 0x0FFF
    }
    pub const fn into_volts(self) -> f32 {
        self.raw() as f32 * 0.0024 + 1.5
    }

    /// Creates a voltage threshold from voltage input
    pub fn from_volts(voltage: f32) -> Self {
        let raw = ((voltage - 1.5) / 0.0024) as u16;
        Threshold(raw)
    }

    pub const fn into_bits(self) -> u16 {
        self.0 & 0x0FFF
    }
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

/// Chip voltage reading from a variety of sources.
///
/// 16-bit signed ADC code with LSB = 3.75 mV, offset = 37.5 V. The reading
/// represents `25 × (raw × 150 µV + 1.5 V)` — i.e. the internal divider
/// down by 25× is undone by the LSB. Sentinel `0x8000` after clear.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct AdbmsChipVoltage(pub u16);

impl AdbmsChipVoltage {
    pub const fn raw(self) -> u16 {
        self.0
    }
    pub const fn is_cleared(self) -> bool {
        self.0 == 0x8000
    }
    pub fn volts(self) -> f32 {
        (self.0 as i16) as f32 * 0.00375 + 37.5
    }

    pub const fn into_bits(self) -> u16 {
        self.0
    }
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

/// Internal die temperature
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct DieTemp(pub u16);

impl DieTemp {
    pub const fn raw(self) -> u16 {
        self.0
    }
    /// True after `CLRAUX`.
    pub const fn is_cleared(self) -> bool {
        self.0 == 0x8000
    }
    pub fn celsius(self) -> f32 {
        (self.0 as i16) as f32 * 0.02 - 73.0
    }

    pub const fn into_bits(self) -> u16 {
        self.0
    }
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}
