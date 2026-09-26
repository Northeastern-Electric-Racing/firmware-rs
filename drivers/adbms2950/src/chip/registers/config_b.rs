//! Register Layouts and Bit Descriptions for Configuration Register Group B.
//!
//! The "main" struct here (i.e., the struct representing the overall register group) is `ConfigB`.
//!
//! This group is almost entirely the overcurrent comparator subsystem: three thresholds with
//! per-channel gain, the deglitch filter, the OCA/OCB output drivers, and the diagnostic signal
//! mux. The remaining byte configures the four GPIOs.
//!
//! For more info about these registers, see Table 71 on page 58 of the datasheet
//! (the register map) and Table 72 on page 58 of the datasheet (the bit descriptions).
//!
//! Per the note under Table 71, every bit the register map shows as `0` must be written as `0`.

use adbms_register_macros::BitfieldEnumDefault;
use bitfield_struct::{bitenum, bitfield};

use super::super::commands;
use super::register_group;

/// Field types relevant to Configuration Register B. See Table 72 on page 58 of the datasheet.
pub mod types {
    use super::{BitfieldEnumDefault, bitenum};

    /// Largest overcurrent threshold magnitude that still asserts the output.
    ///
    /// See `OC1TH` in Table 72 on page 58 of the datasheet.
    pub const OC_THRESHOLD_MAX: u8 = 0b011_1111;

    /// Smallest `OCxTH` code that deasserts the output entirely.
    ///
    /// The datasheet writes this as `0b1xxxxxx`: any code with bit 6 set deasserts the output,
    /// and a `CLRO` command then clears the matching `OCxR` result register to `0b10000000`.
    pub const OC_THRESHOLD_DISABLED: u8 = 0b100_0000;

    /// Whether an `OCxTH` code deasserts the output rather than setting a threshold.
    pub const fn oc_threshold_is_disabled(code: u8) -> bool {
        code & OC_THRESHOLD_DISABLED != 0
    }

    /// The comparison magnitude an `OCxTH` code selects, ignoring the deassert bit.
    ///
    /// A magnitude of `0` asserts the output irrespective of the measured value; a magnitude of
    /// `n` asserts it when `|OCxR| >= n`. What one count is worth in volts depends on the
    /// matching `OCxGC` gain bit: 5 mV at gain 1, 2.5 mV at gain 2.
    pub const fn oc_threshold_magnitude(code: u8) -> u8 {
        code & OC_THRESHOLD_MAX
    }

