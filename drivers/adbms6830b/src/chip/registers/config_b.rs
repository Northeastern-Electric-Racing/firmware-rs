//! Register Layouts and Bit Descriptions for Configuration Register Group B. 
//! 
//! The "main" struct here (i.e., the struct representing the overall register group) is `ConfigB`.
//! `ConfigB` can be translated into full protocol frames (for sending) via `ConfigBReadRequest`, `ConfigBReadResponse`, and `ConfigBWriteFrame`.
//! 
//! For more info about these registers, see Table 103 on page 71 of the datasheet
//! and Table 56 on page 61 of the datasheet.

use bitfield_struct::{bitfield, bitenum};
use adbms6830b_macros::BitfieldEnumDefault;

use super::register_group;
use super::super::commands;

/// Field types relavent to Configuration Register B. See Table 103 on page 71 of the datasheet.
pub mod types {
    use super::{bitenum, bitfield, BitfieldEnumDefault};

    /// Microvolts per VUV/VOV code increment.
    const LSB_MICROVOLTS: i32 = 2400;
    /// The offset of the threshold formula (+1.5 V).
    const OFFSET_MICROVOLTS: i32 = 1_500_000;

    /// Max threshold the register can represent in microvolts.
    /// This is around +6,412,800 uV / +6.4128 V.
    const MAX_MICROVOLTS: i32 = 6_412_800;

    /// Min threshold the register can represent. This is around -3,415,200 uV, or around -3.4152 V.
    const MIN_MICROVOLTS: i32 = -3_415_200;

    /// Takes in a desired UV/OV threshold in microvolts, and returns the raw 12-bit VUV/VOV code.
    /// 
    /// This is based on the equation from the datasheet:
    /// Cell threshold = VUV x 16 x 150uV + 1.5V.
    /// - The threshold is in Volts.
    /// - VUV/VOV is a signed 12-bit (two's complement) code that maps to a threshold voltage.
    /// 
    /// The inverse (with `x` in microvolts) is:
    /// VUV(x) = (x - 1_500_000) / 2400)    (with the result being rounded cus 2400 probably wont divide cleanly most the time)
    /// 
    /// This will return `None` if the microvolts input is outside the representable range.
    const fn vuv_vov_from_microvolts(microvolts: i32) -> Option<u16> {
        if microvolts < MIN_MICROVOLTS || microvolts > MAX_MICROVOLTS { return None; }

        let numerator = microvolts - OFFSET_MICROVOLTS;

        // do the rounding
        let code = if numerator >= 0 {
            (numerator + LSB_MICROVOLTS / 2) / LSB_MICROVOLTS
        } else {
            (numerator - LSB_MICROVOLTS / 2) / LSB_MICROVOLTS
        };

        // Store the low 12 bits of the two's complement representation.
        Some((code as i16 as u16) & 0x0FFF)
    }

    /// Converts a raw 12-bit two's complement VUV/VOV code back into a threshold in microvolts.
    const fn vuv_vov_to_microvolts(code: u16) -> i32 {
        let signed = ((code << 4) as i16) >> 4; // turn the raw code into a signed i16
        signed as i32 * LSB_MICROVOLTS + OFFSET_MICROVOLTS
    }

    /// Undervoltage threshold/comparison voltage (VUV). Signed 12-bit field. Default is for VUV is 0x800 (corresponding to 3,415,200 uV / -3.4152 V).
    /// 
    /// This type provides the `from_microvolts()` function to construct a `UndervoltageThreshold` based on a desired
    /// undervoltage threshold in uV. If you want to construct the raw VUV[11:0] field value directly, use `with_value()`.
    /// 
    /// This type also provides the `as_microvolts()` function to convert an `UndervoltageThreshold` into microvolts (could be useful on reads).
    /// 
    /// Note: Cell undervoltage threshold = VUV x 16 x 150uV + 1.5V (VUV is signed two's complement).
    #[bitfield(u16, defmt = cfg(feature = "defmt"))]
    #[derive(PartialEq, Eq)]
    pub struct UndervoltageThreshold {
        /// Cell undervoltage threshold = VUV x 16 x 150uV + 1.5V. Corresponds to `VUV[11:0]`.
        #[bits(12, default = 0x800)]    pub value: u16,
        #[bits(4, default = 0)]         _reserved: u8,
    }
    impl UndervoltageThreshold { 
        pub const DEFAULT: Self = Self::new();
        pub const MAX_MICROVOLTS: i32 = MAX_MICROVOLTS;
        pub const MIN_MICROVOLTS: i32 = MIN_MICROVOLTS;

