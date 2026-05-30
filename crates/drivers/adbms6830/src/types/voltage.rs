//! Cell-voltage result register groups.

use bitfield_struct::bitfield;

use crate::registers::{AdbmsCommand, AdbmsReadableRegister};
use crate::types::AdbmsCellVoltage;

pub struct Adcv {
    pub rd: bool,
    pub dcp: bool,
    pub cont: bool,
    pub rstf: bool,
    pub ow: OwSettings,
}

impl AdbmsCommand for Adcv {
    fn get_command(&self) -> u16 {
        let mut buf: u16 = 0b0100110000;
        buf |= (self.rd as u16) << 8;
        buf |= (self.cont as u16) << 7;
        buf |= (self.dcp as u16) << 4;
        buf |= (self.rstf as u16) << 2;
        buf |= self.ow as u16;
        buf
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum OwSettings {
    Off = 0b00,
    EvenOn = 0b01,
    OddOn = 0b10,
    BothOn = 0b11,
}

pub struct Adsv {
    pub dcp: bool,
    pub cont: bool,
    pub ow: OwSettings,
}

impl AdbmsCommand for Adsv {
    fn get_command(&self) -> u16 {
        let mut buf: u16 = 0b00101101000;
        buf |= (self.cont as u16) << 7;
        buf |= (self.dcp as u16) << 4;
        buf |= self.ow as u16;
        buf
    }
}

pub struct ClrCell {}
impl AdbmsCommand for ClrCell {
    fn get_command(&self) -> u16 {
        0b11100010001
    }
}

pub struct ClrFc {}
impl AdbmsCommand for ClrFc {
    fn get_command(&self) -> u16 {
        0b11100010100
    }
}

pub struct ClrSpin {}
impl AdbmsCommand for ClrSpin {
    fn get_command(&self) -> u16 {
        0b11100010110
    }
}

// =============================================================================
// Cell voltage — RDCVx
// =============================================================================

/// Cell Voltage Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CellVoltageA {
    #[bits(16)]
    pub c1v: AdbmsCellVoltage,
    #[bits(16)]
    pub c2v: AdbmsCellVoltage,
    #[bits(16)]
    pub c3v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for CellVoltageA {
    const READ_CMD: [u8; 2] = [0x00, 0x04];
}

/// Cell Voltage Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CellVoltageB {
    #[bits(16)]
    pub c4v: AdbmsCellVoltage,
    #[bits(16)]
    pub c5v: AdbmsCellVoltage,
    #[bits(16)]
    pub c6v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for CellVoltageB {
    const READ_CMD: [u8; 2] = [0x00, 0x06];
}

/// Cell Voltage Register Group C.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CellVoltageC {
    #[bits(16)]
    pub c7v: AdbmsCellVoltage,
    #[bits(16)]
    pub c8v: AdbmsCellVoltage,
    #[bits(16)]
    pub c9v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for CellVoltageC {
    const READ_CMD: [u8; 2] = [0x00, 0x08];
}

/// Cell Voltage Register Group D.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CellVoltageD {
    #[bits(16)]
    pub c10v: AdbmsCellVoltage,
    #[bits(16)]
    pub c11v: AdbmsCellVoltage,
    #[bits(16)]
    pub c12v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for CellVoltageD {
    const READ_CMD: [u8; 2] = [0x00, 0x0A];
}

/// Cell Voltage Register Group E.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CellVoltageE {
    #[bits(16)]
    pub c13v: AdbmsCellVoltage,
    #[bits(16)]
    pub c14v: AdbmsCellVoltage,
    #[bits(16)]
    pub c15v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for CellVoltageE {
    const READ_CMD: [u8; 2] = [0x00, 0x09];
}

/// Cell Voltage Register Group F.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CellVoltageF {
    #[bits(16)]
    pub c16v: AdbmsCellVoltage,
    #[bits(32)]
    __: u32,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for CellVoltageF {
    const READ_CMD: [u8; 2] = [0x00, 0x0B];
}

// =============================================================================
// Averaged cell voltage
// =============================================================================

/// Averaged Cell Voltage Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AvgCellVoltageA {
    #[bits(16)]
    pub ac1v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac2v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac3v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AvgCellVoltageA {
    const READ_CMD: [u8; 2] = [0x00, 0x44];
}

/// Averaged Cell Voltage Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AvgCellVoltageB {
    #[bits(16)]
    pub ac4v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac5v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac6v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AvgCellVoltageB {
    const READ_CMD: [u8; 2] = [0x00, 0x46];
}

/// Averaged Cell Voltage Register Group C.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AvgCellVoltageC {
    #[bits(16)]
    pub ac7v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac8v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac9v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AvgCellVoltageC {
    const READ_CMD: [u8; 2] = [0x00, 0x48];
}

/// Averaged Cell Voltage Register Group D.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AvgCellVoltageD {
    #[bits(16)]
    pub ac10v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac11v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac12v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AvgCellVoltageD {
    const READ_CMD: [u8; 2] = [0x00, 0x4A];
}

/// Averaged Cell Voltage Register Group E.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AvgCellVoltageE {
    #[bits(16)]
    pub ac13v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac14v: AdbmsCellVoltage,
    #[bits(16)]
    pub ac15v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AvgCellVoltageE {
    const READ_CMD: [u8; 2] = [0x00, 0x49];
}

/// Averaged Cell Voltage Register Group F.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AvgCellVoltageF {
    #[bits(16)]
    pub ac16v: AdbmsCellVoltage,
    #[bits(32)]
    __: u32,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AvgCellVoltageF {
    const READ_CMD: [u8; 2] = [0x00, 0x4B];
}

// =============================================================================
// Filtered cell voltage
// =============================================================================

/// Filtered Cell Voltage Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct FilteredCellVoltageA {
    #[bits(16)]
    pub fc1v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc2v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc3v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for FilteredCellVoltageA {
    const READ_CMD: [u8; 2] = [0x00, 0x12];
}

/// Filtered Cell Voltage Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct FilteredCellVoltageB {
    #[bits(16)]
    pub fc4v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc5v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc6v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for FilteredCellVoltageB {
    const READ_CMD: [u8; 2] = [0x00, 0x13];
}

/// Filtered Cell Voltage Register Group C.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct FilteredCellVoltageC {
    #[bits(16)]
    pub fc7v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc8v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc9v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for FilteredCellVoltageC {
    const READ_CMD: [u8; 2] = [0x00, 0x14];
}

/// Filtered Cell Voltage Register Group D.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct FilteredCellVoltageD {
    #[bits(16)]
    pub fc10v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc11v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc12v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for FilteredCellVoltageD {
    const READ_CMD: [u8; 2] = [0x00, 0x15];
}

/// Filtered Cell Voltage Register Group E.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct FilteredCellVoltageE {
    #[bits(16)]
    pub fc13v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc14v: AdbmsCellVoltage,
    #[bits(16)]
    pub fc15v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for FilteredCellVoltageE {
    const READ_CMD: [u8; 2] = [0x00, 0x16];
}

/// Filtered Cell Voltage Register Group F.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct FilteredCellVoltageF {
    #[bits(16)]
    pub fc16v: AdbmsCellVoltage,
    #[bits(32)]
    __: u32,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for FilteredCellVoltageF {
    const READ_CMD: [u8; 2] = [0x00, 0x17];
}

// =============================================================================
// S-pin voltage
// =============================================================================

/// S-Voltage Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SVoltageA {
    #[bits(16)]
    pub s1v: AdbmsCellVoltage,
    #[bits(16)]
    pub s2v: AdbmsCellVoltage,
    #[bits(16)]
    pub s3v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for SVoltageA {
    const READ_CMD: [u8; 2] = [0x00, 0x03];
}

/// S-Voltage Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SVoltageB {
    #[bits(16)]
    pub s4v: AdbmsCellVoltage,
    #[bits(16)]
    pub s5v: AdbmsCellVoltage,
    #[bits(16)]
    pub s6v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for SVoltageB {
    const READ_CMD: [u8; 2] = [0x00, 0x05];
}

/// S-Voltage Register Group C.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SVoltageC {
    #[bits(16)]
    pub s7v: AdbmsCellVoltage,
    #[bits(16)]
    pub s8v: AdbmsCellVoltage,
    #[bits(16)]
    pub s9v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for SVoltageC {
    const READ_CMD: [u8; 2] = [0x00, 0x07];
}

/// S-Voltage Register Group D.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SVoltageD {
    #[bits(16)]
    pub s10v: AdbmsCellVoltage,
    #[bits(16)]
    pub s11v: AdbmsCellVoltage,
    #[bits(16)]
    pub s12v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for SVoltageD {
    const READ_CMD: [u8; 2] = [0x00, 0x0D];
}

/// S-Voltage Register Group E.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SVoltageE {
    #[bits(16)]
    pub s13v: AdbmsCellVoltage,
    #[bits(16)]
    pub s14v: AdbmsCellVoltage,
    #[bits(16)]
    pub s15v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for SVoltageE {
    const READ_CMD: [u8; 2] = [0x00, 0x0E];
}

/// S-Voltage Register Group F.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SVoltageF {
    #[bits(16)]
    pub s16v: AdbmsCellVoltage,
    #[bits(32)]
    __: u32,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for SVoltageF {
    const READ_CMD: [u8; 2] = [0x00, 0x0F];
}