    /// OCxADC analog input gain control (OCxGC). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OverCurrentGain {
        /// Gain = 2, so one threshold count is 2.5 mV.
        Gain2 = 1,
        /// Gain = 1, so one threshold count is 5 mV (default).
        #[default]
        #[fallback]
        Gain1 = 0,
    }

    /// Overcurrent deglitch time threshold (OCDGT). Two-bit field.
    ///
    /// Applies to OC1ADC, OC2ADC, and OC3ADC together. Deglitching trades latency for immunity to
    /// noise on the sense inputs.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum DeglitchTime {
        /// 1oo1. Deglitching disabled; an overcurrent event goes straight to the output with no
        /// extra latency (default).
        #[default]
        #[fallback]
        OneOfOne = 0b00,
        /// 2oo3. The event progresses when two of three subsequent samples are above threshold.
        TwoOfThree = 0b01,
        /// 4oo8. The event progresses when four of eight subsequent samples are above threshold.
        FourOfEight = 0b10,
        /// 7oo8. The event progresses when seven of eight subsequent samples are above threshold.
        SevenOfEight = 0b11,
    }

    /// OCA and OCB output mode control (OCMODE). Two-bit field.
    ///
    /// This is the ADBMS2950B's actual PWM feature, and it is an *output*: in PWM1 or PWM2 mode
    /// the chip drives the OCA and OCB pins with a duty cycle that encodes the overcurrent state,
    /// for an external timer or capture-compare unit to decode. There is no PWM configuration
    /// register -- the duty cycles are fixed by the hardware.
    ///
    /// In PWM1 mode (Table 60 on page 47), with `OCAX` and `OCBX` not inverted: 0% high-impedance
    /// means reset or `OCEN = 0`; 0% actively driven low means a detected fault or an in-progress
    /// diagnostic; 25% means no overcurrent; 75% means more than one OCxADC saw the threshold
    /// violated.
    ///
    /// PWM2 mode (Table 61 on page 48) encodes more detail for a microcontroller: 50% is healthy,
    /// higher duty cycles indicate overcurrent, lower ones indicate pending faults or ongoing
    /// diagnostics, and the encoding distinguishes how many of the three OCxADCs tripped. Note
    /// overcurrent events are **not latched inside the chip** in this mode, so any deglitching
    /// beyond `OCDGT` is the host's job. A change in state takes effect at the start of the next
    /// PWM period, not mid-period.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OutputMode {
        /// OCA and OCB are disabled and high impedance (default).
        #[default]
        #[fallback]
        Disabled = 0b00,
        /// OCA and OCB are enabled in PWM1 mode.
        Pwm1 = 0b01,
        /// OCA and OCB are enabled in PWM2 mode.
        Pwm2 = 0b10,
        /// OCA and OCB are enabled in static mode.
        Static = 0b11,
    }

    /// Output XOR inverter for an overcurrent alert pin (OCAX, OCBX). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OutputPolarity {
        /// Inverted: active low, and a PWM period starts with a low pulse.
        Inverted = 1,
        /// Not inverted: active high, and a PWM period starts with a high pulse (default).
        #[default]
        #[fallback]
        ActiveHigh = 0,
    }

    /// OCA and OCB open-drain enable (OCOD). One-bit field.
    ///
    /// Ignored when `OCMODE` is `Disabled`.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OutputDriveMode {
        /// Open drain: high impedance for logic 1, pulled to GND for logic 0. An external pull-up
        /// is required in this mode (default).
        #[default]
        #[fallback]
        OpenDrain = 1,
        /// Push-pull: pushed to VREG for logic 1, pulled to GND for logic 0.
        PushPull = 0,
    }

    /// Reduced safety interval from the `OCEN` rising edge to the OCA/OCB drivers activating
    /// (OCDP). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SafetyInterval {
        /// Reduced interval of 3 OCxADC conversion cycles.
        Reduced = 1,
        /// Normal interval of 10 OCxADC conversion cycles (default).
        #[default]
        #[fallback]
        Normal = 0,
    }

    /// IxADC and VBxADC diagnostic select (DIAGSEL). Three-bit field.
    ///
    /// Chooses what the `ADI1` and `ADI2` diagnostic measurements actually convert. This only
    /// takes effect for commands issued with the diagnostic option bit set.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum DiagnosticSelect {
        /// No current injection. The IxADCs convert their regular inputs (IxA versus IxB) and the
        /// VBxADCs convert theirs, per the `VBxMUX` setting (default).
        #[default]
        #[fallback]
        Regular = 0b000,
        /// Current injected into the IxA and VBATx pins. Both ADCs still convert regular inputs.
        InjectIxaAndVbat = 0b001,
        /// Current injected into the IxB and SGND pins. Both ADCs still convert regular inputs.
        InjectIxbAndSgnd = 0b010,
        /// Current injected into the SxA pins.
        InjectSxa = 0b011,
        /// No injection. The IxADCs convert SxA versus IxA; the VBxADCs convert SGND against SGND,
        /// which is an offset measurement.
        SxaVersusIxaAndOffset = 0b100,
        /// No injection. Both the IxADCs and VBxADCs convert VDIV.
        Vdiv = 0b101,
        /// No injection. The IxADCs convert a scaled VREF2 (nominally -0.125 V) and the VBxADCs
        /// convert a scaled VREF2 (nominally 2.375 V).
        ScaledVref2 = 0b110,
        /// Current injected into the IxB pins. The VBxADCs convert regular inputs.
        InjectIxb = 0b111,
    }

    /// GPIOx output control (GPIOxC). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum GpioOutputControl {
        /// Output driver disabled, unless overruled by the COMM register (default).
        #[default]
        #[fallback]
        DriverDisabled = 1,
        /// Pulls down to GND.
        PullDown = 0,
    }

    /// GPIO2 toggle on OC1ADC end of conversion (GPIO2EOC). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Gpio2EndOfConversion {
        /// GPIO2 toggles on each OC1ADC end of conversion.
        ToggleOnConversion = 1,
        /// GPIO2 is controlled by `GPIO2C` or the COMM register (default).
        #[default]
        #[fallback]
        GeneralPurpose = 0,
    }
}