        /// Creates a new `UndervoltageThreshold` from a desired threshold voltage.
        /// 
        /// ### Parameters
        /// - `microvolts`: Desired undervoltage threshold, in uV. May be negative.
        /// 
        /// This function will return `None` if your input is outside the `MIN_MICROVOLTS` (-3,415,200 uV/~-3.4152 V)
        /// to `MAX_MICROVOLTS` (6,412,800 uV/~6.4128 V) range.
        pub const fn from_microvolts(microvolts: i32) -> Option<Self> {
            match vuv_vov_from_microvolts(microvolts) {
                Some(value) => Some(Self::new().with_value(value)),
                None => None,
            }
        }

        /// Returns the undervoltage threshold this setting represents, in microvolts (may be negative).
        pub const fn as_microvolts(&self) -> i32 {
            vuv_vov_to_microvolts(self.value())
        }
    }

    /// Overvoltage threshold/comparison voltage (VOV). Signed 12-bit field. Default for VOV is 0x7FF (corresponding to 6,412,800 uV / 6.4128 V).
    /// 
    /// This type provides the `from_microvolts()` function to construct a `OvervoltageThreshold` based on a desired
    /// overvoltage threshold in uV. If you want to construct the raw VOV[11:0] field value directly, use `with_value()`.
    /// 
    /// This type also provides the `as_microvolts()` function to convert an `OvervoltageThreshold` into microvolts (could be useful on reads).
    /// 
    /// Note: Cell overvoltage threshold = VOV * 16 * 150 uV + 1.5 V (VOV is signed two's complement).
    #[bitfield(u16, defmt = cfg(feature = "defmt"))]
    #[derive(PartialEq, Eq)]
    pub struct OvervoltageThreshold {
        /// Cell overvoltage threshold = VOV * 16 * 150 uV + 1.5 V. Corresponds to `VOV[11:0]`.
        #[bits(12, default = 0x7FF)]     pub value: u16,
        #[bits(4, default = 0)]         _reserved: u8,
    }
    impl OvervoltageThreshold { 
        pub const DEFAULT: Self = Self::new(); 

        pub const MAX_MICROVOLTS: i32 = MAX_MICROVOLTS;
        pub const MIN_MICROVOLTS: i32 = MIN_MICROVOLTS;

        /// Creates a new `OvervoltageThreshold` from a desired threshold voltage.
        /// 
        /// ### Parameters
        /// - `microvolts`: Desired overvoltage threshold, in uV. May be negative.
        /// 
        /// This function will return `None` if your input is outside the `MIN_MICROVOLTS` (-3,415,200 uV/~-3.4152 V)
        /// to `MAX_MICROVOLTS` (6,412,800 uV/~6.4128 V) range.
        pub const fn from_microvolts(microvolts: i32) -> Option<Self> {
            match vuv_vov_from_microvolts(microvolts) {
                Some(value) => Some(Self::new().with_value(value)),
                None => None,
            }
        }

        /// Returns the overvoltage threshold this setting represents, in microvolts (may be negative).
        pub const fn as_microvolts(&self) -> i32 {
            vuv_vov_to_microvolts(self.value())
        }
    }

