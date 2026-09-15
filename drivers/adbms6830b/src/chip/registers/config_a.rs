//! Register Layouts and Bit Descriptions for Configuration Register Group A. 
//! 
//! The "main" struct here (i.e., the struct representing the overall register group) is `ConfigA`.
//! `ConfigA` can be translated into full protocol frames (for sending) via `ConfigAReadRequest`, `ConfigAReadResponse`, and `ConfigAWriteFrame`.
//! 
//! For more info about these registers, see Table 102 on page 70 of the datasheet
//! and Table 55 on page 61 of the datasheet.

use bitfield_struct::{bitfield, bitenum};
use adbms6830b_macros::BitfieldEnumDefault;

use super::register_group;
use super::super::commands;

/// Field types relavent to Configuration Register A. See Table 102 on page 70 of the datasheet.
pub mod types {
    use super::{bitenum, BitfieldEnumDefault};

    /// Reference powered up (REFON). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum ReferenceOn {
        /// Reference remains powered up until watchdog timeout.
        On = 1,
        /// Reference shuts down after conversions (default).
        #[default]
        #[fallback]
        Off = 0,
    }

    /// C-ADC vs. S-ADC comparison voltage theshold. Three-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum ComparisonThresholdVoltage {
        /// 5.1 mV
        Mv5_1 = 0b000,
        /// 8.1 mV (default)
        #[default]
        #[fallback]
        Mv8_1 = 0b001,
        /// 9.0 mV
        Mv9_0 = 0b010,
        /// 10.05 mV
        Mv10_05 = 0b011,
        /// 15.00 mV
        Mv15_00 = 0b100,
        /// 19.95 mV
        Mv19_95 = 0b101,
        /// 25.05 mV
        Mv25_05 = 0b110,
        /// 40.05 mV
        Mv40_05 = 0b111,
    }

    /// Enable/disable soak time on AUX ADCs (SOAKON). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SoakTimeOn {
        /// Enables soak time for all commands.
        On = 1,
        /// Disables soak time (default).
        #[default]
        #[fallback]
        Off = 0,
    }

    /// Soak time range (OWRNG). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SoakTimeRange {
        /// Long soak time range. 4.1ms to 524ms.
        Long = 1,
        /// Short soak time range (default). 32us to 4.1ms.
        #[default]
        #[fallback]
        Short = 0,
    }

    /// Open wire soak times, for AUX commands (OWA). Three-bit field.
    /// 
    /// This is basically a multiplier for the "base" soak time, where `base` = 2^6 clocks
    /// for `OWRNG=0`, or 2^13 clocks for `OWRNG=1`.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OpenWireSoakTimeMultiplier {
        /// 1x base soak time. 32us for OWRNG=0, 4.096ms for OWRNG=1
        #[default]
        #[fallback]
        X1 = 0b000,
        /// 2x base soak time. 64us for OWRNG=0, 8.192ms for OWRNG=1
        X2 = 0b001,
        /// 4x base soak time. 128us for OWRNG=0, 16.384ms for OWRNG=1
        X4 = 0b010,
        /// 8x base soak time. 256us for OWRNG=0, 32.768ms for OWRNG=1
        X8 = 0b011,
        /// 16x base soak time. 512us for OWRNG=0, 65.536ms for OWRNG=1
        X16 = 0b100,
        /// 32x base soak time. 1.024ms for OWRNG=0, 131.072ms for OWRNG=1
        X32 = 0b101,
        /// 64x base soak time. 2.048ms for OWRNG=0, 262.144ms for OWRNG=1.
        X64 = 0b110,
        /// 128x base soak time. 4.096ms for OWRNG=0, 524.288ms for OWRNG=1.
        X128 = 0b111,
    }

    /// GPIOx pin pull-down config (GPIOx). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum GpioPullDownConfig {
        /// GPIOx pin pull-down on.
        PullDownOn = 0,
        /// GPIOx pin pull-down off (default).
        #[default]
        #[fallback]
        PullDownOff = 1,
    }

    /// Infinite Impulse Response (IIR) filter parameter/frequency settings (FC[2:0]). Three-bit field. See Table 21 on page 21 of the datasheet.
    /// 
    /// This enum basically just lets you configure the -3 dB corner frequency of the IIR Filter, which can be between 110Hz and 0.625Hz.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum IirFilterConfig {
        /// IIR filter is disabled (default).
        #[default]
        #[fallback]
        FilterDisabled = 0b000,
        /// -3dB frequency of 110 Hz. Filter Parameter = 2.
        Hz110 = 0b001,
        /// -3dB frequency of 45 Hz. Filter Parameter = 4.
        Hz45 = 0b010,
        /// -3dB frequency of 21 Hz. Filter Parameter = 8.
        Hz21 = 0b011,
        /// -3dB frequency of 10 Hz. Filter Parameter = 16.
        Hz10 = 0b100,
        /// -3dB frequency of 5 Hz. Filter Parameter = 32.
        Hz5 = 0b101,
        /// -3dB frequency of 1.25 Hz. Filter Parameter = 128.
        Hz1_25 = 0b110,
        /// -3dB frequency of 0.625 Hz. Filter Parameter = 256.
        Hz0_625 = 0b111,
    }

    /// Communication Break configuration (COMM_BK). One-bit field.
    /// 
    /// This lets you enable the communication break feature, which prevents the device from propagation communication further
    /// through the daisy chain.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum CommunicationBreak {
        /// Communication Break is disabled (default).
        #[default]
        #[fallback]
        Disable = 0,
        /// Communication Break is enabled, and the device will not do any propagation communication further through the daisy chain.
        Enable = 1,
    }
    
    /// Lets you configure the mute status (MUTE_ST). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum MuteStatus {
        /// Mute is deactivated.
        #[default]
        #[fallback]
        Deactivated = 0,
        /// Mute is activated and discharging is disabled.
        Activated = 1,
    }

    /// Lets you configure the snapshot status (SNAP_ST). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SnapshotStatus {
        /// Snapshot is deactivated.
        #[default]
        #[fallback]
        Deactivated = 0,
        /// Snapshot is activated, result registers are frozen.
        Activated = 1,
    }
}

