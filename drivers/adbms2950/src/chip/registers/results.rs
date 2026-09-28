//! Register Layouts for the current, battery voltage, accumulator, and overcurrent result groups.
//!
//! These are all read-only. The scaling lives on the field types in
//! [`super::measurement`]; this module is just the byte layouts, which come from the "Return
//! Values" tables rather than from register maps (the result registers are addressed by command,
//! and several commands return overlapping views of the same underlying registers).
//!
//! Three things worth knowing before reading a value out of here:
//!
//! 1. **The second channel of every pair has inverted gain.** `I2` is `-(code * 1 uV)` where `I1`
//!    is `+(code * 1 uV)`, and `VB2` is `-(code * 85 uV)` where `VB1` is `+(code * 100 uV)` --
//!    note the LSB *magnitudes* differ too, not just the signs.
//! 2. **The accumulators hold sums, not averages.** Divide by `ACCN = 4 * (ACCI + 1)` from CFGA;
//!    see `super::measurement::accumulation_count`.
//! 3. **The overcurrent LSB is configurable**, so those results can only be converted to volts
//!    with the matching `OCxGC` gain bit from CFGB in hand.
//!
//! For more info see Table 43 on page 32 (current and battery result registers), Table 44 on
//! page 33 (accumulators), Table 45 on page 33 (their return values), and Table 63 on page 51,
//! Table 64 on page 51, and Table 65 on page 51 (overcurrent) of the datasheet.

use bitfield_struct::bitfield;

use super::super::commands;
use super::measurement;
use super::register_group;

/// Field types specific to the overcurrent result registers.
pub mod types {
    use super::super::config_b::types::OverCurrentGain;
    use super::bitfield;

    /// An OCxADC result code (`OC1R`, `OC2R`, `OC3R`, `OC3MAX`, `OC3MIN`). Signed 8-bit.
    ///
    /// Unlike every other result register on this chip the LSB is not fixed: it is 5 mV at
    /// `OverCurrentGain::Gain1` and 2.5 mV at `Gain2`, per `OCxGC` in Table 72. So converting to
    /// volts needs the gain bit from CFGB, which is why `as_microvolts` takes it as a parameter
    /// rather than this being a plain newtype with a constant.
    #[bitfield(u8, defmt = cfg(feature = "defmt"))]
    #[derive(PartialEq, Eq)]
    pub struct OverCurrentCode {
        /// The raw signed register code.
        #[bits(8)]
        pub raw: i8,
    }
    impl OverCurrentCode {
        /// Microvolts per code at `OverCurrentGain::Gain1`.
        pub const LSB_MICROVOLTS_GAIN1: i32 = 5_000;
        /// Microvolts per code at `OverCurrentGain::Gain2`.
        pub const LSB_MICROVOLTS_GAIN2: i32 = 2_500;
        /// The value the result registers hold after a reset (`0x7F`).
        pub const RESET: Self = Self::from_bits(0x7F);
        /// The value `OC1R`..`OC3R` and `OC3MAX` hold after clearing, and `OC3MIN` after reset
        /// (`0x80`).
        pub const CLEARED: Self = Self::from_bits(0x80u8);

        /// The measured voltage in microvolts, for the given channel gain.
        pub const fn as_microvolts(self, gain: OverCurrentGain) -> i32 {
            let lsb = match gain {
                OverCurrentGain::Gain1 => Self::LSB_MICROVOLTS_GAIN1,
                OverCurrentGain::Gain2 => Self::LSB_MICROVOLTS_GAIN2,
            };
            self.raw() as i32 * lsb
        }
    }
}

/// I1ADC and I2ADC current results, as returned by `RDI`.
///
/// See Table 45 on page 33 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::current::rdi().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Currents {
    /// I1ADC result (`I1[23:0]`), bytes 0 through 2.
    #[bits(24, default = measurement::Current1::DEFAULT)]  pub i1: measurement::Current1,
    /// I2ADC result (`I2[23:0]`), bytes 3 through 5. Inverted gain relative to `i1`.
    #[bits(24, default = measurement::Current2::DEFAULT)]  pub i2: measurement::Current2,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]  _padding: u16,
}

/// VB1ADC and VB2ADC battery voltage results, as returned by `RDVB`.
///
/// Bytes 0 and 1 are not used by this command and read back as `0xFF`.
/// See Table 45 on page 33 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::current::rdvb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct BatteryVoltages {
    /// Unused by this command; reads back as `0xFF` in both bytes.
    #[bits(16, default = 0xFFFF)]  _unused: u16,
    /// VB1ADC result (`VB1[15:0]`), bytes 2 and 3.
    #[bits(16, default = measurement::BatteryVoltage1::DEFAULT)]       pub vb1: measurement::BatteryVoltage1,
    /// VB2ADC result (`VB2[15:0]`), bytes 4 and 5. Inverted gain relative to `vb1`.
    #[bits(16, default = measurement::BatteryVoltage2::DEFAULT)]       pub vb2: measurement::BatteryVoltage2,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]       _padding: u16,
}

