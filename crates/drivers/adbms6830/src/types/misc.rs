//! Miscellaneous registers: serial ID (read-only) and 48-bit retention
//! register (read/write through `ULRR` + `WRRR`).

use bitfield_struct::bitfield;

use crate::registers::{AdbmsCommand, AdbmsReadableRegister, AdbmsWritableRegister};

/// Serial ID Register Group (`RDSID`).
///
/// 48 bits total: a unique per-chip ID plus a 6-bit device identification
/// (datasheet Table 54). The whole field is exposed as a single `u64` for
/// downstream hashing / logging; [`SerialId::device_id`] extracts the 6
/// identification bits (expected value `0b000011` for the ADBMS6830B).
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SerialId {
    /// Raw 48-bit ID, byte 0 in the low bits and byte 5 in the high bits.
    #[bits(48)]
    pub raw: u64,
    #[bits(16)]
    __: u16,
}

impl SerialId {
    /// Datasheet device identification, `SIDR1`.  The
    /// ADBMS6830B returns `0b000011`.
    pub const fn device_id(&self) -> u8 {
        // SIDR1
        ((self.raw() >> 9) & 0x3F) as u8
    }
}

impl AdbmsReadableRegister for SerialId {
    const READ_CMD: [u8; 2] = [0x00, 0x2C];
}

pub struct Ulrr {}

impl AdbmsCommand for Ulrr {
    fn get_command(&self) -> u16 {
        0b00000111000
    }
}

/// Retention Register Group (`ULRR` / `WRRR` / `RDRR`).
///
/// 48 bits of scratch storage that survives sleep. Writes require the
/// register to be unlocked via the `ULRR` command first.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct Retention {
    /// Raw 48-bit retention data.
    #[bits(48)]
    pub raw: u64,
    #[bits(16)]
    __: u16,
}

impl AdbmsWritableRegister for Retention {
    const WRITE_CMD: [u8; 2] = [0x00, 0x39];
}
impl AdbmsReadableRegister for Retention {
    const READ_CMD: [u8; 2] = [0x00, 0x3A];
}
