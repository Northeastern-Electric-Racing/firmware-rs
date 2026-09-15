//! Register Layouts and Bit Descriptions for the Result Register groups (cell voltages, averaged cell voltages, filtered cell voltages, etc).
//! 
//! For more info about these registers, see Table 104 on page 71 of the datasheet
//! and Tables 57 through 88 on pages 61 through 67 of the datasheet.
//! 
//! ### Getting Started/Cool Tips
//! - If you want to read all cell voltages, send a `CellVoltagesAllReadRequest`, and then read in the response
//! as a `CellVoltagesAllReadResponse`.
//! - There's also `AverageCellVoltagesReadRequest`/`AverageCellVoltagesReadResponse`, `FilteredCellVoltagesReadRequest`/`FilteredCellVoltagesReadResponse`, etc.
//! - If you want to read a specific cell register group, there's types like `CellVoltagesAReadRequest`/`CellVoltagesAReadResponse`, `CellVoltagesBReadRequest`/`CellVoltagesBReadResponse`, etc.

use bitfield_struct::{bitfield};

use super::{register_group, register_group_aggregate};
use super::super::commands;

/// Field types relavent to the Result Register groups. See Table 104 on page 71 of the datasheet.
pub mod types {
    use crate::chip::registers::table107::{impl_firstrowregister, impl_vpvinner};

    impl_firstrowregister!(
        /// Represents a cell voltage result (CxV). The voltage represented by this struct can be returned via `as_microvolts()`.
        ///
        /// This is a 16-bit ADC measurement value for Cell `x`. Cell voltage for Cell `x` = CxV x 150uV + 1.5V.
        /// CxV is reset to 0x8000 on power-up and after clear command (CLRCELL), which corresponds to -3,415,200 uV / -3.4152 V.
        CellVoltage,
        0x8000
    );

    impl_firstrowregister!(
        /// Represents an average cell voltage result (ACxV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit average of 8 conversion results for value Cell `x`. Averaged Cell voltage for Cell `x` = CxV x 150uV + 1.5V.
        /// ACxV is reset to 0x8000 on power-up and after clear command (CLRCELL), which corresponds to -3,415,200 uV / -3.4152 V.
        AverageCellVoltage,
        0x8000
    );

    impl_firstrowregister!(
        /// Represents an filtered cell voltage result (FCxV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit IIR filtered measurement value for Cell `x`. Filtered Cell voltage for Cell `x` = CxV x 150uV + 1.5V.
        /// FCxV is reset to 0x8000 on power-up and after clear command (CLRCELL), which corresponds to -3,415,200 uV / -3.4152 V.
        FilteredCellVoltage,
        0x8000
    );

    impl_firstrowregister!(
        /// Represents an S-pin voltage result (SxV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit ADC measurement value for Sx pin from ADSV or ADCV commands. S-pin voltage for channel `x` = SxV x 150uV + 1.5V.
        /// SxV is reset to 0x8000 on power-up and after clear command (CLRCELL), which corresponds to -3,415,200 uV / -3.4152 V.
        SVoltage,
        0x8000
    );

    impl_firstrowregister!(
        /// Represents a GPIO voltage result (GxV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit ADC measurement value for GPIOx voltage for GPIOx = GxV x 150 uV + 1.5 V.
        /// 
        /// Note: This type is essentially identical to `RedundantGpioVoltage`, and only exists for verbosity and to be
        /// consistent with the datasheet. So, `GpioVoltage` and `RedundantGpioVoltage` both implement `From` so they can be converted to and from each other
        /// at will. Importantly, even though these two types are identical in what kind of value they represent, they are not identical in meaning and
        /// come from different register groups.
        /// 
        /// Also, if you're in a `const` context, prefer using `from_redundant()` instead of the `From`/`.into()` trait stuff.
        GpioVoltage,
        0x8000
    );
    impl From<RedundantGpioVoltage> for GpioVoltage {
        fn from(value: RedundantGpioVoltage) -> Self {
            Self::from_redundant(value)
        }
    }
    impl GpioVoltage {
        /// Creates a `GpioVoltage` from a `RedundantGpioVoltage`.
        pub const fn from_redundant(value: RedundantGpioVoltage) -> Self {
            GpioVoltage::from_bits(value.into_bits())
        }
    }

