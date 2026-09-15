//! Status Registers!!!!!!!

use bitfield_struct::{bitfield, bitenum};
use adbms6830b_macros::BitfieldEnumDefault;

use super::{register_group};
use super::super::commands;

/// Field types relavent to the status registers.
pub mod types {
    use crate::chip::registers::table107::{impl_firstrowregister, impl_itmpinner};
    use super::{bitenum, BitfieldEnumDefault, bitfield};

    /// Field types relavent to Status Register A. See Table 105 on page 71 of the datasheet.
    pub mod a {
        use super::{impl_firstrowregister, impl_itmpinner};

        impl_firstrowregister!(
            /// Second reference voltage (VREF2). See Table 105 on page 71 of the datasheet.
            /// 
            /// 16-bit ADC measurement value for second reference voltage for second reference = VREF2 × 150 μV +1.5 V.
            /// Normal range is within 2.988 V to 3.012 V considering data sheet limits, thermal hysteresis, and long-term drift.
            /// Reset to 0x8000 after power-up, sleep, or clear command (CLRAUX)
            Vref2,
            0x8000
        );

        impl_itmpinner!(
            /// Internal die temperature (ITMP). See Table 105 on page 72 of the datasheet.
            /// 
            /// 16-bit ADC measurement value of Internal Die temperature. Temperature measurement voltage = (ITMP × 150
            /// μV + 1.5 V)/7.5 mV/°C – 273°C. Reset to 0x7FFF after power-up, sleep, and to 0x8000 after clear command
            /// (CLRAUX).
            InternalDieTemperature,
            0x7FFF
        );
    }

    /// Field types relavent to Status Register B. See Table 106 on page 71 of the datasheet.
    pub mod b {
        use super::{impl_firstrowregister};

        impl_firstrowregister!(
            /// Digital power supply voltage (VD). See Table 106 on page 71 of the datasheet.
            /// VD is off in sleep.
            /// 
            /// 16-bit ADC measurement value of digital power supply voltage. Digital power supply voltage = VD × 150 μV +
            /// 1.5 V. Normal range is within 2.7 V to 3.6 V. Reset to 0x7FFF after power-up, sleep, and to 0x8000 after clear
            /// command (CLRAUX).
            DigitalPowerSupplyVoltage,
            0x7FFF
        );

        impl_firstrowregister!(
            /// Analog power supply voltage (VA). See Table 106 on page 71 of the datasheet.
            /// voltage = voltage at the V_REG pin. VD is off in sleep.
            /// 
            /// Note: "VD is off in sleep" is taken directly from the datasheet but maybe whoever wrote the datasheet meant to say "VA is off in sleep"? Or maybe not. I do not know
            /// 
            /// 6-bit ADC measurement value of analog power supply voltage. Analog power supply voltage = VA × 150 μV + 1.5
            /// V. The value of VA is set by external components and must be in the range of 4.5 V to 5.5 V for normal operation.
            /// Reset to 0x7FFF after power-up, sleep, and to 0x8000 after clear command (CLRAUX).
            AnalogPowerSupplyVoltage,
            0x7FFF
        );

        impl_firstrowregister!(
            /// VREF2 across resistor (VRES). See Table 106 on page 71 of the datasheet.
            /// 
            /// 16-bit ADC value of VREF2 with series resistor for open wire check. Voltage = VRES × 150 μV + 1.5 V. Reset to
            /// 0x7FFF after power-up, sleep, and to 0x8000 after clear command (CLRAUX).
            Vres,
            0x7FFF
        );
    }

    /// Field types relavent to Status Register C. See Table 108 on pages 72-73 of the datasheet.
    pub mod c {
        use super::{bitenum, BitfieldEnumDefault, bitfield};

