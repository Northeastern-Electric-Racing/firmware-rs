//! Driver for the Analog Devices ADBMS2950B battery monitor.
//!
//! The ADBMS2950B is a current and battery-voltage monitor, not a cell monitor. It measures two
//! 24-bit current channels across a shunt (I1ADC/I2ADC), two battery voltages (VB1ADC/VB2ADC),
//! accumulates all four for coulomb counting, and carries a standalone overcurrent comparator
//! subsystem with its own alert pins.
//!
//! Modules, outermost first:
//!
//! - [`api`]: **start here.** A stateful layer over the transport that tracks the command
//!   counter, tallies PEC failures, caches the configuration registers for read-modify-write,
//!   and wraps each conversion into a single trigger-and-poll call.
//! - [`mod@line`]: the transport for one isoSPI line. Frames a transaction and hands back bytes,
//!   remembering nothing between calls. [`api`] is built on this, and most applications should
//!   use [`api`] rather than reaching for it directly -- see the [`mod@line`] module docs for the
//!   cases where it is the right choice.
//! - [`chip`]: basically a PAC. The types representing the register and command schemas from
//!   the datasheet. Both layers above are built from these.
//!
//! Note you cannot accidentally mix [`api`] and [`mod@line`]: [`api::Api::new`] takes the lines
//! **by value**, so once an `Api` exists the `Line`s have moved into it and cannot be reached.
//!
#![no_std]

pub mod api;
pub mod chip;
pub mod line;