    impl_firstrowregister!(
        /// Represents a Redundant GPIO voltage result (R_GxV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit ADC measurement value for GPIOx voltage for GPIOx = GxV x 150 uV + 1.5 V.
        /// 
        /// Note: This type is essentially identical to `GpioVoltage`, and only exists for verbosity and to be
        /// consistent with the datasheet. So, `GpioVoltage` and `RedundantGpioVoltage` both implement `From` so they can be converted to and from each other
        /// at will. Importantly, even though these two types are identical in what kind of value they represent, they are not identical in meaning and
        /// come from different register groups.
        /// 
        /// Also, if you're in a `const` context, prefer using `from_standard()` instead of the `From`/`.into()` trait stuff.
        RedundantGpioVoltage,
        0x8000
    );
    impl From<GpioVoltage> for RedundantGpioVoltage {
        fn from(value: GpioVoltage) -> Self {
            Self::from_standard(value)
        }
    }
    impl RedundantGpioVoltage {
        /// Creates a `RedundantGpioVoltage` from a standard `GpioVoltage`.
        pub const fn from_standard(value: GpioVoltage) -> Self {
            RedundantGpioVoltage::from_bits(value.into_bits())
        }
    }

    impl_firstrowregister!(
        /// Represents a voltage measurement from S1N to V- (VMV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit ADC measurement value for S1N to V- = VMV x 150 uV + 1.5 V).
        /// 
        /// Reset to 0x8000 after power-up, sleep, or clear command (CLRAUX).
        VMinusVoltage,
        0x8000
    );

    impl_vpvinner!(
        /// Represents a voltage measurement from V+ to V- voltage result (VPV). The voltage represented by this struct can be returned via `as_microvolts()`.
        /// 
        /// This is a 16-bit ADC measurement value for V+ to V- = 25 x (VPV x 150 uV + 1.5 V).
        /// 
        /// Reset to 0x8000 after power-up, sleep, or clear command (CLRAUX).
        VPlusVoltage,
        0x8000
    );
}

/// Cell Voltage Register Group A (CVA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 1 through 3.
/// 
/// See Table 57 on page 61 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::cell_voltage::rdcva().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct CellVoltagesA {
    /// Cell 1 Voltage Result. Corresponds to `C1V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c1v: types::CellVoltage,
    /// Cell 2 Voltage Result. Corresponds to `C2V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c2v: types::CellVoltage,
    /// Cell 3 Voltage Result. Corresponds to `C3V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c3v: types::CellVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Cell Voltage Register Group B (CVB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 4 through 6.
/// 
/// See Table 58 on page 61 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::cell_voltage::rdcvb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct CellVoltagesB {
    /// Cell 4 Voltage Result. Corresponds to `C4V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c4v: types::CellVoltage,
    /// Cell 5 Voltage Result. Corresponds to `C5V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c5v: types::CellVoltage,
    /// Cell 6 Voltage Result. Corresponds to `C6V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c6v: types::CellVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Cell Voltage Register Group C (CVC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 7 through 9.
/// 
/// See Table 59 on page 62 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::cell_voltage::rdcvc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct CellVoltagesC {
    /// Cell 7 Voltage Result. Corresponds to `C7V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c7v: types::CellVoltage,
    /// Cell 8 Voltage Result. Corresponds to `C8V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c8v: types::CellVoltage,
    /// Cell 9 Voltage Result. Corresponds to `C9V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c9v: types::CellVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Cell Voltage Register Group D (CVD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 10 through 12.
/// 
/// See Table 60 on page 62 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::cell_voltage::rdcvd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct CellVoltagesD {
    /// Cell 10 Voltage Result. Corresponds to `C10V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c10v: types::CellVoltage,
    /// Cell 11 Voltage Result. Corresponds to `C11V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c11v: types::CellVoltage,
    /// Cell 12 Voltage Result. Corresponds to `C12V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c12v: types::CellVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Cell Voltage Register Group E (CVE). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 13 through 15.