        /// C-ADC vs. S-ADC fault of Channel `X` (CSxFLT). One-bit field.
        /// 
        /// There are 16 channels (so this corresponds to CS1FLT through CS16FLT).
        /// 
        /// This fault condition is related to the `ComparisonThreshold` setting from Configuration Register A.
        /// Also, this bit defaults to `1`, meaning every channel powers up as indicating a mismatch.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum ComparisonFault {
            /// A mismatch between C-ADC and S-ADC measurement on Channel `X` occurred.
            #[default]
            #[fallback]
            MismatchOccured = 1,
            /// No mismatch between C-ADC and S-ADC measurement on Channel `X` occurred.
            Okay = 0,
        }
        impl ComparisonFault {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// 5V analog rail OV (VA_OV). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_VAOV = 1.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum AnalogRailOvervoltage {
            /// Overvoltage event detected on the main 5 V analog power rail during an ADC operation.
            #[default]
            #[fallback]
            OvervoltageEventDetected = 1,
            /// No overvoltage event detected on the analog power rail.
            Okay = 0,
        }
        impl AnalogRailOvervoltage {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// 5V analog rail UV (VA_UV). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_VAUV = 1. Because VA is derived from
        /// V_REG, VA_UV is set when entering standby from sleep.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum AnalogRailUndervoltage {
            /// Undervoltage event detected on the main 5 V analog power rail during an ADC operation.
            #[default]
            #[fallback]
            UndervoltageEventDetected = 1,
            /// No undervoltage event detected on the analog power rail.
            Okay = 0,
        }
        impl AnalogRailUndervoltage {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// 3V digital rail OV (VD_OV). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_VDOV = 1
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum DigitalRailOvervoltage {
            /// Overvoltage event detected on the digital power rail during an ADC operation.
            #[default]
            #[fallback]
            OvervoltageEventDetected = 1,
            /// No overvoltage event detected on the digital power rail.
            Okay = 0,
        }
        impl DigitalRailOvervoltage {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// 3V digital rail UV (VD_UV). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_VDUV = 1. Because VD is derived from
        /// V_REG, VD_UV is set when entering standby from sleep.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum DigitalRailUndervoltage {
            /// Undervoltage event detected on the digital power rail during an ADC operation.
            #[default]
            #[fallback]
            UndervoltageEventDetected = 1,
            /// No undervoltage event detected on the digital power rail.
            Okay = 0,
        }
        impl DigitalRailUndervoltage {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// C-trim error detection (CED). One-bit field. This bit defaults to `1`.
        /// 
        /// The ADBMS6830B can correct single trim errors. 
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum CTrimError {
            /// Trim error detected in C-NVM.
            #[default]
            #[fallback]
            CTrimErrorDetected = 1,
            /// No trim error detected in C-NVM.
            Okay = 0,
        }
        impl CTrimError {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// C-trim multiple error detection (CMED). One-bit field. This bit defaults to `1`.
        /// 
        /// Multiple trim errors can lead to parameters out of specification.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum CTrimMultipleError {
            /// Multiple trim errors detected in C-NVM.
            #[default]
            #[fallback]
            CTrimMultipleErrorsDetected = 1,
            /// No multiple trim errors detected in C-NVM.
            Okay = 0,
        }
        impl CTrimMultipleError {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// S-trim error detection (SED). One-bit field. This bit defaults to `1`.
        /// 
        /// The ADBMS6830B can correct single trim errors. 
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum STrimError {
            /// Trim error detected in S-NVM.
            #[default]
            #[fallback]
            STrimErrorDetected = 1,
            /// No trim error detected in S-NVM.
            Okay = 0,
        }
        impl STrimError {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// S-trim multiple error detection (SMED). One-bit field. This bit defaults to `1`.
        /// 
        /// Multiple trim errors can lead to parameters out of specification.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum STrimMultipleError {
            /// Multiple trim errors detected in S-NVM.
            #[default]
            #[fallback]
            STrimMultipleErrorsDetected = 1,
            /// No multiple trim errors detected in S-NVM.
            Okay = 0,
        }
        impl STrimMultipleError {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Supply rail delta (VDE). One-bit field. This bit defaults to `1`.
        /// 
        /// This flag indicates if ANY of the 5V supplies differ from VREG by more than 0.5 V.
        /// This is different from `SupplyRailDeltaLatent`, which indicates if ALL of the 5V supplies
        /// differ from VREG by more than 0.5 V.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_VDE = 1.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum SupplyRailDelta {
            /// Any of the 5 V supplies differ from VREG by more than 0.5 V.
            #[default]
            #[fallback]
            AnyDeltaDetected = 1,
            /// No delta of 5 V supplies detected
            Okay = 0,
        }
        impl SupplyRailDelta {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Supply rail delta latent (VDEL). One-bit field. This bit defaults to `1`.
        /// 
        /// This flag indicates if ALL of the 5V supplies differ from VREG by more than 0.5 V.
        /// This is different from `SupplyRailDelta`, which indicates if ANY of the 5V supplies
        /// differ from VREG by more than 0.5 V.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_VDEL = 1. VDEL allows to check supply rail
        /// monitors for latent faults.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum SupplyRailDeltaLatent {
            /// All the 5 V supplies differed from VREG by more than 0.5 V.
            #[default]
            #[fallback]
            DeltaDetectedForAll = 1,
            /// Not all the 5 V supplies differed from VREG by more than 0.5 V.
            Okay = 0,
        }
        impl SupplyRailDeltaLatent {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// ComparisonActive (COMP). One-bit field. This bit defaults to `0`.
        /// 
        /// Indicates that the comparison between C-ADC and S-ADC results is active. 
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum ComparisonActive {
            /// Indicates that the comparison between C-ADC and S-ADC results is active.
            ComparisonActive = 1,
            /// C-ADC vs. S-ADC comparison off.
            #[default]
            #[fallback]
            ComparisonOff = 0,
        }
        impl ComparisonActive {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// SPI fault (SPIFLT). One-bit field. This bit defaults to `1`.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum SpiFault {
            /// A mismatch between redundant SPI slave outputs occurred.
            #[default]
            #[fallback]
            MismatchOccured = 1,
            /// No mismatch between redundant SPI slave outputs occurred.
            Okay = 0,
        }
        impl SpiFault {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Sleep mode detection (SLEEP). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_SLEEP = 1. 
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum SleepModeDetection {
            /// The device has previously power cycled or entered sleep mode.
            #[default]
            #[fallback]
            SleepModeDetected = 1,
            /// The device has not power cycled or entered sleep mode.
            SleepModeNotDetected = 0,
        }
        impl SleepModeDetection {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Thermal shutdown status (THSD). One-bit field. This bit defaults to `0`.
        /// 
        /// THSD bit cleared to 0 by using the CLRFLAG command with CL_THSD = 1.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum ThermalShutdownStatus {
            /// Thermal shutdown occurred.
            Occurred = 1,
            /// Thermal shutdown did not occur.
            #[default]
            #[fallback]
            DidNotOccur = 0,
        }
        impl ThermalShutdownStatus {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Test mode detection (TMODCHK). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_TMODE = 1.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum TestModeDetection {
            /// The device has previously activated a test mode.
            #[default]
            #[fallback]
            TestModeDetected = 1,
            /// The device has not activated a test mode.
            TestModeNotDetected = 0,
        }
        impl TestModeDetection {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Oscillator check (OSCCHK). One-bit field. This bit defaults to `1`.
        /// 
        /// This bit can be cleared to 0 by using the CLRFLAG command with CL_OSCCHK = 1.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum OscillatorCheck {
            /// An out of range oscillator count is detected during an ADC operation.
            #[default]
            #[fallback]
            OutOfRangeOscillatorDetected = 1,
            /// No out of range oscillator counts are detected.
            Okay = 0,
        }
        impl OscillatorCheck {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Conversions counter (CT[10:0]). 11-bit field. Default is 0.
        /// 
        /// This is a free-running C-ADC conversion counter. Resets with every ADCV command. Rolls over after the maximum value.
        /// 
        /// This struct needs to be constructed manually from a `ConversionsCounterLower` and `ConversionsCounterUpper`, via the `ConversionsCounter::new()` function.
        /// 
        /// See `ConversionsCount` for the combined `CT[10:0]` + `CTS[1:0]` counter.
        #[bitfield(u16, new = false, defmt = cfg(feature = "defmt"))]
        #[derive(PartialEq, Eq)]
        pub struct ConversionsCounter {
            /// Lower 6 bits (`CT[5:0]` of the Conversion Counter
            #[bits(6, default = ConversionsCounterLower::DEFAULT)] lower: ConversionsCounterLower,
            /// Upper 5 bits (`CT[10:6]` of the Conversion Counter
            #[bits(5, default = ConversionsCounterUpper::DEFAULT)] upper: ConversionsCounterUpper,
            #[bits(5, default = 0)] _reserved: u8,
        }
        impl ConversionsCounter {
            /// Creates a new `ConversionsCounter` from a `ConversionsCounterLower` and `ConversionsCounterUpper`
            pub const fn new(lower: ConversionsCounterLower, upper: ConversionsCounterUpper) -> Self {
                let me: Self = ConversionsCounter(0);
                me.with_lower(lower).with_upper(upper)
            }

