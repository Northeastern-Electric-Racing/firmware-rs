//! Driver for the ADBMS6830 chips
//!
//! This uses the `embedded-hal` and `embedded-hal-async` crates to implement interfaces for data.
//!
//! At this time, the driver requires full use of the bus and CS Pins.
//! Support for `SpiDevice` and therefore a shared bus via Mutex is planned but low-priority.
//!
//! Unsupported API Features and not planned
//!  - Read all registers
//!  - Dual 2950/6830 chains
//!  - IC count being non-const
//!
#![no_std]

pub mod client;
pub mod registers;
pub mod types;