/// 
/// See Table 61 on page 62 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::cell_voltage::rdcve().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct CellVoltagesE {
    /// Cell 13 Voltage Result. Corresponds to `C13V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c13v: types::CellVoltage,
    /// Cell 14 Voltage Result. Corresponds to `C14[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c14v: types::CellVoltage,
    /// Cell 15 Voltage Result. Corresponds to `C15V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c15v: types::CellVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Cell Voltage Register Group F (CVF). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Just contains cell 16.
/// 
/// See Table 62 on page 62 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::cell_voltage::rdcvf().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct CellVoltagesF {
    /// Cell 16 Voltage Result. Corresponds to `C16V[15:0]`.
    #[bits(16, default = types::CellVoltage::DEFAULT)]  pub c16v: types::CellVoltage,
    #[bits(32, default = u32::MAX)]                     _reserved: u32,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// All cell voltage results (Cell Voltage Register Group A through F).
#[register_group_aggregate(
    write = None,
    read = Some(commands::cell_voltage::rdcvall().frame()),
)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CellVoltagesAll {
    /// Cells 1–3 (Cell Voltage Register Group A).
    pub a: CellVoltagesA,
    /// Cells 4–6 (Cell Voltage Register Group B).
    pub b: CellVoltagesB,
    /// Cells 7–9 (Cell Voltage Register Group C).
    pub c: CellVoltagesC,
    /// Cells 10–12 (Cell Voltage Register Group D).
    pub d: CellVoltagesD,
    /// Cells 13–15 (Cell Voltage Register Group E).
    pub e: CellVoltagesE,
    /// Cell 16 (Cell Voltage Register Group F).
    /// 
    /// There's only 1 cell in Cell Voltage Register Group F, so this type has to be `types::CellVoltage` instead of `CellVoltagesF`.
    /// Otherwise the serialization would get messed up. Sad! This is bothering me but I may have OCD.
    pub f: types::CellVoltage,
}

/// Avergage Cell Voltage Register Group A (ACA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 1-3.
/// 
/// See Table 63 on page 62 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::avg_cell_voltage::rdaca().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AverageCellVoltagesA {
    /// Cell 1 Average Voltage Result. Corresponds to `AC1V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac1v: types::AverageCellVoltage,
    /// Cell 2 Average Voltage Result. Corresponds to `AC2V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac2v: types::AverageCellVoltage,
    /// Cell 3 Average Voltage Result. Corresponds to `AC3V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac3v: types::AverageCellVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// Avergage Cell Voltage Register Group B (ACB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 4-6.
/// 
/// See Table 64 on page 63 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::avg_cell_voltage::rdacb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AverageCellVoltagesB {
    /// Cell 4 Average Voltage Result. Corresponds to `AC4V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac4v: types::AverageCellVoltage,
    /// Cell 5 Average Voltage Result. Corresponds to `AC5V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac5v: types::AverageCellVoltage,
    /// Cell 6 Average Voltage Result. Corresponds to `AC6V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac6v: types::AverageCellVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// Avergage Cell Voltage Register Group C (ACC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 7-9.
/// 
/// See Table 65 on page 63 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::avg_cell_voltage::rdacc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AverageCellVoltagesC {
    /// Cell 7 Average Voltage Result. Corresponds to `AC7V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac7v: types::AverageCellVoltage,
    /// Cell 8 Average Voltage Result. Corresponds to `AC8V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac8v: types::AverageCellVoltage,
    /// Cell 9 Average Voltage Result. Corresponds to `AC9V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac9v: types::AverageCellVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// Avergage Cell Voltage Register Group D (ACD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 10-12.
/// 
/// See Table 66 on page 63 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::avg_cell_voltage::rdacd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AverageCellVoltagesD {
    /// Cell 10 Average Voltage Result. Corresponds to `AC10V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac10v: types::AverageCellVoltage,
    /// Cell 11 Average Voltage Result. Corresponds to `AC11V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac11v: types::AverageCellVoltage,
    /// Cell 12 Average Voltage Result. Corresponds to `AC12V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac12v: types::AverageCellVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// Avergage Cell Voltage Register Group E (ACE). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 13-15.