            /// The 11-bit Conversions Counter value (`CT[10:0]`).
            pub const fn value(&self) -> u16 {
                ((self.upper().value() as u16) << 6) | self.lower().value() as u16
            }
        }

        /// Struct representing the upper 5 bits (CT[10:6]) of the conversion counter.
        /// Pass this struct into `ConversionsCounter::new()` alongside a `ConversionsCounterLower` instance
        /// to construct a real `ConversionsCounter`.
        /// 
        /// This type needs to exist because `CT[10:0]` is a non-contiguous field inside Status Register Group C (see Table 91 on page 68 of the datasheet).
        /// Because of that, the lower 6 bits (`CT[4:0]`) and upper 5 bits (`CT[10:6]`) need to be represented separately for the initial
        /// SPI read and then combined into a real `ConversionsCounter` value.
        #[bitfield(u8, defmt = cfg(feature = "defmt"))]
        #[derive(PartialEq, Eq)]
        pub struct ConversionsCounterUpper {
            /// (CT[10:6]).
            /// 
            /// This is private outside of this submodule since it isn't really
            /// meant to be accessed directly. This type as a whole should only be used
            /// to construct a real `ConversionsCounter`.
            #[bits(5, default = 0)]        pub(in super) value: u8,
            #[bits(3, default = 0)]        _reserved: u8,
        }
        impl ConversionsCounterUpper { pub const DEFAULT: Self = Self::new(); }

