//! Command opcodes, response markers, and protocol limits.

// The first request byte selects a command; remaining bytes are its arguments.
pub const GET_INFO: u8 = 0xA1;
pub const GET_STATUS: u8 = 0xA2;
pub const START_APP: u8 = 0xA4;
pub const WRITE_MEMORY: u8 = 0xA5;
pub const ACTIVATE: u8 = 0xA7;
pub const COMPUTE_CRC: u8 = 0xA8;
pub const START_UPDATE: u8 = 0xA9;

// Responses prefix the command opcode with ACK or NACK.
pub const ACK: u8 = 0x5A;
pub const NACK: u8 = 0xB5;

// Version packs the major and minor numbers into the high and low nibbles.
pub const VERSION: u8 = 0x10;
// Keep this count consistent with the GET_INFO opcode list.
pub const COMMAND_COUNT: u8 = 7;

// Maximum host transfer block, in bytes.
pub const MAX_WRITE: usize = 256;
// Maximum silence between packets of an in-progress WRITE_MEMORY command.
pub const WRITE_TIMEOUT_MS: u64 = 5000;
// Bound hardware transmit draining when the bus is unavailable.
pub const TX_TIMEOUT_MS: u64 = 1000;