/// 
/// See Table 67 on page 63 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::avg_cell_voltage::rdace().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AverageCellVoltagesE {
    /// Cell 13 Average Voltage Result. Corresponds to `AC13V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac13v: types::AverageCellVoltage,
    /// Cell 14 Average Voltage Result. Corresponds to `AC14V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac14v: types::AverageCellVoltage,
    /// Cell 15 Average Voltage Result. Corresponds to `AC15V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac15v: types::AverageCellVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// Average Cell Voltage Register Group F (ACF). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Just contains cell 16.
/// 
/// See Table 68 on page 63 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::avg_cell_voltage::rdacf().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AverageCellVoltagesF {
    /// Cell 16 Average Voltage Result. Corresponds to `AC16V[15:0]`.
    #[bits(16, default = types::AverageCellVoltage::DEFAULT)]  pub ac16v: types::AverageCellVoltage,
    #[bits(32, default = u32::MAX)]                            _reserved: u32,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// All average cell voltage results (Average Cell Voltage Register Group A through F).
#[register_group_aggregate(
    write = None,
    read = Some(commands::avg_cell_voltage::rdacall().frame()),
)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AverageCellVoltagesAll {
    /// Cells 1–3 (Average Cell Voltage Register Group A).
    pub a: AverageCellVoltagesA,
    /// Cells 4–6 (Average Cell Voltage Register Group B).
    pub b: AverageCellVoltagesB,
    /// Cells 7–9 (Average Cell Voltage Register Group C).
    pub c: AverageCellVoltagesC,
    /// Cells 10–12 (Average Cell Voltage Register Group D).
    pub d: AverageCellVoltagesD,
    /// Cells 13–15 (Average Cell Voltage Register Group E).
    pub e: AverageCellVoltagesE,
    /// Cell 16 (Average Cell Voltage Register Group F).
    /// 
    /// There's only 1 cell in Average Cell Voltage Register Group F, so this type has to be `types::AverageCellVoltage` instead of `AverageCellVoltagesF`.
    /// Otherwise the serialization would get messed up. Sad! This is bothering me but I may have OCD.
    pub f: types::AverageCellVoltage,
}

/// Filtered Cell Voltage Register Group A (FCA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 1-3.
/// 
/// See Table 69 on page 63 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfca().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct FilteredCellVoltagesA {
    /// Cell 1 Filtered Voltage Result. Corresponds to `FC1V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc1v: types::FilteredCellVoltage,
    /// Cell 2 Filtered Voltage Result. Corresponds to `FC2V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc2v: types::FilteredCellVoltage,
    /// Cell 3 Filtered Voltage Result. Corresponds to `FC3V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc3v: types::FilteredCellVoltage,
    #[bits(16, default = 0)]                                    _padding: u16,
}

/// Filtered Cell Voltage Register Group B (FCB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 4-6.
/// 
/// See Table 70 on page 64 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfcb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct FilteredCellVoltagesB {
    /// Cell 4 Filtered Voltage Result. Corresponds to `FC4V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc4v: types::FilteredCellVoltage,
    /// Cell 5 Filtered Voltage Result. Corresponds to `FC5V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc5v: types::FilteredCellVoltage,
    /// Cell 6 Filtered Voltage Result. Corresponds to `FC6V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc6v: types::FilteredCellVoltage,
    #[bits(16, default = 0)]                                    _padding: u16,
}

/// Filtered Cell Voltage Register Group C (FCC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 7-9.
/// 
/// See Table 71 on page 64 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfcc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct FilteredCellVoltagesC {
    /// Cell 7 Filtered Voltage Result. Corresponds to `FC7V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc7v: types::FilteredCellVoltage,
    /// Cell 8 Filtered Voltage Result. Corresponds to `FC8V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc8v: types::FilteredCellVoltage,
    /// Cell 9 Filtered Voltage Result. Corresponds to `FC9V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc9v: types::FilteredCellVoltage,
    #[bits(16, default = 0)]                                    _padding: u16,
}