        /// Struct representing the lower 6 bits (CT[5:0]) of the conversion counter.
        /// Pass this struct into `ConversionsCounter::new()` alongside a `ConversionsCounterUpper` instance
        /// to construct a real `ConversionsCounter`.
        /// 
        /// This type needs to exist because `CT[10:0]` is a non-contiguous field inside Status Register Group C (see Table 91 on page 68 of the datasheet).
        /// Because of that, the lower 6 bits (`CT[5:0]`) and upper 5 bits (`CT[10:6]`) need to be represented separately for the initial
        /// SPI read and then combined into a real `ConversionsCounter` value.
        #[bitfield(u8, defmt = cfg(feature = "defmt"))]
        #[derive(PartialEq, Eq)]
        pub struct ConversionsCounterLower {
            /// (CT[5:0]).
            /// 
            /// This is private outside of this submodule since it isn't really
            /// meant to be accessed directly. This type as a whole should only be used
            /// to construct a real `ConversionsCounter`.
            #[bits(6, default = 0)]        pub(in super) value: u8,
            #[bits(2, default = 0)]        _reserved: u8,
        }
        impl ConversionsCounterLower { pub const DEFAULT: Self = Self::new(); }

        /// Conversions subcounter (CTS[1:0]). 2-bit field. Default is 0.
        /// 
        /// This is a free running C-ADC subsample conversion counter. Four increments per sample. Resets with every ADCV
        /// command. Rolls over after maximum value. 
        /// 
        /// CT[10:0], CTS[1:0] can be treated as a 13-bit counter CCTS[12:0] that
        /// increments four times per sample. Can be read coherently to CADC results using the SNAP command to identify
        /// new or old samples. Coherency to SADC results is guaranteed only when CCTS is not 31, 32, 63, 64, …
        /// 
        /// You can create the 13-bit `CCTS[12:0]` view via the `ConversionsCount` type.
        #[bitfield(u16, defmt = cfg(feature = "defmt"))]
        #[derive(PartialEq, Eq)]
        pub struct ConversionsSubcounter {
            /// The 2-bit conversions subcounter value.
            #[bits(2, default = 0)]        pub value: u8,
            #[bits(14, default = 0)]       _reserved: u16,
        }
        impl ConversionsSubcounter { pub const DEFAULT: Self = Self::new(); }

        /// The combined conversion count (CCTS[12:0]). 13-bit value. Default is 0.
        /// 
        /// This counter resets with every ADCV command and rolls over after its maximum value.
        /// It can be read coherently to C-ADC results using the SNAP command to identify new or old samples. Coherency
        /// to S-ADC results is guaranteed only when `CCTS` is not 31, 32, 63, 64, ….
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct ConversionsCount {
            counter: ConversionsCounter,
            subcounter: ConversionsSubcounter,
        }
        impl ConversionsCount {
            /// Creates a new `ConversionsCount` from the three raw StatusC fields.
            pub const fn new(lower: ConversionsCounterLower, upper: ConversionsCounterUpper, subcounter: ConversionsSubcounter) -> Self {
                Self::from_counter(ConversionsCounter::new(lower, upper), subcounter)
            }

            /// Creates a new `ConversionsCount` from a `ConversionsCounter` and a `ConversionsSubcounter`.
            pub const fn from_counter(counter: ConversionsCounter, subcounter: ConversionsSubcounter) -> Self {
                Self { counter, subcounter }
            }

            /// Full 13-bit conversion count (`CCTS[12:0]`). This increments four times per sample.
            pub const fn value(&self) -> u16 {
                (self.samples() << 2) | self.subsample() as u16
            }

            /// Sample count (`CT[10:0]`). This increments once per sample.
            pub const fn samples(&self) -> u16 { self.counter.value() }

            /// Subsample count within the current sample (`CTS[1:0]`).
            pub const fn subsample(&self) -> u8 { self.subcounter.value() }

            /// `ConversionsCounter` part of this count (`CT[10:0]`).
            pub const fn ct(&self) -> ConversionsCounter { self.counter }

            /// `ConversionsSubcounter` part of this count (`CTS[1:0]`).
            pub const fn cts(&self) -> ConversionsSubcounter { self.subcounter }
        }
    }

    /// Field types relavent to Status Register D. See Table 109 on pages 73-74 of the datasheet.
    pub mod d {
        use super::{bitenum, BitfieldEnumDefault, bitfield};