    /// Lets you enable/disable the discharge timer monitor (DTMEN). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum DischargeTimerMonitor {
        /// Disables the discharge timer monitor function (default).
        #[default]
        #[fallback]
        Disabled = 0,
        /// Enables the discharge timer monitor function if the device transitions to the extended balancing state.
        Enabled = 1,
    }

    /// Lets you configure the discharge timer range (DTRNG). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum DischargeTimerRange {
        /// Uses the short discharge timer range (0 to 63 minutes in 1 minute increments). (default)
        #[default]
        #[fallback]
        ShortRange = 0,
        /// Uses the long discharge timer range (0 to 16.8 hours in 16 minute increments).
        LongRange = 1,
    }

    /// Lets you configure the discharge timeout value, or read back the timer's status (DCTO). Six-bit field.
    /// 
    /// Note: This setting causes different behaviors depending on write/read.
    /// - Write = Set new value, 16-minute or 1-minute increments according to DTRNG read.
    /// - Read = remaining value, 16-minute or 1-minute increments according to DTRNG.
    /// 
    /// To convert between increments and minutes, you need to match on your current `DischargeTimerRange` setting:
    /// - If `DischargeTimerRange::ShortRange`, minutes = increments() * 1 (since 1 increment = 1 minute)
    /// - If `DischargeTimerRange::LongRange`, minutes = increments() * 16 (since 1 increment = 16 minutes)
    #[bitfield(u8, defmt = cfg(feature = "defmt"))]
    #[derive(PartialEq, Eq)]
    pub struct DischargeTimerStatus {
        /// Number of increments remaining on the timer. This value can range between 0 and 63. Defaults to 0.
        /// 
        /// Note: The meaning of an increment depends on the `DischargeTimerRange`/DTRNG value set.
        #[bits(6, default = 0)]     pub increments: u8,
        #[bits(2, default = 0)]     _reserved: u8,
    }
    impl DischargeTimerStatus {
        pub const DEFAULT: Self = Self::new();

        /// Indicates whether or not we are timed out. In other words, this checks if our remaining increments == 0 or not.
        /// 
        /// - `false` means that there's one or more increments remaining.
        /// - `true` means that there's zero increments remaining. This means that we've timed out, or that 
        /// `DischargeTimerStatus` simply hasn't been set yet (since it defaults to 0).
        pub const fn timed_out(&self) -> bool {
            self.increments() == 0
        }
    }

    /// Lets you configure the shorting switch for a discharge cell `x`. One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum DischargeCellConfig {
        /// Continuously turns off shorting switch for Cell `x` (default).
        #[default]
        #[fallback]
        ShortingSwitchOff = 0,
        /// Continuously turns on shorting switch for Cell `x`.
        ShortingSwitchOn = 1,
    }
}

/// Configuration Register Group B (CFGB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// See Table 56 on page 61 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::config::wrcfgb().frame()),
    read = Some(commands::config::rdcfgb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct ConfigB {
    /// UV threshold/comparison voltage (VUV). Cell undervoltage threshold = VUV x 16 x 150uV + 1.5V. 12-bit field. Corresponds to `VUV[11:0]`.
    #[bits(12, default = types::UndervoltageThreshold::DEFAULT)]  pub vuv: types::UndervoltageThreshold,
    /// OV threshold/comparison voltage (VOV). Cell overvoltage threshold = VOV × 16 × 150 μV + 1.5 V. 12-bit field. Corresponds to `VOV[11:0]`.
    #[bits(12, default = types::OvervoltageThreshold::DEFAULT)]  pub vov: types::OvervoltageThreshold,
    /// Status of the discharge timer. Corresponds to `DCTO[5:0]`.
    #[bits(6, default = types::DischargeTimerStatus::DEFAULT)]  pub dcto: types::DischargeTimerStatus,
    /// Range of the discharge timer.
    #[bits(1, default = types::DischargeTimerRange::DEFAULT)]   pub dtrng: types::DischargeTimerRange,
    /// Enable/disable discharge timer monitoring.
    #[bits(1, default = types::DischargeTimerMonitor::DEFAULT)] pub dtmen: types::DischargeTimerMonitor,
    /// Discharge Cell 1 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc1: types::DischargeCellConfig,
    /// Discharge Cell 2 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc2: types::DischargeCellConfig,
    /// Discharge Cell 3 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc3: types::DischargeCellConfig,
    /// Discharge Cell 4 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc4: types::DischargeCellConfig,
    /// Discharge Cell 5 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc5: types::DischargeCellConfig,
    /// Discharge Cell 6 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc6: types::DischargeCellConfig,
    /// Discharge Cell 7 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc7: types::DischargeCellConfig,
    /// Discharge Cell 8 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc8: types::DischargeCellConfig,
    /// Discharge Cell 9 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc9: types::DischargeCellConfig,
    /// Discharge Cell 10 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc10: types::DischargeCellConfig,
    /// Discharge Cell 11 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc11: types::DischargeCellConfig,
    /// Discharge Cell 12 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc12: types::DischargeCellConfig,
    /// Discharge Cell 13 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc13: types::DischargeCellConfig,
    /// Discharge Cell 14 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc14: types::DischargeCellConfig,
    /// Discharge Cell 15 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc15: types::DischargeCellConfig,
    /// Discharge Cell 16 Configuration
    #[bits(1, default = types::DischargeCellConfig::DEFAULT)]   pub dcc16: types::DischargeCellConfig,
    #[bits(16, default = 0)]                                    _padding: u16,

}