/// Filtered Cell Voltage Register Group D (FCD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 10-12.
/// 
/// See Table 72 on page 64 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfcd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct FilteredCellVoltagesD {
    /// Cell 10 Filtered Voltage Result. Corresponds to `FC10V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc10v: types::FilteredCellVoltage,
    /// Cell 11 Filtered Voltage Result. Corresponds to `FC11V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc11v: types::FilteredCellVoltage,
    /// Cell 12 Filtered Voltage Result. Corresponds to `FC12V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc12v: types::FilteredCellVoltage,
    #[bits(16, default = 0)]                                    _padding: u16,
}

/// Filtered Cell Voltage Register Group E (FCE). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains cells 13-15.
/// 
/// See Table 73 on page 64 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfce().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct FilteredCellVoltagesE {
    /// Cell 13 Filtered Voltage Result. Corresponds to `FC13V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc13v: types::FilteredCellVoltage,
    /// Cell 14 Filtered Voltage Result. Corresponds to `FC14V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc14v: types::FilteredCellVoltage,
    /// Cell 15 Filtered Voltage Result. Corresponds to `FC15V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc15v: types::FilteredCellVoltage,
    #[bits(16, default = 0)]                                    _padding: u16,
}

/// Filtered Cell Voltage Register Group F (FCF). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Just contains cell 16.
/// 
/// See Table 74 on page 64 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfcf().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct FilteredCellVoltagesF {
    /// Cell 16 Filtered Voltage Result. Corresponds to `FC16V[15:0]`.
    #[bits(16, default = types::FilteredCellVoltage::DEFAULT)]  pub fc16v: types::FilteredCellVoltage,
    #[bits(32, default = u32::MAX)]                     _reserved: u32,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// All filtered cell voltage results (Filtered Cell Voltage Register Group A through F).
#[register_group_aggregate(
    write = None,
    read = Some(commands::filtered_cell_voltage::rdfcall().frame()),
)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FilteredCellVoltagesAll {
    /// Cells 1–3 (Filtered Cell Voltage Register Group A).
    pub a: FilteredCellVoltagesA,
    /// Cells 4–6 (Filtered Cell Voltage Register Group B).
    pub b: FilteredCellVoltagesB,
    /// Cells 7–9 (Filtered Cell Voltage Register Group C).
    pub c: FilteredCellVoltagesC,
    /// Cells 10–12 (Filtered Cell Voltage Register Group D).
    pub d: FilteredCellVoltagesD,
    /// Cells 13–15 (Filtered Cell Voltage Register Group E).
    pub e: FilteredCellVoltagesE,
    /// Cell 16 (Filtered Cell Voltage Register Group F).
    /// 
    /// There's only 1 cell in Filtered Cell Voltage Register Group F, so this type has to be `types::FilteredCellVoltage` instead of `FilteredCellVoltagesF`.
    /// Otherwise the serialization would get messed up. Sad! This is bothering me but I may have OCD.
    pub f: types::FilteredCellVoltage,
}

/// S-Voltage Register Group A (SCA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains S pins 1-3.
/// 
/// See Table 75 on page 65 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::s_voltage::rdsva().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SVoltagesA {
    /// Cell 1 S-Voltage Result. Corresponds to `S1V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s1v: types::SVoltage,
    /// Cell 2 S-Voltage Result. Corresponds to `S2V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s2v: types::SVoltage,
    /// Cell 3 S-Voltage Result. Corresponds to `S3V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s3v: types::SVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// S-Voltage Register Group B (SCB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains S pins 4-6.
/// 
/// See Table 76 on page 65 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::s_voltage::rdsvb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SVoltagesB {
    /// Cell 4 S-Voltage Result. Corresponds to `S4V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s4v: types::SVoltage,
    /// Cell 5 S-Voltage Result. Corresponds to `S5V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s5v: types::SVoltage,
    /// Cell 6 S-Voltage Result. Corresponds to `S6V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s6v: types::SVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// S-Voltage Register Group C (SCC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains S pins 7-9.