/// Configuration Register Group A (CFGA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// See Table 55 on page 61 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::config::wrcfga().frame()),
    read = Some(commands::config::rdcfga().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct ConfigA {
    // CFGAR0! first byte of the register group.
    /// C-ADC vs. S-ADC comparison voltage theshold. Three-bit field.
    #[bits(3, default = types::ComparisonThresholdVoltage::DEFAULT)]  pub cth: types::ComparisonThresholdVoltage,
    #[bits(4, default = 0)]                                           _reserved0: u8,
    /// Reference powered up (REFON). One-bit field.
    #[bits(1, default = types::ReferenceOn::DEFAULT)]                 pub refon: types::ReferenceOn,

    // CFGAR1! second byte of the register group.
    /// Forces oscillator counter fast.
    #[bits(1, default = false)]  pub force_oscillator_counter_fast: bool,
    /// Forces oscillator counter slow.
    #[bits(1, default = false)]  pub force_oscillator_counter_slow: bool,
    /// Forces supply error detection.
    #[bits(1, default = false)]  pub force_supply_error_detection: bool,
    /// `true` selects supply OV and delta detection. `false` selects UV.
    #[bits(1, default = false)]  pub ov_uv: bool,
    /// Sets THSD.
    #[bits(1, default = false)]  pub thsd: bool,
    /// Forces nonvolatile memory (NVM) error detection (ED). Sets CED and SED.
    #[bits(1, default = false)]  pub force_nvmed: bool,
    /// Forces NVM multiple error detection (MED). Sets CMED and SMED.
    #[bits(1, default = false)]  pub force_nvmmed: bool,
    /// Forces TMODCHK.
    #[bits(1, default = false)]  pub force_tomdchk: bool,

    // CFGAR2! third byte of the register group.
    #[bits(3, default = 0)]     _reserved1: u8,
    /// Open wire soak times, for AUX commands.
    #[bits(3, default = types::OpenWireSoakTimeMultiplier::DEFAULT)]     pub owa: types::OpenWireSoakTimeMultiplier,
    /// Soak time range.
    #[bits(1, default = types::SoakTimeRange::DEFAULT)]                  pub owrng: types::SoakTimeRange,
    /// Soak time enabled/disabled.
    #[bits(1, default = types::SoakTimeOn::DEFAULT)]                     pub soakon: types::SoakTimeOn,

    // CFGAR3! fourth byte of the register group.
    /// Pull-up/pull-down config for GPIO1.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio1: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO2.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio2: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO3.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio3: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO4.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio4: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO5.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio5: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO6.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio6: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO7.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio7: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO8.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio8: types::GpioPullDownConfig,

    // CFGAR4! fifth byte of the register group.
    /// Pull-up/pull-down config for GPIO9.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio9: types::GpioPullDownConfig,
    /// Pull-up/pull-down config for GPIO10.
    #[bits(1, default = types::GpioPullDownConfig::DEFAULT)]     pub gpio10: types::GpioPullDownConfig,
    #[bits(6, default = 0)]     _reserved2: u8,

    // CFGAR5! sixth byte of the register group.
    /// Infinite Impulse Response (IIR) filter configuration.
    #[bits(3, default = types::IirFilterConfig::DEFAULT)]     pub fc: types::IirFilterConfig,
    /// Communication Break configuration.
    #[bits(1, default = types::CommunicationBreak::DEFAULT)]  pub comm_bk: types::CommunicationBreak,
    /// Mute status configuration.
    #[bits(1, default = types::MuteStatus::DEFAULT)]          pub mute_st: types::MuteStatus,
    /// Snapshot status configuration.
    #[bits(1, default = types::SnapshotStatus::DEFAULT)]      pub snap_st: types::SnapshotStatus,
    #[bits(2, default = 0)]                                   _reserved3: u8,

    #[bits(16, default = 0)]                                  _padding: u16,
}