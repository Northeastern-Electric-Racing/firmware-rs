//! Register Layouts and Bit Descriptions for Configuration Register Group A.
//!
//! The "main" struct here (i.e., the struct representing the overall register group) is `ConfigA`.
//!
//! For more info about these registers, see Table 69 on page 54 of the datasheet
//! (the register map) and Table 70 on page 54 of the datasheet (the bit descriptions).
//!
//! Note that `REFUP` and `SNAPST` are read-only status bits that live inside this otherwise
//! writable register group. Writing them has no effect.

use adbms_register_macros::BitfieldEnumDefault;
use bitfield_struct::{bitenum, bitfield};

use super::super::commands;
use super::register_group;

/// Field types relevant to Configuration Register A. See Table 70 on page 54 of the datasheet.
pub mod types {
    use super::{BitfieldEnumDefault, bitenum};

    /// OCxADC enable (OCEN). One-bit field.
    ///
    /// Toggling this requires a settling delay before the overcurrent results are valid; the
    /// datasheet specifies a wait between a `WRCFGA` clearing `OCEN` and a subsequent `WRCFGA`
    /// setting it again.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OverCurrentAdcEnable {
        /// OC1ADC, OC2ADC, and OC3ADC enabled.
        Enabled = 1,
        /// OC1ADC, OC2ADC, and OC3ADC disabled (default).
        #[default]
        #[fallback]
        Disabled = 0,
    }

    /// Negative input selection for a two-bit VSx field (VS1, VS2). Two-bit field.
    ///
    /// Applies to the V1 and V2 measurement channels, which can additionally reference V3 and V4.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum VoltageReferenceWide {
        /// Measure against SGND (default).
        #[default]
        #[fallback]
        Sgnd = 0b00,
        /// Measure against VREF1P25.
        Vref1p25 = 0b01,
        /// Measure against V3.
        V3 = 0b10,
        /// Measure against V4.
        V4 = 0b11,
    }

    /// Negative input selection for a one-bit VSx field (VS3 through VS10). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum VoltageReference {
        /// Measure against VREF1P25.
        Vref1p25 = 1,
        /// Measure against SGND (default).
        #[default]
        #[fallback]
        Sgnd = 0,
    }

    /// Soak time applied to the V1ADC and V2ADC (SOAK). Three-bit field.
    ///
    /// Delays the response of the V1ADC and V2ADC to an `ADV` command and to any further
    /// measurements in a round-robin sweep, letting the input settle first.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SoakTime {
        /// Soak time disabled (default).
        #[default]
        #[fallback]
        Disabled = 0b000,
        /// 100 us of soak time.
        Us100 = 0b001,
        /// 500 us of soak time.
        Us500 = 0b010,
        /// 1 ms of soak time.
        Ms1 = 0b011,
        /// 2 ms of soak time.
        Ms2 = 0b100,
        /// 10 ms of soak time.
        Ms10 = 0b101,
        /// 20 ms of soak time.
        Ms20 = 0b110,
        /// 150 ms of soak time.
        Ms150 = 0b111,
    }

    /// Output state control for GPO1 through GPO5 (GPOxC). One-bit field.
    ///
    /// What the "driven" state physically does depends on the matching `GPOxOD` bit: push-pull
    /// drives to VDD, open-drain goes high impedance.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum GpoOutputState {
        /// Pushed to VDD (if `GPOxOD` is push-pull) or high impedance (if open-drain). Default.
        #[default]
        #[fallback]
        Driven = 1,
        /// Pulled low to GND.
        PulledLow = 0,
    }

    /// Output state control for GPO6 (GPO6C). Two-bit field.
    ///
    /// GPO6 is the only GPO that can emit a PWM clock instead of a static level.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Gpo6OutputState {
        /// Pulled low to GND.
        PulledLow = 0b00,
        /// Pushed to VDD (if `GPO6OD` is push-pull) or high impedance (if open-drain). Default.
        #[default]
        Driven = 0b01,
        /// Outputs 200 kHz. Readback is disabled in this mode.
        Clock200kHz = 0b10,
        /// Reserved.
        #[fallback]
        Reserved = 0b11,
    }

    /// Open-drain mode for a GPO pin (GPOxOD). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum GpoDriveMode {
        /// Open drain: high impedance when driven, pulled to GND otherwise (default).
        #[default]
        #[fallback]
        OpenDrain = 1,
        /// Push-pull: pushed to VDD when driven, pulled to GND otherwise.
        PushPull = 0,
    }

    /// GPIO1 fault output enable (GPIO1FE). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Gpio1FaultOutput {
        /// GPIO1 outputs fault status.
        FaultStatus = 1,
        /// GPIO1 is controlled by `GPIO1C`, or by the COMM register when used as an SPI
        /// controller (default).
        #[default]
        #[fallback]
        GeneralPurpose = 0,
    }

    /// SPI controller mode select (SPI3W). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SpiWireMode {
        /// 3-wire: SDIM and SDOM share one pin.
        ThreeWire = 1,
        /// 4-wire: SDIM and SDOM on separate pins (default).
        #[default]
        #[fallback]
        FourWire = 0,
    }

    /// isoSPI communication break (COMMBK). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum CommunicationBreak {
        /// No transmission from the peripheral port to the controller port.
        Enabled = 1,
        /// Communication propagates between Port A and Port B, in whichever direction it was
        /// initiated (default).
        #[default]
        #[fallback]
        Disabled = 0,
    }

    /// Voltage reference power status (REFUP). One-bit field. Read only.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum ReferencePowered {
        /// VREF1 and VREF2 are powered.
        Powered = 1,
        /// VREF1 and VREF2 are not (yet) powered (default).
        #[default]
        #[fallback]
        NotPowered = 0,
    }

    /// SNAP status indicator (SNAPST). One-bit field. Read only.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum SnapshotStatus {
        /// SNAP active. Result registers are frozen until the next `UNSNAP` command.
        Active = 1,
        /// SNAP inactive. Result registers progress normally (default).
        #[default]
        #[fallback]
        Inactive = 0,
    }

    /// Negative input multiplexer select for VB1ADC (VB1MUX). One-bit field.
    ///
    /// The VB1ADC latches this bit when an `ADI1` command arrives, so a change here needs a new
    /// `ADI1` before the ADC behavior updates.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Battery1MuxSelect {
        /// VB1ADC measures VBAT1 against VBAT2 (differential).
        VsBat2 = 1,
        /// VB1ADC measures VBAT1 against SGND (default).
        #[default]
        #[fallback]
        VsSgnd = 0,
    }

    /// Negative input multiplexer select for VB2ADC (VB2MUX). One-bit field.
    ///
    /// The VB2ADC latches this bit when an `ADI1` with `RD = 1`, or an `ADI2`, arrives.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Battery2MuxSelect {
        /// VB2ADC measures VBAT2 against VBAT1. Note this is a double polarity inversion, because
        /// the VB2ADC has a negative LSB.
        VsBat1 = 1,
        /// VB2ADC measures VBAT2 against SGND (default).
        #[default]
        #[fallback]
        VsSgnd = 0,
    }

    /// Accumulator depth for the IxACC and VBxACC registers (ACCI). Three-bit field.
    ///
    /// The accumulators are updated every `ACCN` conversions with the sum of `ACCN` samples,
    /// where `ACCN = 4 * (ACCI + 1)`. A new value takes effect on the next `ADI1` or `ADI2`.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum AccumulatorDepth {
        /// Accumulate 4 samples.
        Samples4 = 0b000,
        /// Accumulate 8 samples (default).
        #[default]
        #[fallback]
        Samples8 = 0b001,
        /// Accumulate 12 samples.
        Samples12 = 0b010,
        /// Accumulate 16 samples.
        Samples16 = 0b011,
        /// Accumulate 20 samples.
        Samples20 = 0b100,
        /// Accumulate 24 samples.
        Samples24 = 0b101,
        /// Accumulate 28 samples.
        Samples28 = 0b110,
        /// Accumulate 32 samples.
        Samples32 = 0b111,
    }

    /// Clock monitor diagnostic injection (INJOSC). Two-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum InjectOscillatorFault {
        /// Regular clock applied to the clock monitor; no-clock detector in regular mode
        /// (default).
        #[default]
        #[fallback]
        Normal = 0b00,
        /// Faster clock applied to the clock monitor; no-clock detector in regular mode.
        FastClock = 0b01,
        /// Slower clock applied to the clock monitor; no-clock detector checks for clock stuck
        /// high.
        SlowClockAndStuckHigh = 0b10,
        /// Regular clock applied to the clock monitor; no-clock detector checks for clock stuck
        /// low.
        StuckLow = 0b11,
    }

    /// Supply monitor and deglitcher diagnostic injection (INJMON). Two-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum InjectSupplyMonitorFault {
        /// Supply monitors and deglitcher operate normally (default).
        #[default]
        #[fallback]
        Normal = 0b00,
        /// Supply monitors operate normally; deglitcher mismatch forced, to set the `OCMM` flag.
        ForceDeglitcherMismatch = 0b01,
        /// Force undervoltage, to trigger the `VDDUV`, `VREGUV`, and `VDIGUV` checks.
        ForceUndervoltage = 0b10,
        /// Force overvoltage, to trigger the `VREGOV`, `VDIGOV`, `VDE`, and `VDEL` checks.
        ForceOvervoltage = 0b11,
    }

    /// Thermal shutdown diagnostic injection (INJTS). One-bit field.
    ///
    /// Setting this forces the `THSD` flag but does not itself trigger an internal reset. The host
    /// must keep a log when setting it: after reading `THSD`, both `INJTS` (via `WRCFGA`) and
    /// `THSD` (via `CLRFLAG`) must be cleared. If an `SRST` is issued before clearing `THSD`, the
    /// FLAG register reports `RESET` and `THSD` together, indistinguishable from a real thermal
    /// shutdown.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum InjectThermalShutdown {
        /// Forces the `THSD` flag.
        Forced = 1,
        /// Normal operation (default).
        #[default]
        #[fallback]
        Normal = 0,
    }

    /// Test mode indicator diagnostic injection (INJTM). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum InjectTestMode {
        /// Forces the `TMODE` flag.
        Forced = 1,
        /// Normal operation (default).
        #[default]
        #[fallback]
        Normal = 0,
    }

    /// ECC diagnostic injection (INJECC). One-bit field.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum InjectEccError {
        /// Inject a bit error and trigger the ECC logic, setting `SED1`, `SED2`, `MED1`, and
        /// `MED2`.
        Forced = 1,
        /// Normal operation (default).
        #[default]
        #[fallback]
        Normal = 0,
    }
}