/// 
/// See Table 77 on page 65 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::s_voltage::rdsvc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SVoltagesC {
    /// Cell 7 S-Voltage Result. Corresponds to `S7V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s7v: types::SVoltage,
    /// Cell 8 S-Voltage Result. Corresponds to `S8V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s8v: types::SVoltage,
    /// Cell 9 S-Voltage Result. Corresponds to `S9V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s9v: types::SVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// S-Voltage Register Group D (SCD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains S pins 10-12.
/// 
/// See Table 78 on page 65 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::s_voltage::rdsvd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SVoltagesD {
    /// Cell 10 S-Voltage Result. Corresponds to `S10V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s10v: types::SVoltage,
    /// Cell 11 S-Voltage Result. Corresponds to `S11V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s11v: types::SVoltage,
    /// Cell 12 S-Voltage Result. Corresponds to `S12V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s12v: types::SVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// S-Voltage Register Group E (SCE). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains S pins 13-15.
/// 
/// See Table 79 on page 65 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::s_voltage::rdsve().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SVoltagesE {
    /// Cell 13 S-Voltage Result. Corresponds to `S13V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s13v: types::SVoltage,
    /// Cell 14 S-Voltage Result. Corresponds to `S14V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s14v: types::SVoltage,
    /// Cell 15 S-Voltage Result. Corresponds to `S15V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s15v: types::SVoltage,
    #[bits(16, default = 0)]                                   _padding: u16,
}

/// S-Voltage Register Group F (FCF). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Just S pin 16.
/// 
/// See Table 80 on page 66 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::s_voltage::rdsvf().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SVoltagesF {
    /// Cell 16 S-Voltage Result. Corresponds to `S16V[15:0]`.
    #[bits(16, default = types::SVoltage::DEFAULT)]  pub s16v: types::SVoltage,
    #[bits(32, default = u32::MAX)]                  _reserved: u32,
    #[bits(16, default = 0)]                         _padding: u16,
}

/// All S-voltage results (S-Voltage Register Group A through F).
#[register_group_aggregate(
    write = None,
    read = Some(commands::s_voltage::rdsall().frame()),
)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SVoltagesAll {
    /// Cells/Pins 1–3 (S-Voltage Register Group A).
    pub a: SVoltagesA,
    /// Cells/Pins 4–6 (S-Voltage Register Group B).
    pub b: SVoltagesB,
    /// Cells/Pins 7–9 (S-Voltage Register Group C).
    pub c: SVoltagesC,
    /// Cells/Pins 10–12 (S-Voltage Register Group D).
    pub d: SVoltagesD,
    /// Cells 13–15 (S-Voltage Register Group E).
    pub e: SVoltagesE,
    /// Cell/Pin 16 (S-Voltage Register Group F).
    /// 
    /// There's only 1 cell in S-Voltage Register Group F, so this type has to be `types::SVoltage` instead of `SVoltagesF`.
    /// Otherwise the serialization would get messed up. Sad! This is bothering me but I may have OCD.
    pub f: types::SVoltage,
}

/// Auxillary Register Group A (AUXA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIOs 1-3.
/// 
/// See Table 81 on page 66 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::aux::rdauxa().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AuxillaryA {
    /// GPIO 1 Voltage Result. Corresponds to `G1V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g1v: types::GpioVoltage,
    /// GPIO 2 Voltage Result. Corresponds to `G2V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g2v: types::GpioVoltage,
    /// GPIO 3 Voltage Result. Corresponds to `G3V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g3v: types::GpioVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Auxillary Register Group B (AUXB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIOs 4-6.
/// 
/// See Table 82 on page 66 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::aux::rdauxb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AuxillaryB {
    /// GPIO 4 Voltage Result. Corresponds to `G4V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g4v: types::GpioVoltage,
    /// GPIO 5 Voltage Result. Corresponds to `G5V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g5v: types::GpioVoltage,
    /// GPIO 6 Voltage Result. Corresponds to `G6V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g6v: types::GpioVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Auxillary Register Group C (AUXC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIOs 7-9.
