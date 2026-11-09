#![no_std]

//! CAN firmware-update bootloader logic shared across boards.
//! Boards provide the CAN transport, flash partitions, `memory.x`, and application handoff.

mod flash;
mod handlers;
mod image;
mod link;
mod session;
mod transport;
mod types;

pub mod protocol;

pub use image::active_address;
pub use session::run;
pub use transport::{BootloaderCanIds, Frame, Transport};
pub use types::BootAction;