/// Configuration Register Group B.
///
/// See Table 71 on page 58 of the datasheet for the register map, and Table 72 on the same page
/// for the bit descriptions.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = Some(commands::config::wrcfgb().frame()),
    read = Some(commands::config::rdcfgb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct ConfigB {
    // CFGB0! first byte of the register group.
    /// OC1ADC overcurrent threshold (OC1TH). Seven-bit field.
    ///
    /// A signed 7-bit comparison code. `0` asserts OC1 irrespective of the measured value, `n`
    /// asserts it when `|OC1R| >= n`, and any code with bit 6 set deasserts it entirely. See
    /// `types::oc_threshold_magnitude` and `types::oc_threshold_is_disabled`. What one count is
    /// worth depends on `oc1gc`.
    #[bits(7, default = 0)]  pub oc1th: u8,
    #[bits(1, default = 0)]  _reserved0: u8,
    // CFGB1! second byte of the register group.
    /// OC2ADC overcurrent threshold (OC2TH). Seven-bit field. Interpreted like `oc1th`.
    #[bits(7, default = 0)]  pub oc2th: u8,
    #[bits(1, default = 0)]  _reserved1: u8,
    // CFGB2! third byte of the register group.
    /// OC3ADC overcurrent threshold (OC3TH). Seven-bit field. Interpreted like `oc1th`.
    #[bits(7, default = 0)]  pub oc3th: u8,
    #[bits(1, default = 0)]  _reserved2: u8,
    // CFGB3! fourth byte of the register group.
    /// Overcurrent deglitch time threshold (OCDGT). Two-bit field.
    #[bits(2, default = types::DeglitchTime::DEFAULT)]     pub ocdgt: types::DeglitchTime,
    #[bits(1, default = 0)]                                _reserved3: u8,
    /// Reduced safety interval from the `OCEN` rising edge to driver activation (OCDP). One-bit field.
    #[bits(1, default = types::SafetyInterval::DEFAULT)]   pub ocdp: types::SafetyInterval,
    #[bits(4, default = 0)]                                _reserved4: u8,
    // CFGB4! fifth byte of the register group.
    /// OCA and OCB open-drain enable (OCOD). One-bit field.
    #[bits(1, default = types::OutputDriveMode::DEFAULT)]  pub ocod: types::OutputDriveMode,
    /// OC1ADC analog input gain control (OC1GC). One-bit field.
    #[bits(1, default = types::OverCurrentGain::DEFAULT)]  pub oc1gc: types::OverCurrentGain,
    /// OC2ADC analog input gain control (OC2GC). One-bit field.
    #[bits(1, default = types::OverCurrentGain::DEFAULT)]  pub oc2gc: types::OverCurrentGain,
    /// OC3ADC analog input gain control (OC3GC). One-bit field.
    #[bits(1, default = types::OverCurrentGain::DEFAULT)]  pub oc3gc: types::OverCurrentGain,
    /// OCA and OCB output mode control (OCMODE). Two-bit field.
    #[bits(2, default = types::OutputMode::DEFAULT)]       pub ocmode: types::OutputMode,
    /// OCA output XOR inverter (OCAX). One-bit field.
    #[bits(1, default = types::OutputPolarity::DEFAULT)]   pub ocax: types::OutputPolarity,
    /// OCB output XOR inverter (OCBX). One-bit field.
    #[bits(1, default = types::OutputPolarity::DEFAULT)]   pub ocbx: types::OutputPolarity,
    // CFGB5! sixth byte of the register group.
    /// IxADC and VBxADC diagnostic select (DIAGSEL). Three-bit field.
    #[bits(3, default = types::DiagnosticSelect::DEFAULT)]        pub diagsel: types::DiagnosticSelect,
    /// GPIO2 toggle on OC1ADC end of conversion (GPIO2EOC). One-bit field.
    #[bits(1, default = types::Gpio2EndOfConversion::DEFAULT)]    pub gpio2eoc: types::Gpio2EndOfConversion,
    /// GPIO1 output control (GPIO1C). One-bit field.
    #[bits(1, default = types::GpioOutputControl::DEFAULT)]       pub gpio1c: types::GpioOutputControl,
    /// GPIO2 output control (GPIO2C). One-bit field.
    #[bits(1, default = types::GpioOutputControl::DEFAULT)]       pub gpio2c: types::GpioOutputControl,
    /// GPIO3 output control (GPIO3C). One-bit field.
    #[bits(1, default = types::GpioOutputControl::DEFAULT)]       pub gpio3c: types::GpioOutputControl,
    /// GPIO4 output control (GPIO4C). One-bit field.
    #[bits(1, default = types::GpioOutputControl::DEFAULT)]       pub gpio4c: types::GpioOutputControl,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]                                      _padding: u16,
}