        /// Cell `x` overvoltage flag (CxOV). One-bit field. This bit defaults to `1`.
        /// 
        /// x = 1 to 16: C-ADC 1 ms cell voltage measurement result compared to VOV comparison voltage.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum CellOvervoltageFlag {
            /// Cell `x` flagged for overvoltage condition.
            #[default]
            #[fallback]
            Overvoltage = 1,
            /// Cell `x` not flagged for overvoltage condition.
            Okay = 0,
        }
        impl CellOvervoltageFlag {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Cell `x` undervoltage flag (CxUV). One-bit field. This bit defaults to `1`.
        /// 
        /// x = 1 to 16: C-ADC 1 ms cell voltage measurement result compared to VUV comparison voltage.
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum CellUndervoltageFlag {
            /// Cell `x` flagged for undervoltage condition.
            #[default]
            #[fallback]
            Undervoltage = 1,
            /// Cell `x` not flagged for undervoltage condition.
            Okay = 0,
        }
        impl CellUndervoltageFlag {
            /// Is this flag set?
            pub const fn is_set(&self) -> bool { (*self as u8) == 1}
        }

        /// Oscillator check counter (OC_CNTR[7:0]). 8-bit field. Default is 0.
        /// 
        /// Stores the results of the oscillator counter check. If no OSCCHK failure occurs, this stores the most recently
        /// acquired oscillator count. If an OSCCHK failure occurs, this stores the first failing counter value. Passing
        /// range is 52 to 71.
        /// 
        /// ^^ In other words, the way you inerperet this value should depend on the `OSCCHK` result from Status Register C (see the `OscillatorCheck` type).
        /// If `OSCCHK == Okay` (meaning no OSCCHK failure occured), the `count()` value on this type indicates the most recently acquired oscillator count.
        /// If `OSCCHK == OutOfRangeOscillatorDetected` (meaning a OSCCHK failure occured), the `count()` value on this type indicates the first failing counter value.
        #[bitfield(u8, defmt = cfg(feature = "defmt"))]
        #[derive(PartialEq, Eq)]
        pub struct OscillatorCheckCounter {
            /// The 8-bit oscillator check counter.
            #[bits(8, default = 0, access = RO)]        pub count: u8,
        }
        impl OscillatorCheckCounter { pub const DEFAULT: Self = Self::new(); }
    }

    /// Field types relavent to Status Register E. See Table 110 on page 74 of the datasheet.
    /// 
    /// Note: This module doesn't have types for `RSVD`, `RSVD0`, or `RSVD1` (from Table 110), since those are just
    /// reserved bits. Maybe in the future it would be good to have types for them just for error checking or something?
    /// But for now who cares! Those fields will just be treat as normal private `_reserved` fields inside the bitfield struct.
    pub mod e {
        use super::{bitenum, BitfieldEnumDefault, bitfield};

        /// GPIOx pin state (GPIx). One-bit field. This bit defaults to `0`.
        /// 
        /// Indicates whether pin `x` is at logic 0/Low or logic 1/High
        #[repr(u8)]
        #[bitenum]
        #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum GpioPinState {
            /// Pin is at Logic 1 (HIGH).
            High = 1,
            /// Pin is at Logic 0 (LOW).
            #[default]
            #[fallback]
            Low = 0,
        }

        /// Device revision code (REV[3:0]). 4-bit field.
        /// 
        /// Uhh it seems like the datasheet specifies no default for this field. I am guessing that is because this is just
        /// a constant/hardcoded field in the hardware that never changes. Actually that probably is why. Nonetheless for software purposes I
        /// am making this field default to 0. Probably see Table 110 on page 74 of the datasheet if care
        #[bitfield(u8, defmt = cfg(feature = "defmt"))]
        #[derive(PartialEq, Eq)]
        pub struct RevisionCode {
            /// The 4-bit device revision code.
            #[bits(4, default = 0, access = RO)]        pub code: u8,
            #[bits(4, default = 0)]                     _reserved: u8,
        }
        impl RevisionCode { pub const DEFAULT: Self = Self::new(); }
    }
}

/// Status Register Group A (STATA). 
/// Contains six 1-byte registers (so 6 bytes total),but the last two bytes are all reserved.
/// 
/// See Table 89 on page 67 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::status::rdstata().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct StatusA {
    /// Second reference voltage. Corresponds to `VREF2[15:0]`.
    #[bits(16, default = types::a::Vref2::DEFAULT)]                    pub vref2: types::a::Vref2,
    /// Internal die temperature. Corresponds to `ITMP[15:0]`.
    #[bits(16, default = types::a::InternalDieTemperature::DEFAULT)]   pub itmp: types::a::InternalDieTemperature,

