//! Register Layouts for the V1ADC and V2ADC voltage result groups.
//!
//! The chip has ten general-purpose voltage inputs measured by two independent ADCs, which is
//! what makes the voltage path redundant: the V1ADC covers V1 through V8 and the V2ADC covers
//! V1 through V6 plus V9 and V10, so V1 through V6 are measured twice and can be compared.
//!
//! Two quirks of these groups, both straight from Table 52:
//!
//! 1. **The V2ADC path has inverted gain and a different LSB** -- `-85 uV` per code against the
//!    V1ADC's `+100 uV`. So `V1A` and `V1B` measure the same pin but do not produce comparable
//!    raw codes; convert both with `as_microvolts()` before comparing.
//! 2. **Several commands return overlapping views.** `RDV1D` repeats `V7A` and `V8A` from
//!    `RDV1C` but substitutes `V9B` for `VREF2A`, and `RDV2D` repeats `V10B` and `VREF2B` from
//!    `RDV2C`. These are alternative reads of the same registers, not extra registers.
//!
//! For more info see Table 51 on page 38 of the datasheet (the result registers and their
//! scaling) and Table 52 on page 39 (the per-command byte layouts).

use bitfield_struct::bitfield;

use super::super::commands;
use super::measurement;
use super::register_group;

/// V1ADC results for V1, V2, and V3, as returned by `RDV1A`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::voltage::rdv1a().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages1A {
    /// V1ADC V1 result (`V1A`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v1a: measurement::VoltageA,
    /// V1ADC V2 result (`V2A`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v2a: measurement::VoltageA,
    /// V1ADC V3 result (`V3A`), bytes 4 and 5.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v3a: measurement::VoltageA,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V1ADC results for V4, V5, and V6, as returned by `RDV1B`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::voltage::rdv1b().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages1B {
    /// V1ADC V4 result (`V4A`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v4a: measurement::VoltageA,
    /// V1ADC V5 result (`V5A`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v5a: measurement::VoltageA,
    /// V1ADC V6 result (`V6A`), bytes 4 and 5.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v6a: measurement::VoltageA,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V1ADC results for V7 and V8 plus the V1ADC's VREF2, as returned by `RDV1C`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::voltage::rdv1c().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages1C {
    /// V1ADC V7 result (`V7A`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v7a: measurement::VoltageA,
    /// V1ADC V8 result (`V8A`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v8a: measurement::VoltageA,
    /// V1ADC VREF2 result (`VREF2A`), bytes 4 and 5. Scaled by the VREF2 divider, so 240 uV per
    /// code rather than 100 uV.
    #[bits(16, default = measurement::Vref2A::DEFAULT)]    pub vref2a: measurement::Vref2A,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V1ADC results for V7 and V8 plus the V2ADC's V9, as returned by `RDV1D`.
///
/// Note the mixed ADC paths: `v7a` and `v8a` are the same registers `RDV1C` returns, but `v9b`
/// comes from the V2ADC and so carries the inverted `-85 uV` LSB.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::voltage::rdv1d().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages1D {
    /// V1ADC V7 result (`V7A`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v7a: measurement::VoltageA,
    /// V1ADC V8 result (`V8A`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageA::DEFAULT)]  pub v8a: measurement::VoltageA,
    /// V2ADC V9 result (`V9B`), bytes 4 and 5.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v9b: measurement::VoltageB,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V2ADC results for V1, V2, and V3, as returned by `RDV2A`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::redundant_voltage::rdv2a().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages2A {
    /// V2ADC V1 result (`V1B`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v1b: measurement::VoltageB,
    /// V2ADC V2 result (`V2B`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v2b: measurement::VoltageB,
    /// V2ADC V3 result (`V3B`), bytes 4 and 5.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v3b: measurement::VoltageB,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V2ADC results for V4, V5, and V6, as returned by `RDV2B`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::redundant_voltage::rdv2b().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages2B {
    /// V2ADC V4 result (`V4B`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v4b: measurement::VoltageB,
    /// V2ADC V5 result (`V5B`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v5b: measurement::VoltageB,
    /// V2ADC V6 result (`V6B`), bytes 4 and 5.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v6b: measurement::VoltageB,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V2ADC results for V9 and V10 plus the V2ADC's VREF2, as returned by `RDV2C`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::redundant_voltage::rdv2c().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages2C {
    /// V2ADC V9 result (`V9B`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v9b: measurement::VoltageB,
    /// V2ADC V10 result (`V10B`), bytes 2 and 3.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v10b: measurement::VoltageB,
    /// V2ADC VREF2 result (`VREF2B`), bytes 4 and 5. Scaled by the VREF2 divider, so -204 uV per
    /// code.
    #[bits(16, default = measurement::Vref2B::DEFAULT)]    pub vref2b: measurement::Vref2B,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V2ADC V10 plus both VREF2 results, as returned by `RDV2D`.
///
/// This is the one group that returns both ADCs' VREF2 registers together, which is what makes it
/// useful for cross-checking the two reference paths against each other.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::redundant_voltage::rdv2d().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages2D {
    /// V2ADC V10 result (`V10B`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v10b: measurement::VoltageB,
    /// V1ADC VREF2 result (`VREF2A`), bytes 2 and 3.
    #[bits(16, default = measurement::Vref2A::DEFAULT)]    pub vref2a: measurement::Vref2A,
    /// V2ADC VREF2 result (`VREF2B`), bytes 4 and 5.
    #[bits(16, default = measurement::Vref2B::DEFAULT)]    pub vref2b: measurement::Vref2B,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// V2ADC V10 alone, as returned by `RDV2E`.
///
/// Only bytes 0 and 1 carry data; bytes 2 through 5 read back as `0xFF`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::redundant_voltage::rdv2e().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Voltages2E {
    /// V2ADC V10 result (`V10B`), bytes 0 and 1.
    #[bits(16, default = measurement::VoltageB::DEFAULT)]  pub v10b: measurement::VoltageB,
    /// Unused by this command; reads back as `0xFF` in all four bytes.
    #[bits(32, default = 0xFFFF_FFFF)]                     _unused: u32,
    #[bits(16, default = 0)]                               _padding: u16,
}
