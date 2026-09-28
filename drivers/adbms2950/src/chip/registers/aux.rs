//! Register Layouts for the AUX ADC result groups.
//!
//! The AUX ADC measures the chip's own internal rails and its two temperature sensors, which is
//! what the supply and thermal diagnostics read. Note the three rails do not share one LSB:
//! `VREF1P25`, `VDIV`, and `EPAD` are 100 uV per code, `VREG` and `VDIG` are 240 uV (they are
//! measured through a divider), and `VDD` is 1 mV.
//!
//! The two temperature sensors use different formulas *and* different divisors from each other,
//! so `TMP1` and `TMP2` are distinct types rather than one shared one.
//!
//! For more info see Table 51 on page 38 of the datasheet (the result registers and their
//! scaling), Table 52 on page 39 (the per-command byte layouts), and Table 53 on page 39
//! (the oscillator counter).

use bitfield_struct::bitfield;

use super::super::commands;
use super::measurement;
use super::register_group;

/// Lowest `OSCCNT` value the datasheet considers a healthy oscillator.
///
/// See Table 53 on page 39 of the datasheet.
pub const OSCCNT_VALID_MIN: u8 = 0x34;

/// Highest `OSCCNT` value the datasheet considers a healthy oscillator.
pub const OSCCNT_VALID_MAX: u8 = 0x47;

/// Whether an `OSCCNT` reading is in the expected range.
///
/// A value outside `OSCCNT_VALID_MIN ..= OSCCNT_VALID_MAX` is what asserts the `OSCFLT` flag, and
/// the first such value is latched until `OSCFLT` is cleared.
pub const fn osccnt_is_valid(osccnt: u8) -> bool {
    osccnt >= OSCCNT_VALID_MIN && osccnt <= OSCCNT_VALID_MAX
}

/// AUX ADC results for VREF1P25, TMP1, and VREG, as returned by `RDXA`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::aux::rdxa().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct AuxA {
    /// AUX ADC VREF1P25 result (`VREF1P25`), bytes 0 and 1. 100 uV per code.
    #[bits(16, default = measurement::AuxVoltage::DEFAULT)]         pub vref1p25: measurement::AuxVoltage,
    /// AUX ADC temperature 1 result (`TMP1`), bytes 2 and 3. This is the die temperature.
    #[bits(16, default = measurement::Temperature1::DEFAULT)]       pub tmp1: measurement::Temperature1,
    /// AUX ADC VREG result (`VREG`), bytes 4 and 5. 240 uV per code.
    #[bits(16, default = measurement::AuxDividedVoltage::DEFAULT)]  pub vreg: measurement::AuxDividedVoltage,
    #[bits(16, default = 0)]                                        _padding: u16,
}

/// AUX ADC results for VDD, VDIG, and EPAD, as returned by `RDXB`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::aux::rdxb().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct AuxB {
    /// AUX ADC VDD result (`VDD`), bytes 0 and 1. 1 mV per code, unlike every other rail here.
    #[bits(16, default = measurement::SupplyVoltage::DEFAULT)]      pub vdd: measurement::SupplyVoltage,
    /// AUX ADC VDIG result (`VDIG`), bytes 2 and 3. 240 uV per code.
    #[bits(16, default = measurement::AuxDividedVoltage::DEFAULT)]  pub vdig: measurement::AuxDividedVoltage,
    /// AUX ADC EPAD result (`EPAD`), bytes 4 and 5. 100 uV per code.
    #[bits(16, default = measurement::AuxVoltage::DEFAULT)]         pub epad: measurement::AuxVoltage,
    #[bits(16, default = 0)]                                        _padding: u16,
}

/// AUX ADC results for VDIV and TMP2 plus the oscillator counter, as returned by `RDXC`.
///
/// Byte 4 is unused and reads back as `0xFF`.
#[rustfmt::skip]
#[register_group(bytes = 6, write = None, read = Some(commands::aux::rdxc().frame()))]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct AuxC {
    /// AUX ADC VDIV result (`VDIV`), bytes 0 and 1. 100 uV per code.
    ///
    /// This tracks the nominal VREF1 through the VREF1-to-VDIV ratio in Table 7, so it varies
    /// between individual parts.
    #[bits(16, default = measurement::AuxVoltage::DEFAULT)]    pub vdiv: measurement::AuxVoltage,
    /// AUX ADC temperature 2 result (`TMP2`), bytes 2 and 3.
    #[bits(16, default = measurement::Temperature2::DEFAULT)]  pub tmp2: measurement::Temperature2,
    /// Unused by this command; reads back as `0xFF`.
    #[bits(8, default = 0xFF)]                                 _unused: u8,
    /// Oscillator counter (`OSCCNT`), byte 5.
    ///
    /// The most recently counted OSC1 clocks within one OSC2 half period. See `osccnt_is_valid`
    /// for the healthy range; on an `OSCFLT` event the first invalid count is latched here until
    /// the flag is cleared.
    #[bits(8, default = 0)]                                    pub osccnt: u8,
    #[bits(16, default = 0)]                                   _padding: u16,
}