/// Configuration Register Group A.
///
/// See Table 69 on page 54 of the datasheet for the register map, and Table 70 on the same page
/// for the bit descriptions.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = Some(commands::config::wrcfga().frame()),
    read = Some(commands::config::rdcfga().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct ConfigA {
    // CFGA0! first byte of the register group.
    /// Reference voltage for the V1 measurement (VS1). Two-bit field.
    #[bits(2, default = types::VoltageReferenceWide::DEFAULT)]  pub vs1: types::VoltageReferenceWide,
    /// Reference voltage for the V2 measurement (VS2). Two-bit field.
    #[bits(2, default = types::VoltageReferenceWide::DEFAULT)]  pub vs2: types::VoltageReferenceWide,
    /// Reference voltage for the V3 measurement (VS3). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]      pub vs3: types::VoltageReference,
    /// Reference voltage for the V4 measurement (VS4). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]      pub vs4: types::VoltageReference,
    /// Reference voltage for the V5 measurement (VS5). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]      pub vs5: types::VoltageReference,
    /// OCxADC enable (OCEN). One-bit field.
    #[bits(1, default = types::OverCurrentAdcEnable::DEFAULT)]  pub ocen: types::OverCurrentAdcEnable,
    // CFGA1! second byte of the register group.
    /// Clock monitor diagnostic injection (INJOSC). Two-bit field.
    #[bits(2, default = types::InjectOscillatorFault::DEFAULT)]     pub injosc: types::InjectOscillatorFault,
    /// Supply monitor and deglitcher diagnostic injection (INJMON). Two-bit field.
    #[bits(2, default = types::InjectSupplyMonitorFault::DEFAULT)]  pub injmon: types::InjectSupplyMonitorFault,
    /// Thermal shutdown diagnostic injection (INJTS). One-bit field.
    #[bits(1, default = types::InjectThermalShutdown::DEFAULT)]     pub injts: types::InjectThermalShutdown,
    #[bits(1, default = 0)]                                         _reserved0: u8,
    /// ECC diagnostic injection (INJECC). One-bit field.
    #[bits(1, default = types::InjectEccError::DEFAULT)]            pub injecc: types::InjectEccError,
    /// Test mode indicator diagnostic injection (INJTM). One-bit field.
    #[bits(1, default = types::InjectTestMode::DEFAULT)]            pub injtm: types::InjectTestMode,
    // CFGA2! third byte of the register group.
    /// Reference voltage for the V6 measurement (VS6). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]  pub vs6: types::VoltageReference,
    /// Reference voltage for the V7 measurement (VS7). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]  pub vs7: types::VoltageReference,
    /// Reference voltage for the V8 measurement (VS8). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]  pub vs8: types::VoltageReference,
    /// Reference voltage for the V9 measurement (VS9). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]  pub vs9: types::VoltageReference,
    /// Reference voltage for the V10 measurement (VS10). One-bit field.
    #[bits(1, default = types::VoltageReference::DEFAULT)]  pub vs10: types::VoltageReference,
    /// Soak time applied to the V1ADC and V2ADC (SOAK). Three-bit field.
    #[bits(3, default = types::SoakTime::DEFAULT)]          pub soak: types::SoakTime,
    // CFGA3! fourth byte of the register group.
    /// Output state control for GPO1 (GPO1C). One-bit field.
    #[bits(1, default = types::GpoOutputState::DEFAULT)]   pub gpo1c: types::GpoOutputState,
    /// Output state control for GPO2 (GPO2C). One-bit field.
    #[bits(1, default = types::GpoOutputState::DEFAULT)]   pub gpo2c: types::GpoOutputState,
    /// Output state control for GPO3 (GPO3C). One-bit field.
    #[bits(1, default = types::GpoOutputState::DEFAULT)]   pub gpo3c: types::GpoOutputState,
    /// Output state control for GPO4 (GPO4C). One-bit field.
    #[bits(1, default = types::GpoOutputState::DEFAULT)]   pub gpo4c: types::GpoOutputState,
    /// Output state control for GPO5 (GPO5C). One-bit field.
    #[bits(1, default = types::GpoOutputState::DEFAULT)]   pub gpo5c: types::GpoOutputState,
    /// Output state control for GPO6 (GPO6C). Two-bit field.
    #[bits(2, default = types::Gpo6OutputState::DEFAULT)]  pub gpo6c: types::Gpo6OutputState,
    #[bits(1, default = 0)]                                _reserved1: u8,
    // CFGA4! fifth byte of the register group.
    /// Open-drain mode for GPO1 (GPO1OD). One-bit field.
    #[bits(1, default = types::GpoDriveMode::DEFAULT)]        pub gpo1od: types::GpoDriveMode,
    /// Open-drain mode for GPO2 (GPO2OD). One-bit field.
    #[bits(1, default = types::GpoDriveMode::DEFAULT)]        pub gpo2od: types::GpoDriveMode,
    /// Open-drain mode for GPO3 (GPO3OD). One-bit field.
    #[bits(1, default = types::GpoDriveMode::DEFAULT)]        pub gpo3od: types::GpoDriveMode,
    /// Open-drain mode for GPO4 (GPO4OD). One-bit field.
    #[bits(1, default = types::GpoDriveMode::DEFAULT)]        pub gpo4od: types::GpoDriveMode,
    /// Open-drain mode for GPO5 (GPO5OD). One-bit field.
    #[bits(1, default = types::GpoDriveMode::DEFAULT)]        pub gpo5od: types::GpoDriveMode,
    /// Open-drain mode for GPO6 (GPO6OD). One-bit field.
    #[bits(1, default = types::GpoDriveMode::DEFAULT)]        pub gpo6od: types::GpoDriveMode,
    /// GPIO1 fault output enable (GPIO1FE). One-bit field.
    #[bits(1, default = types::Gpio1FaultOutput::DEFAULT)]    pub gpio1fe: types::Gpio1FaultOutput,
    /// SPI controller mode select (SPI3W). One-bit field.
    #[bits(1, default = types::SpiWireMode::DEFAULT)]         pub spi3w: types::SpiWireMode,
    // CFGA5! sixth byte of the register group.
    /// Accumulator depth for the IxACC and VBxACC registers (ACCI). Three-bit field.
    #[bits(3, default = types::AccumulatorDepth::DEFAULT)]      pub acci: types::AccumulatorDepth,
    /// isoSPI communication break (COMMBK). One-bit field.
    #[bits(1, default = types::CommunicationBreak::DEFAULT)]    pub commbk: types::CommunicationBreak,
    /// Voltage reference power status (REFUP). One-bit field. Read only.
    #[bits(1, default = types::ReferencePowered::DEFAULT)]      pub refup: types::ReferencePowered,
    /// SNAP status indicator (SNAPST). One-bit field. Read only.
    #[bits(1, default = types::SnapshotStatus::DEFAULT)]        pub snapst: types::SnapshotStatus,
    /// Negative input multiplexer select for VB1ADC (VB1MUX). One-bit field.
    #[bits(1, default = types::Battery1MuxSelect::DEFAULT)]     pub vb1mux: types::Battery1MuxSelect,
    /// Negative input multiplexer select for VB2ADC (VB2MUX). One-bit field.
    #[bits(1, default = types::Battery2MuxSelect::DEFAULT)]     pub vb2mux: types::Battery2MuxSelect,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]                                    _padding: u16,
}
