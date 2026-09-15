//! Big submodule for modelling the chip's hardware interface.
//! 
//! Basically, this crate provides the types that map to the
//! chip's registers and commands, mostly taken straight from the datasheet and encoded into Rust. The full ADBMS6830B driver can use this interface to interact with registers
//! and such without doing any manual bit fiddling.
//! 
//! In other words, it's kind of like a PAC, but not really since this is an external peripheral.

// This is being used since the [b7:b0] syntax for describing bit fields (used a lot in this module) is interpreted as broken intradoc
// links by rustdoc 
#![allow(rustdoc::broken_intra_doc_links)]

#[rustfmt::skip]
pub mod commands;
#[rustfmt::skip]
pub mod pec;
#[rustfmt::skip]
pub mod registers;