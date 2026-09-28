//! Register group declarations based on the ADBMS2950B datasheet.
//!
//! Each group here is a `#[bitfield]` struct laid out to match its table in the MEMORY MAP section
//! of the datasheet. Groups that can be written implement the `WritableGroup` trait, and groups that can be
//! read implement the `ReadableGroup` trait.

#![allow(dead_code)]
#![allow(rustdoc::broken_intra_doc_links)]

use adbms_register_macros::register_group;

use super::commands::CommandFrame;

/// Size of every ADBMS2950B register group in bytes.
///
/// All the ordinary register groups in the datasheet are 6 bytes.
pub const GROUP_BYTES: usize = 6;

/// A register group the host can write.
///
/// Implemented by the `register_group` macro for any group given a `write` command.
pub trait WritableGroup: Copy {
    /// Command that writes this group.
    const WRITE_COMMAND: CommandFrame;

    /// Serializes the group into its protocol bytes.
    fn to_bytes(self) -> [u8; GROUP_BYTES];
}

/// A register group the host can read.
///
/// Implemented by the `register_group` macro for any group given a `read` command.
pub trait ReadableGroup: Copy {
    /// Command that reads this group.
    const READ_COMMAND: CommandFrame;

    /// Reconstructs the group from its protocol bytes.
    fn from_bytes(bytes: [u8; GROUP_BYTES]) -> Self;
}

pub mod aux;
pub mod comm;
pub mod config_a;
pub mod config_b;
pub mod flag;
pub mod results;
pub mod serial_id;
pub mod status;
pub mod voltage;

/// Shared measurement result types, generated from the scaling formulas in the datasheet.
///
/// Every result register on this chip is a signed two's-complement code with a fixed LSB, so these
/// are all the same shape: a newtype over the raw code plus an `as_microvolts()` that applies the
/// LSB. Note several of the LSBs are **negative** -- the V2ADC path and the VB2ADC have inverted
/// gain relative to their V1ADC/VB1ADC counterparts -- so sign handling is not optional.
///
/// See Table 43 on page 32, Table 44 on page 33, and Table 51 on page 38 of the datasheet.
pub mod measurement {
    use bitfield_struct::bitfield;

