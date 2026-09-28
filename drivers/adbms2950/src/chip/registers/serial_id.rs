//! Register Layouts and Bit Descriptions for the Serial ID Register Group.
//!
//! The "main" struct here (i.e., the struct representing the overall register group) is `SerialId`.
//!
//! This group is a single 48-bit unique serial identifier, read with `RDSID`. Six of its bits
//! (`SID[46:41]`) double as the device derivative identifier, which makes this the cheapest way
//! to confirm you are actually talking to an ADBMS2950B.
//!
//! For more info about these registers, see Table 76 on page 61 of the datasheet
//! (the register map) and Table 77 on page 62 of the datasheet (the bit descriptions).

use bitfield_struct::bitfield;

use super::super::commands;
use super::register_group;

/// Device derivative identifier (`DEVID`) for the ADBMS2950B.
///
/// See `DEVID` in Table 77 on page 62 of the datasheet.
pub const ADBMS2950B_DEVICE_ID: u8 = 0b00_0110;

/// Serial ID Register Group.
///
/// See Table 76 on page 61 of the datasheet for the register map.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::misc::rdsid().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct SerialId {
    /// `SID[47:0]`, the 48-bit unique serial identifier. Read only.
    #[bits(48, default = 0)]  pub sid: u64,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]  _padding: u16,
}

impl SerialId {
    /// The device derivative identifier, `DEVID[5:0]`, which is `SID[46:41]`.
    pub const fn device_id(&self) -> u8 {
        ((self.sid() >> 41) & 0b11_1111) as u8
    }

    /// Whether this serial ID reports an ADBMS2950B.
    pub const fn is_adbms2950b(&self) -> bool {
        self.device_id() == ADBMS2950B_DEVICE_ID
    }
}