    /// Corresponds to the "Reserved" bytes (last two) in Table 89.
    /// Technically these don't have a default at all and reading from them is undefined
    #[bits(16, default = 0)]                                           _reserved: u32,

    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}

/// Status Register Group B (STATB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// See Table 90 on pages 67-68 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::status::rdstatb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct StatusB {
    /// Digital power supply voltage. Corresponds to `VD[15:0]`.
    #[bits(16, default = types::b::DigitalPowerSupplyVoltage::DEFAULT)] pub vd: types::b::DigitalPowerSupplyVoltage,
    /// Analog power supply voltage. Corresponds to `VA[15:0]`.
    #[bits(16, default = types::b::AnalogPowerSupplyVoltage::DEFAULT)]  pub va: types::b::AnalogPowerSupplyVoltage,
    /// VREF2 voltage across resistor. Corresponds to `VRES[15:0]`.
    #[bits(16, default = types::b::Vres::DEFAULT)]                      pub vres: types::b::Vres,

    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}

/// Status Register Group C (STATC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// See Table 91 on page 68 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::status::rdstatc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct StatusC {
    /// Comparison fault for Channel 1. Corresponds to `CS1FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs1flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 2. Corresponds to `CS2FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs2flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 3. Corresponds to `CS3FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs3flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 4. Corresponds to `CS4FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs4flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 5. Corresponds to `CS5FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs5flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 6. Corresponds to `CS6FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs6flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 7. Corresponds to `CS7FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs7flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 8. Corresponds to `CS8FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs8flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 9. Corresponds to `CS9FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs9flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 10. Corresponds to `CS10FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs10flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 11. Corresponds to `CS11FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs11flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 12. Corresponds to `CS12FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs12flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 13. Corresponds to `CS13FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs13flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 14. Corresponds to `CS14FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs14flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 15. Corresponds to `CS15FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs15flt: types::c::ComparisonFault,
    /// Comparison fault for Channel 16. Corresponds to `CS16FLT`.
    #[bits(1, default = types::c::ComparisonFault::DEFAULT)] pub cs16flt: types::c::ComparisonFault,

    /// Upper 5 bits of the Conversions Counter. Corresponds to `CT[10:6]`.
    /// 
    /// Combine with `ct_lower` in `ConversionsCounter::new()` to get a real combined/usable conversions counter value.
    #[bits(5, default = types::c::ConversionsCounterUpper::DEFAULT)] pub ct_upper: types::c::ConversionsCounterUpper,
    #[bits(3, default = 0)]                                          _reserved: u8,

    /// Conversions Subcounter. Corresponds to `CTS[1:0]`.
    #[bits(2, default = types::c::ConversionsSubcounter::DEFAULT)]   pub cts: types::c::ConversionsSubcounter,
    /// Lower 6 bits of the Conversions Counter. Corresponds to `CT[5:0]`.
    /// 
    /// Combine with `ct_upper` in `ConversionsCounter::new()` to get a real combined/usable conversions counter value.
    #[bits(6, default = types::c::ConversionsCounterLower::DEFAULT)] pub ct_lower: types::c::ConversionsCounterLower,

    /// S-trim multiple error detection. Corresponds to `SMED`.
    #[bits(1, default = types::c::STrimMultipleError::DEFAULT)]      pub smed: types::c::STrimMultipleError,
    /// S-trim error detection. Corresponds to `SED`.
    #[bits(1, default = types::c::STrimError::DEFAULT)]              pub sed: types::c::STrimError,
    /// C-trim multiple error detection. Corresponds to `CMED`.
    #[bits(1, default = types::c::CTrimMultipleError::DEFAULT)]      pub cmed: types::c::CTrimMultipleError,
    /// C-trim error detection. Corresponds to `CED`.
    #[bits(1, default = types::c::CTrimError::DEFAULT)]              pub ced: types::c::CTrimError,
    /// Digital rail undervoltage. Corresponds to `VD_UV`.
    #[bits(1, default = types::c::DigitalRailUndervoltage::DEFAULT)] pub vd_uv: types::c::DigitalRailUndervoltage,
    /// Digital rail overvoltage. Corresponds to `VD_OV`.
    #[bits(1, default = types::c::DigitalRailOvervoltage::DEFAULT)]  pub vd_ov: types::c::DigitalRailOvervoltage,
    /// Analog rail undervoltage. Corresponds to `VA_UV`.
    #[bits(1, default = types::c::AnalogRailUndervoltage::DEFAULT)]  pub va_uv: types::c::AnalogRailUndervoltage,
    /// Analog rail overvoltage. Corresponds to `VA_OV`.
    #[bits(1, default = types::c::AnalogRailOvervoltage::DEFAULT)]   pub va_ov: types::c::AnalogRailOvervoltage,