/// I1ADC current plus VB1ADC battery voltage, as returned by `RDIVB1`.
///
/// This is the coherent single-command read of the primary current and voltage channels, which
/// is what the production measurement loop wants. Byte 3 is unused and reads back as `0xFF`.
/// See Table 45 on page 33 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::current::rdivb1().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct CurrentAndBatteryVoltage {
    /// I1ADC result (`I1[23:0]`), bytes 0 through 2.
    #[bits(24, default = measurement::Current1::DEFAULT)]     pub i1: measurement::Current1,
    /// Unused by this command; reads back as `0xFF`.
    #[bits(8, default = 0xFF)]   _unused: u8,
    /// VB1ADC result (`VB1[15:0]`), bytes 4 and 5.
    #[bits(16, default = measurement::BatteryVoltage1::DEFAULT)]     pub vb1: measurement::BatteryVoltage1,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]     _padding: u16,
}

/// I1ACC and I2ACC accumulated current results, as returned by `RDIACC`.
///
/// These are sums of `ACCN` samples, not averages. See Table 45 on page 33 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::accumulated::rdiacc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct AccumulatedCurrents {
    /// I1ADC accumulator (`I1ACC[23:0]`), bytes 0 through 2.
    #[bits(24, default = measurement::AccumulatedCurrent1::DEFAULT)]  pub i1acc: measurement::AccumulatedCurrent1,
    /// I2ADC accumulator (`I2ACC[23:0]`), bytes 3 through 5. Inverted gain.
    #[bits(24, default = measurement::AccumulatedCurrent2::DEFAULT)]  pub i2acc: measurement::AccumulatedCurrent2,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]  _padding: u16,
}

/// VB1ACC and VB2ACC accumulated battery voltage results, as returned by `RDVBACC`.
///
/// These are sums of `ACCN` samples, not averages. See Table 45 on page 33 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::accumulated::rdvbacc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct AccumulatedBatteryVoltages {
    /// VB1ADC accumulator (`VB1ACC[23:0]`), bytes 0 through 2.
    #[bits(24, default = measurement::AccumulatedBatteryVoltage1::DEFAULT)]  pub vb1acc: measurement::AccumulatedBatteryVoltage1,
    /// VB2ADC accumulator (`VB2ACC[23:0]`), bytes 3 through 5. Inverted gain.
    #[bits(24, default = measurement::AccumulatedBatteryVoltage2::DEFAULT)]  pub vb2acc: measurement::AccumulatedBatteryVoltage2,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]  _padding: u16,
}

/// I1ACC plus VB1ACC, as returned by `RDIVB1ACC`.
///
/// The accumulator counterpart to `CurrentAndBatteryVoltage`, and what the production loop uses
/// for coulomb counting. See Table 45 on page 33 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::accumulated::rdivb1acc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct AccumulatedCurrentAndBatteryVoltage {
    /// I1ADC accumulator (`I1ACC[23:0]`), bytes 0 through 2.
    #[bits(24, default = measurement::AccumulatedCurrent1::DEFAULT)]  pub i1acc: measurement::AccumulatedCurrent1,
    /// VB1ADC accumulator (`VB1ACC[23:0]`), bytes 3 through 5.
    #[bits(24, default = measurement::AccumulatedBatteryVoltage1::DEFAULT)]  pub vb1acc: measurement::AccumulatedBatteryVoltage1,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]  _padding: u16,
}

/// Overcurrent comparator results and the OC3 extremes, as returned by `RDOC`.
///
/// Byte 3 is reserved. Note `oc3min` resets to `0x80` and clears to `0x7F` -- the opposite of
/// every other register here -- because it tracks a running minimum.
/// See Table 63 on page 51 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::current::rdoc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct OverCurrentResults {
    /// OC1ADC result (`OC1R`). Scale with the `oc1gc` gain from CFGB.
    #[bits(8, default = types::OverCurrentCode::RESET)]  pub oc1r: types::OverCurrentCode,
    /// OC2ADC result (`OC2R`). Scale with the `oc2gc` gain from CFGB.
    #[bits(8, default = types::OverCurrentCode::RESET)]  pub oc2r: types::OverCurrentCode,
    /// OC3ADC result (`OC3R`). Scale with the `oc3gc` gain from CFGB.
    #[bits(8, default = types::OverCurrentCode::RESET)]  pub oc3r: types::OverCurrentCode,
    /// Reserved.
    #[bits(8, default = 0)]     _reserved: u8,
    /// Running maximum of the OC3ADC conversions (`OC3MAX`). Cleared by the `OCAGD/CLRM` bit of
    /// a `CLRFLAG` write, not by `CLRO`.
    #[bits(8, default = types::OverCurrentCode::RESET)]  pub oc3max: types::OverCurrentCode,
    /// Running minimum of the OC3ADC conversions (`OC3MIN`). Resets to `0x80` and clears to
    /// `0x7F`, inverted relative to `oc3max`.
    #[bits(8, default = types::OverCurrentCode::CLEARED)]  pub oc3min: types::OverCurrentCode,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]    _padding: u16,
}
