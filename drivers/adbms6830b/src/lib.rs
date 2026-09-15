//! Driver for the ADBMS6830B battery monitor.
//!
//! Summary of modules:
//! - `chip`: kind of like a PAC? contains the types representing the register and command schemas from the datasheet.
//! - `line`: the transport for one isoSPI line of daisy-chained devices. See the `Line` struct.
//! - `turnkey`: higher-level module for the two-line isoSPI configuration. it manages two `Line` instances and handles line splitting and state tracking
//! under the hood. See the `Service` struct.
//!
//! ### Defmt Support
//! - The `defmt` feature flag will implement `defmt::Format` for every public type this driver exposes. This makes
//! it so you can log register groups, field values, errors, etc. directly. To enable, add this
//! to your Cargo.toml:
//! ```toml
//! adbms6830b = { path = "...", features = ["defmt"] }
//! ```

#![no_std]

#[rustfmt::skip]
pub mod chip;
#[rustfmt::skip]
pub mod turnkey;
#[rustfmt::skip]
pub mod line;
#[rustfmt::skip]
mod docs;