    /// Oscillator check. Corresponds to `OSCCHK`.
    #[bits(1, default = types::c::OscillatorCheck::DEFAULT)]         pub oscchk: types::c::OscillatorCheck,
    /// Test mode detection. Corresponds to `TMODCHK`.
    #[bits(1, default = types::c::TestModeDetection::DEFAULT)]       pub tmodchk: types::c::TestModeDetection,
    /// Thermal shutdown status. Corresponds to `THSD`.
    #[bits(1, default = types::c::ThermalShutdownStatus::DEFAULT)]   pub thsd: types::c::ThermalShutdownStatus,
    /// Sleep mode detection. Corresponds to `SLEEP`.
    #[bits(1, default = types::c::SleepModeDetection::DEFAULT)]      pub sleep: types::c::SleepModeDetection,
    /// SPI fault detection. Corresponds to `SPIFLT`.
    #[bits(1, default = types::c::SpiFault::DEFAULT)]                pub spiflt: types::c::SpiFault,
    /// Comparison active indicator. Corresponds to `COMP`.
    #[bits(1, default = types::c::ComparisonActive::DEFAULT)]        pub comp: types::c::ComparisonActive,
    /// Supply rail delta. Corresponds to `VDE`.
    #[bits(1, default = types::c::SupplyRailDelta::DEFAULT)]         pub vde: types::c::SupplyRailDelta,
    /// Supply rail latent delta. Corresponds to `VDEL`.
    #[bits(1, default = types::c::SupplyRailDeltaLatent::DEFAULT)]   pub vdel: types::c::SupplyRailDeltaLatent,
    
    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}
/// Status Register Group C read with the ERR bit set (RDSTATCERR).
///
/// This has the same layout as `StatusC`, but the read forces a mismatch between the redundant
/// internal SPI slaves so `SPIFLT` reads as a fault. This proves the bit is not stuck low.
/// 
/// Note: You have to clear SPIFLT after calling this (just like you would any other time).
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StatusCErr(pub StatusC);

impl super::ReadableGroup for StatusCErr {
    const READ_COMMAND: commands::CommandFrame = commands::status::rdstatcerr().frame();
    fn from_bytes(bytes: [u8; super::GROUP_BYTES]) -> Self { Self(StatusC::from_bytes(bytes)) }
}


/// Status Register Group D (STATD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// See Table 92 on page 68 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::status::rdstatd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct StatusD {
    /// Cell 1 undervoltage flag. Corresponds to `C1UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c1uv: types::d::CellUndervoltageFlag,
    /// Cell 1 overvoltage flag. Corresponds to `C1OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c1ov: types::d::CellOvervoltageFlag,
    /// Cell 2 undervoltage flag. Corresponds to `C2UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c2uv: types::d::CellUndervoltageFlag,
    /// Cell 2 overvoltage flag. Corresponds to `C2OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c2ov: types::d::CellOvervoltageFlag,
    /// Cell 3 undervoltage flag. Corresponds to `C3UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c3uv: types::d::CellUndervoltageFlag,
    /// Cell 3 overvoltage flag. Corresponds to `C3OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c3ov: types::d::CellOvervoltageFlag,
    /// Cell 4 undervoltage flag. Corresponds to `C4UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c4uv: types::d::CellUndervoltageFlag,
    /// Cell 4 overvoltage flag. Corresponds to `C4OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c4ov: types::d::CellOvervoltageFlag,

    /// Cell 5 undervoltage flag. Corresponds to `C5UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c5uv: types::d::CellUndervoltageFlag,
    /// Cell 5 overvoltage flag. Corresponds to `C5OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c5ov: types::d::CellOvervoltageFlag,
    /// Cell 6 undervoltage flag. Corresponds to `C6UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c6uv: types::d::CellUndervoltageFlag,
    /// Cell 6 overvoltage flag. Corresponds to `C6OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c6ov: types::d::CellOvervoltageFlag,
    /// Cell 7 undervoltage flag. Corresponds to `C7UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c7uv: types::d::CellUndervoltageFlag,
    /// Cell 7 overvoltage flag. Corresponds to `C7OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c7ov: types::d::CellOvervoltageFlag,
    /// Cell 8 undervoltage flag. Corresponds to `C8UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c8uv: types::d::CellUndervoltageFlag,
    /// Cell 8 overvoltage flag. Corresponds to `C8OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c8ov: types::d::CellOvervoltageFlag,

