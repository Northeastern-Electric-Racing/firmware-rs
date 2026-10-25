//! Driver for the Analog Devices ADBMS2950B battery monitor.
//!
//! Not a cell monitor: it measures two 24-bit current channels across a shunt (I1ADC/I2ADC), two
//! battery voltages (VB1ADC/VB2ADC), accumulates all four for coulomb counting, and carries a
//! standalone overcurrent comparator subsystem with its own alert pins.
//!
//! Summary of modules:
//! - `api`: **start here, and stay here.** The stateful layer over the transport: command counter,
//! PEC tallies, cached configuration for read-modify-write, startup waits, and each conversion as
//! a single trigger-and-poll call. See the `Api` struct.
//! - `chip`: kind of like a PAC? contains the types representing the register and command schemas from the datasheet.
//! - `spi_timing`: the datasheet's Table 14, and which layer owns each constraint.
//!
//! The SPI transport is a private `line` module. `Api` builds and owns both isoSPI lines from the
//! devices you hand it, so there is nothing out here to construct or get wrong. Its public types
//! -- `Error`, `Response`, `conversion_times` -- are re-exported from `api`.
//!
//! ### Defmt Support
//! - The `defmt` feature flag will implement `defmt::Format` for every public type this driver exposes. This makes
//! it so you can log register groups, field values, errors, etc. directly. To enable, add this
//! to your Cargo.toml:
//! ```toml
//! adbms2950 = { path = "...", features = ["defmt"] }
//! ```

#![no_std]

pub mod api;
pub mod chip;
pub mod spi_timing;

mod line;
