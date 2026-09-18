//! Driver for the Analog Devices ADBMS2950B battery monitor.
//!
//! The ADBMS2950B is a current and battery-voltage monitor, not a cell monitor. It measures two
//! 24-bit current channels across a shunt (I1ADC/I2ADC), two battery voltages (VB1ADC/VB2ADC),
//! accumulates all four for coulomb counting, and carries a standalone overcurrent comparator
//! subsystem with its own alert pins.
//!
//! Modules:
//! - `chip`: Basically a PAC. Contains the types representing the register and command schemas
//!   from the datasheet.
//!
//! ### Relationship to the ADBMS6830B driver
//! This chip speaks the same isoSPI protocol as the ADBMS6830B, with the same command framing,
//! the same PEC15/PEC10 algorithms, and the same 6-byte register groups. It is nonetheless a
//! completely separate crate on purpose, because *the two chips reuse the same opcodes for
//! different registers*: `0x004` is `RDCVA` (cell voltages) on the ADBMS6830B but `RDI` (currents)
//! here, `0x030` is `RDSTATA` there but `RDXA` here, `0x711` is `CLRCELL` there but `CLRI` here.
//! Around 35 opcodes collide like this. Sharing a command module between the two drivers would
//! compile perfectly well and talk to the wrong register, so we don't.
#![no_std]

pub mod chip;
pub mod line;
