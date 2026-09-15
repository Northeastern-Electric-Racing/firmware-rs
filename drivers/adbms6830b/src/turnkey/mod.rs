//! "Turnkey" implementation of the isoSPI manager based on the two-line ADBMS6830B configuration.
//! 
//! Basically, this module models a "manager" that owns two instances of the `Line` driver
//! and manages the line splitting and chip metadata (i.e., PEC and command counts) automatically
//! under the hood.
//! 
//! This all is exposed via the `service` module.

pub mod api;
pub mod diagnostics;
pub mod service;
mod accumulator;