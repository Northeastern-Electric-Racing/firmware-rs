//! Auxiliary measurement registers (`RDAUXx`, `RDRAXx`) and commands.

use bitfield_struct::bitfield;

use crate::registers::{AdbmsCommand, AdbmsReadableRegister};
use crate::types::{AdbmsCellVoltage, AdbmsChipVoltage};

///The CLRAUX command clears Auxiliary Register Group A through
/// Auxiliary Register Group D, the Redundant Auxiliary Register
/// Group A through Redundant Auxiliary Register Group D, and Status
/// Register Group A and Status Register Group B. All bytes in these
/// registers are set to 0x8000 by the CLRAUX command. Note that
/// this register value of 0x8000 resulting from a CLRAUX command is,
/// for some registers, different than their default value after power-up.
pub struct ClrAux {}

impl AdbmsCommand for ClrAux {
    fn get_command(&self) -> u16 {
        0b11100010010
    }
}

// =============================================================================
// AUX results — RDAUXx
// =============================================================================

/// Auxiliary Register Group A
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AuxA {
    #[bits(16)]
    pub g1v: AdbmsCellVoltage,
    #[bits(16)]
    pub g2v: AdbmsCellVoltage,
    #[bits(16)]
    pub g3v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AuxA {
    const READ_CMD: [u8; 2] = [0x00, 0x19];
}

/// Auxiliary Register Group B
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AuxB {
    #[bits(16)]
    pub g4v: AdbmsCellVoltage,
    #[bits(16)]
    pub g5v: AdbmsCellVoltage,
    #[bits(16)]
    pub g6v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AuxB {
    const READ_CMD: [u8; 2] = [0x00, 0x1A];
}

/// Auxiliary Register Group C
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AuxC {
    #[bits(16)]
    pub g7v: AdbmsCellVoltage,
    #[bits(16)]
    pub g8v: AdbmsCellVoltage,
    #[bits(16)]
    pub g9v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AuxC {
    const READ_CMD: [u8; 2] = [0x00, 0x1B];
}

/// Auxiliary Register Group D
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct AuxD {
    #[bits(16)]
    pub g10v: AdbmsCellVoltage,
    /// S1N to V− measurement
    ///
    /// 16-bit ADC measurement value of S1N to V− = VMV × 150 μV + 1.5 V.
    /// Reset to 0x8000 after power-up, sleep, or clear command (CLRAUX).
    #[bits(16)]
    pub vmv: AdbmsCellVoltage,
    /// V+ to V− measurement
    ///
    /// 16-bit ADC measurement value of V+ to V− = 25 × (VPV × 150 μV + 1.5 V).
    /// Reset to 0x8000 after power-up, sleep, or clear command (CLRAUX).
    #[bits(16)]
    pub vpv: AdbmsChipVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for AuxD {
    const READ_CMD: [u8; 2] = [0x00, 0x1F];
}

/// Run an adax command
pub struct Adax {
    /// Use Open Wire detect
    pub ow: bool,
    /// Open Wire detect pull up if true, pull down if false
    pub pup: bool,
    /// Selection of channels
    pub ch: AdaxChannels,
}

impl AdbmsCommand for Adax {
    fn get_command(&self) -> u16 {
        let mut buf: u16 = 0b10000010000;
        buf |= (self.ow as u16) << 9u16;
        buf |= (self.pup as u16) << 8u16;
        buf |= ((self.ch as u16) >> 4) << 7;
        buf |= (self.ch as u16) & 0b1111;
        buf
    }
}

/// All of the channels scannable by the Adax command/Aux register
#[repr(u16)]
#[derive(Clone, Copy)]
pub enum AdaxChannels {
    /// Scan every channel
    All = 0b00000,
    GPIO1,
    GPIO2,
    GPIO3,
    GPIO4,
    GPIO5,
    GPIO6,
    GPIO7,
    GPIO8,
    GPIO9,
    GPIO10,
    VREF2,
    VD,
    VA,
    ITEMP,
    VPV,
    VMV,
    VRES,
}

// =============================================================================
// Redundant AUX (AUX2-ADC) results
// =============================================================================

/// Redundant Auxiliary Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct RedundantAuxA {
    #[bits(16)]
    pub r_g1v: AdbmsCellVoltage,
    #[bits(16)]
    pub r_g2v: AdbmsCellVoltage,
    #[bits(16)]
    pub r_g3v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for RedundantAuxA {
    const READ_CMD: [u8; 2] = [0x00, 0x1C];
}

/// Redundant Auxiliary Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct RedundantAuxB {
    #[bits(16)]
    pub r_g4v: AdbmsCellVoltage,
    #[bits(16)]
    pub r_g5v: AdbmsCellVoltage,
    #[bits(16)]
    pub r_g6v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for RedundantAuxB {
    const READ_CMD: [u8; 2] = [0x00, 0x1D];
}

/// Redundant Auxiliary Register Group C.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct RedundantAuxC {
    #[bits(16)]
    pub r_g7v: AdbmsCellVoltage,
    #[bits(16)]
    pub r_g8v: AdbmsCellVoltage,
    #[bits(16)]
    pub r_g9v: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for RedundantAuxC {
    const READ_CMD: [u8; 2] = [0x00, 0x1E];
}

/// Redundant Auxiliary Register Group D.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct RedundantAuxD {
    #[bits(16)]
    pub r_g10v: AdbmsCellVoltage,
    #[bits(32)]
    __: u32,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for RedundantAuxD {
    const READ_CMD: [u8; 2] = [0x00, 0x25];
}

/// Run an adax2 command (read aux2)
pub struct Adax2 {
    /// Selection of channels
    pub ch: Adax2Channels,
}

impl AdbmsCommand for Adax2 {
    fn get_command(&self) -> u16 {
        let mut buf: u16 = 0b10000000000;
        buf |= self.ch as u16;
        buf
    }
}

/// All of the channels scannable by the Adax2 command/Redudant aux register
#[repr(u16)]
#[derive(Clone, Copy)]
pub enum Adax2Channels {
    /// Scan every channel
    All = 0b00000,
    GPIO1,
    GPIO2,
    GPIO3,
    GPIO4,
    GPIO5,
    GPIO6,
    GPIO7,
    GPIO8,
    GPIO9,
    GPIO10,
}
