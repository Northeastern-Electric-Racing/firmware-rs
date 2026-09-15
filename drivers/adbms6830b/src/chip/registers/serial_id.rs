//! Serial ID Registers.

use bitfield_struct::{bitfield};
use super::register_group;
use super::super::commands;

/// Serial ID register group. Contains the 48-bit SID[47:0] field.
/// 
/// This value is a constant identifier for the chip. You can read it whenever you want to
/// determine the exact chip you are talking to. If this value ever comes back unexpected at runtime,
/// either some transmission error corrupted the reading (although this should be caught by the PEC check, which
/// would report `PecStatus::Failed` for that chip's response), or you're not reading the chip(s) you are expecting
/// (or in the order you are expecting).
/// 
/// To get the overall unique ID, use the `.sid()` function. The `.device_id()` and `.is_adbms6830b()` functions
/// may also be useful if you need to confirm that a chip is in-fact an ADBMS6830B and not some other chip/variant.
/// 
/// See Table 53 and Table 54 on page 61 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::misc::rdsid().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct SerialId {
    /// This chip's 48-bit Serial ID value. Corresponds to `SID[47:0]`.
    #[bits(48, default = 0x00, access = RO)]  pub sid: u64,
    #[bits(16, default = 0)]                  _padding: u16,
}

impl SerialId {
    /// Device ID (`SID[14:9]`, aka SIDR1 bits `[6:1]`). See Table 54 on page 61 of the datasheet.
    /// 
    /// This constant is a subsection of the overall 48-bit `.sid()` field. However, it is constant for
    /// all ADBMS6830B chips (whereas the rest of the `.sid()` field) should vary for each unique chip.
    /// Because of this, it can serve as a quick sanity check to make sure you're actually talking to
    /// an ADBMS6830B. See the `.device_id()` and `is_adbms6830b()` functions.
    pub const ADBMS6830B_DEVICE_ID: u8 = 0b00_0011;

    /// Returns the device ID subsection of the chip's serial ID.
    /// 
    /// This should always be equal to `ADBMS6830B_DEVICE_ID`, assuming the chip
    /// you are talking to is an ADBMS6830B.
    pub const fn device_id(&self) -> u8 {
        ((self.sid() >> 9) & 0x3F) as u8
    }

    /// Checks whether this chip's device ID matches the ADBMS6830B.
    pub const fn is_adbms6830b(&self) -> bool {
        self.device_id() == Self::ADBMS6830B_DEVICE_ID
    }
}