    /// Declares a 16-bit signed result register with a fixed microvolt LSB.
    macro_rules! impl_measurement16 {
        ($(#[$meta:meta])* $name:ident, $lsb_microvolts:expr, $reset:expr, $cleared:expr) => {
            $(#[$meta])*
            #[bitfield(u16, defmt = cfg(feature = "defmt"))]
            #[derive(PartialEq, Eq)]
            pub struct $name {
                /// The raw signed register code.
                #[bits(16)] pub raw: i16,
            }
            impl $name {
                /// Microvolts per register code increment. Negative where the ADC has inverted gain.
                pub const LSB_MICROVOLTS: i32 = $lsb_microvolts;
                /// The value this register holds after a reset, per the datasheet's Reset column.
                pub const DEFAULT: Self = Self::from_bits($reset);
                /// The value this register holds after its clear command.
                pub const CLEARED: Self = Self::from_bits($cleared);
                /// The measured voltage in microvolts.
                pub const fn as_microvolts(self) -> i32 {
                    self.raw() as i32 * Self::LSB_MICROVOLTS
                }
            }
        };
    }

    /// Declares a 24-bit signed result register with a fixed microvolt LSB.
    macro_rules! impl_measurement24 {
        ($(#[$meta:meta])* $name:ident, $lsb_microvolts:expr, $reset:expr, $cleared:expr) => {
            $(#[$meta])*
            #[bitfield(u32, defmt = cfg(feature = "defmt"))]
            #[derive(PartialEq, Eq)]
            pub struct $name {
                /// The raw signed register code, sign extended from 24 bits.
                #[bits(24)] pub raw: i32,
                #[bits(8)] _reserved: u8,
            }
            impl $name {
                /// Microvolts per register code increment. Negative where the ADC has inverted gain.
                pub const LSB_MICROVOLTS: i32 = $lsb_microvolts;
                /// The value this register holds after a reset, per the datasheet's Reset column.
                pub const DEFAULT: Self = Self::from_bits($reset);
                /// The value this register holds after its clear command.
                pub const CLEARED: Self = Self::from_bits($cleared);
                /// The measured voltage in microvolts.
                pub const fn as_microvolts(self) -> i32 {
                    self.raw() * Self::LSB_MICROVOLTS
                }
                /// The averaged voltage in microvolts, for accumulator registers.
                ///
                /// Accumulators hold a sum of `accn` samples, where `accn` comes from
                /// `accumulation_count(acci)` for the `ACCI` field in CFGA.
                pub const fn averaged_microvolts(self, accn: u16) -> i32 {
                    self.as_microvolts() / accn as i32
                }
            }
        };
    }

    impl_measurement24!(
        /// I1ADC current result (`I1`). Signed 24-bit, 1 uV per code.
        ///
        /// This is the voltage across the shunt, `VSHUNT = I1A - I1B`, not a current. Dividing by
        /// the shunt resistance is the caller's job and deliberately not done here.
        /// Resets to `0x03FFFF`; a `CLRI` command sets it to `0xFC0000`.
        Current1, 1, 0x03FFFF, 0xFC0000
    );
    impl_measurement24!(
        /// I2ADC current result (`I2`). Signed 24-bit, -1 uV per code.
        ///
        /// The I2ADC has **inverted gain** relative to the I1ADC: `VSHUNT = -(I2 * 1 uV) = I2B - I2A`.
        /// Resets to `0x03FFFF`; a `CLRI` command sets it to `0xFC0000`.
        Current2, -1, 0x03FFFF, 0xFC0000
    );
    impl_measurement24!(
        /// I1ADC accumulated current (`I1ACC`). Signed 24-bit, 1 uV per code.
        ///
        /// This is a **sum**, not an average. To get the averaged shunt voltage, divide by `ACCN`,
        /// which is `4 * (ACCI + 1)` for the `ACCI` field in CFGA -- see `averaged_microvolts`.
        AccumulatedCurrent1, 1, 0x03FFFF, 0xFC0000
    );
    impl_measurement24!(
        /// I2ADC accumulated current (`I2ACC`). Signed 24-bit, -1 uV per code (inverted gain).
        AccumulatedCurrent2, -1, 0x03FFFF, 0xFC0000
    );
    impl_measurement24!(
        /// VB1ADC accumulated battery voltage (`VB1ACC`). Signed 24-bit, 100 uV per code.
        AccumulatedBatteryVoltage1, 100, 0x7FFFFF, 0x800000
    );
    impl_measurement24!(
        /// VB2ADC accumulated battery voltage (`VB2ACC`). Signed 24-bit, -85 uV per code.
        ///
        /// The VB2ADC has inverted gain *and* a different LSB magnitude than the VB1ADC.
        AccumulatedBatteryVoltage2, -85, 0x7FFFFF, 0x800000
    );

    impl_measurement16!(
        /// VB1ADC battery voltage result (`VB1`). Signed 16-bit, 100 uV per code.
        BatteryVoltage1, 100, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// VB2ADC battery voltage result (`VB2`). Signed 16-bit, -85 uV per code.
        ///
        /// Inverted gain and a different LSB magnitude than the VB1ADC: `VBAT = -(VB2 * 85 uV)`.
        BatteryVoltage2, -85, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// A V1ADC voltage channel result (`V1A` .. `V8A`). Signed 16-bit, 100 uV per code.
        VoltageA, 100, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// A V2ADC voltage channel result (`V1B` .. `V10B`). Signed 16-bit, -85 uV per code.
        ///
        /// The whole V2ADC path has inverted gain relative to the V1ADC path.
        VoltageB, -85, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// V1ADC VREF2 result (`VREF2A`). Signed 16-bit, 240 uV per code.
        ///
        /// The LSB is the V1ADC's 100 uV scaled by the 3 : 1.25 VREF2 divider.
        Vref2A, 240, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// V2ADC VREF2 result (`VREF2B`). Signed 16-bit, -204 uV per code.
        Vref2B, -204, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// An AUX ADC rail measured at the AUX ADC's own 100 uV LSB (`VREF1P25`, `VDIV`, `EPAD`).
        AuxVoltage, 100, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// An AUX ADC rail measured through a divider, at 240 uV per code (`VREG`, `VDIG`).
        AuxDividedVoltage, 240, 0x7FFF, 0x8000
    );
    impl_measurement16!(
        /// AUX ADC VDD result (`VDD`). Signed 16-bit, 1 mV per code.
        SupplyVoltage, 1000, 0x7FFF, 0x8000
    );

    /// AUX ADC temperature 1 result (`TMP1`), the die temperature. Signed 16-bit.
    ///
    /// Per Table 51, `temperature in C = (TMP1 / 61.8) - 250`.
    #[bitfield(u16, defmt = cfg(feature = "defmt"))]
    #[derive(PartialEq, Eq)]
    pub struct Temperature1 {
        /// The raw signed register code.
        #[bits(16)]
        pub raw: i16,
    }
    impl Temperature1 {
        /// The value this register holds after a reset (`0x7FFF`), per Table 51.
        pub const DEFAULT: Self = Self::from_bits(0x7FFF);
        /// The value this register holds after a `CLRVX` command (`0x8000`).
        pub const CLEARED: Self = Self::from_bits(0x8000);
        /// The measured temperature in millidegrees Celsius.
        ///
        /// `(raw / 61.8) - 250` in degrees, rearranged as exact integer arithmetic to avoid
        /// floating point (`f64` is denied workspace-wide, and `f32` would lose precision here).
        pub const fn as_millicelsius(self) -> i32 {
            (self.raw() as i32 * 10_000) / 618 - 250_000
        }
    }

    /// AUX ADC temperature 2 result (`TMP2`). Signed 16-bit.
    ///
    /// Per Table 51, `temperature in C = (TMP2 / 20.5) - 267`.
    #[bitfield(u16, defmt = cfg(feature = "defmt"))]
    #[derive(PartialEq, Eq)]
    pub struct Temperature2 {
        /// The raw signed register code.
        #[bits(16)]
        pub raw: i16,
    }
    impl Temperature2 {
        /// The value this register holds after a reset (`0x7FFF`), per Table 51.
        pub const DEFAULT: Self = Self::from_bits(0x7FFF);
        /// The value this register holds after a `CLRVX` command (`0x8000`).
        pub const CLEARED: Self = Self::from_bits(0x8000);
        /// The measured temperature in millidegrees Celsius.
        ///
        /// `(raw / 20.5) - 267` in degrees, as exact integer arithmetic.
        pub const fn as_millicelsius(self) -> i32 {
            (self.raw() as i32 * 2_000) / 41 - 267_000
        }
    }

    /// Number of samples the accumulators sum, for a given `ACCI` code from CFGA.
    ///
    /// Per `ACCI` in Table 70, `ACCN = 4 * (ACCI + 1)`. The accumulator registers hold a sum of
    /// `ACCN` samples, so dividing by this is what turns one into an average.
    pub const fn accumulation_count(acci: u8) -> u16 {
        4 * (acci as u16 + 1)
    }
}