/// 
/// See Table 83 on page 66 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::aux::rdauxc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AuxillaryC {
    /// GPIO 7 Voltage Result. Corresponds to `G7V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g7v: types::GpioVoltage,
    /// GPIO 8 Voltage Result. Corresponds to `G8V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g8v: types::GpioVoltage,
    /// GPIO 9 Voltage Result. Corresponds to `G9V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]  pub g9v: types::GpioVoltage,
    #[bits(16, default = 0)]                            _padding: u16,
}

/// Auxillary Register Group D (AUXD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIO 10, and VMV/VPV.
/// 
/// See Table 84 on page 66 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::aux::rdauxd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct AuxillaryD {
    /// GPIO 10 Voltage Result. Corresponds to `G10V[15:0]`.
    #[bits(16, default = types::GpioVoltage::DEFAULT)]    pub g10v: types::GpioVoltage,
    /// VMV Voltage Result. Corresponds to `VMV[15:0]`.
    #[bits(16, default = types::VMinusVoltage::DEFAULT)]  pub vmv: types::VMinusVoltage,
    /// VPV Voltage Result. Corresponds to `VPV[15:0]`.
    #[bits(16, default = types::VPlusVoltage::DEFAULT)]   pub vpv: types::VPlusVoltage,
    #[bits(16, default = 0)]                              _padding: u16,
}

/// Redundant Auxillary Register Group A (RAXA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIOs 1-3.
/// 
/// See Table 85 on page 66 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::redundant_aux::rdraxa().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct RedundantAuxillaryA {
    /// GPIO 1 Voltage Result. Corresponds to `R_G1V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g1v: types::RedundantGpioVoltage,
    /// GPIO 2 Voltage Result. Corresponds to `R_G2V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g2v: types::RedundantGpioVoltage,
    /// GPIO 3 Voltage Result. Corresponds to `R_G3V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g3v: types::RedundantGpioVoltage,
    #[bits(16, default = 0)]                                     _padding: u16,
}

/// Redundant Auxillary Register Group B (RAXB). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIOs 4-6.
/// 
/// See Table 86 on page 67 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::redundant_aux::rdraxb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct RedundantAuxillaryB {
    /// GPIO 4 Voltage Result. Corresponds to `R_G4V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g4v: types::RedundantGpioVoltage,
    /// GPIO 5 Voltage Result. Corresponds to `R_G5V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g5v: types::RedundantGpioVoltage,
    /// GPIO 6 Voltage Result. Corresponds to `R_G6V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g6v: types::RedundantGpioVoltage,
    #[bits(16, default = 0)]                                     _padding: u16,
}

/// Redundant Auxillary Register Group C (RAXC). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Contains GPIOs 7-9.
/// 
/// See Table 87 on page 67 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::redundant_aux::rdraxc().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct RedundantAuxillaryC {
    /// GPIO 7 Voltage Result. Corresponds to `R_G7V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g7v: types::RedundantGpioVoltage,
    /// GPIO 8 Voltage Result. Corresponds to `R_G8V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g8v: types::RedundantGpioVoltage,
    /// GPIO 9 Voltage Result. Corresponds to `R_G9V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]  pub r_g9v: types::RedundantGpioVoltage,
    #[bits(16, default = 0)]                                     _padding: u16,
}

/// Redundant Auxillary Register Group D (RAXD). Contains six 1-byte registers (so 6 bytes total).
/// 
/// Just contains GPIO 10.
/// 
/// See Table 88 on page 67 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::redundant_aux::rdraxd().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct RedundantAuxillaryD {
    /// GPIO 10 Voltage Result. Corresponds to `R_G10V[15:0]`.
    #[bits(16, default = types::RedundantGpioVoltage::DEFAULT)]    pub r_g10v: types::RedundantGpioVoltage,
    #[bits(32, default = u32::MAX)]                                _reserved: u32,
    #[bits(16, default = 0)]                                       _padding: u16,
}