    /// Cell 9 undervoltage flag. Corresponds to `C9UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c9uv: types::d::CellUndervoltageFlag,
    /// Cell 9 overvoltage flag. Corresponds to `C9OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c9ov: types::d::CellOvervoltageFlag,
    /// Cell 10 undervoltage flag. Corresponds to `C10UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c10uv: types::d::CellUndervoltageFlag,
    /// Cell 10 overvoltage flag. Corresponds to `C10OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c10ov: types::d::CellOvervoltageFlag,
    /// Cell 11 undervoltage flag. Corresponds to `C11UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c11uv: types::d::CellUndervoltageFlag,
    /// Cell 11 overvoltage flag. Corresponds to `C11OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c11ov: types::d::CellOvervoltageFlag,
    /// Cell 12 undervoltage flag. Corresponds to `C12UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c12uv: types::d::CellUndervoltageFlag,
    /// Cell 12 overvoltage flag. Corresponds to `C12OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c12ov: types::d::CellOvervoltageFlag,

    /// Cell 13 undervoltage flag. Corresponds to `C13UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c13uv: types::d::CellUndervoltageFlag,
    /// Cell 13 overvoltage flag. Corresponds to `C13OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c13ov: types::d::CellOvervoltageFlag,
    /// Cell 14 undervoltage flag. Corresponds to `C14UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c14uv: types::d::CellUndervoltageFlag,
    /// Cell 14 overvoltage flag. Corresponds to `C14OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c14ov: types::d::CellOvervoltageFlag,
    /// Cell 15 undervoltage flag. Corresponds to `C15UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c15uv: types::d::CellUndervoltageFlag,
    /// Cell 15 overvoltage flag. Corresponds to `C15OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c15ov: types::d::CellOvervoltageFlag,
    /// Cell 16 undervoltage flag. Corresponds to `C16UV`.
    #[bits(1, default = types::d::CellUndervoltageFlag::DEFAULT)] pub c16uv: types::d::CellUndervoltageFlag,
    /// Cell 16 overvoltage flag. Corresponds to `C16OV`.
    #[bits(1, default = types::d::CellOvervoltageFlag::DEFAULT)]  pub c16ov: types::d::CellOvervoltageFlag,

    /// Byte 4 is just a reserved byte of all 1s
    #[bits(8, default = 0xFF)]                                     _reserved: u8,

    /// Oscillator check coutner. Corresponds to `OC_CNTR[7:0]`.
    #[bits(8, default = types::d::OscillatorCheckCounter::DEFAULT)]  pub oc_cntr: types::d::OscillatorCheckCounter,
    
    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}

/// Status Register Group E (STATE). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Even though this is 6 bytes, only like 2 of the bytes actually have stuff in them. The rest is reserved.
/// 
/// See Table 93 on page 68 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::status::rdstate().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct StatusE {
    /// Reserved.
    #[bits(8, default = 0xFF)] _ster0: u8,
    /// Reserved.
    #[bits(8, default = 0xFF)] _ster1: u8,
    /// Reserved.
    #[bits(8, default = 0xFF)] _ster2: u8,

    /// Reserved.
    /// This is bit 0 in STER3, which unlike the rest of STER3 (which is all 1s), looks like it could be either 0 or 1 based
    /// on the datasheet. I am using `default = 0` here for software purposes but I guess it is kind of up in the air
    /// 
    /// This probably doesn't matter since this is a reserved bit anyway, but it might as well be distinguished from
    /// the rest of STER3 in case it becomes important in the future
    #[bits(1, default = 0)] _ster3_first_bit: u8,
    /// Reserved.
    #[bits(7, default = 0x7F)] _ster3_except_for_the_first_bit: u8,

    /// GPIO 1 pin state. Corresponds to `GPI[1]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi1: types::e::GpioPinState,
    /// GPIO 2 pin state. Corresponds to `GPI[2]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi2: types::e::GpioPinState,
    /// GPIO 3 pin state. Corresponds to `GPI[3]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi3: types::e::GpioPinState,
    /// GPIO 4 pin state. Corresponds to `GPI[4]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi4: types::e::GpioPinState,
    /// GPIO 5 pin state. Corresponds to `GPI[5]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi5: types::e::GpioPinState,
    /// GPIO 6 pin state. Corresponds to `GPI[6]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi6: types::e::GpioPinState,
    /// GPIO 7 pin state. Corresponds to `GPI[7]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi7: types::e::GpioPinState,
    /// GPIO 8 pin state. Corresponds to `GPI[8]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi8: types::e::GpioPinState,

    /// GPIO 9 pin state. Corresponds to `GPI[9]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi9: types::e::GpioPinState,
    /// GPIO 10 pin state. Corresponds to `GPI[10]`.
    #[bits(1, default = types::e::GpioPinState::DEFAULT)]  pub gpi10: types::e::GpioPinState,
    /// Reserved.
    #[bits(2, default = 0)] _b2_b3: u8,
    /// Device revision code. Corresponds to `REV[3:0]`.
    #[bits(4, default = types::e::RevisionCode::DEFAULT)]  pub rev: types::e::RevisionCode,

    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}