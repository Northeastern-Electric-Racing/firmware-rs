//! Register Layouts and Bit Descriptions for the FLAG Register Group.
//!
//! The "main" struct here (i.e., the struct representing the overall register group) is `Flag`.
//!
//! FLAG stores fault and status information. Almost every bit is a sticky `RW1C` latch: it stays
//! set once the fault is seen, and the host clears it by writing a 1 to that position with a
//! `CLRFLAG` command *after* the underlying condition is gone.
//!
//! Two things about this group are easy to get wrong:
//!
//! 1. **The fault latches reset to `1`, not `0`.** Per the Reset column of Table 74, every latch
//!    here powers up asserted (the one exception is `thsd`, which resets to `0`). So a freshly
//!    reset device reports essentially every fault until the host issues a `CLRFLAG`. Treat
//!    `Flag::new()` as "everything latched", not "all clear".
//! 2. **`I1CNT` is split non-contiguously.** `I1CNT[10:6]` lives in byte 2 and `I1CNT[5:0]` in
//!    byte 3, with `I2CNT` and `I1PHA` in between, so it cannot be one bitfield. Use the
//!    `i1cnt()` accessor rather than reading the halves.
//!
//! Not every bit indicates a fault, and the overcurrent-related bits are deliberately *not*
//! perfectly synchronized with each other (they have slightly different propagation delays) so
//! that they stay independent for diagnostic purposes.
//!
//! For more info about these registers, see Table 73 on page 59 of the datasheet
//! (the register map) and Table 74 on pages 60 and 61 of the datasheet (the bit descriptions).

use bitfield_struct::bitfield;

use super::super::commands;
use super::register_group;

/// FLAG Register Group.
///
/// See Table 73 on page 59 of the datasheet for the register map, and Table 74 on pages 60 and 61
/// for the bit descriptions.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = Some(commands::clear::clrflag().frame()),
    read = Some(commands::status::rdflag().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct Flag {
    // FLAG0! first byte of the register group.
    /// OC1ADC overcurrent latch (OC1L). Set when an OC1 threshold event was detected and passed
    /// deglitching.
    #[bits(1, default = true)]   pub oc1l: bool,
    /// Majority voter A overcurrent latch (OCAL). Set when an overcurrent event passed majority
    /// voter A.
    #[bits(1, default = true)]   pub ocal: bool,
    /// OCA gate/drain mismatch latch, and clear for the OC3 min/max registers (OCAGD/CLRM).
    ///
    /// Set when an unexpected readback from the OCA output was detected while `OCEN` was 1.
    /// Clearing this bit also clears the `OC3MIN` and `OC3MAX` registers.
    #[bits(1, default = true)]   pub ocagd_clrm: bool,
    /// OC3ADC overcurrent latch (OC3L). Set when an OC3 threshold event was detected and passed
    /// deglitching.
    #[bits(1, default = true)]   pub oc3l: bool,
    /// OC configuration mismatch latch (OCMM). Set on an internal mismatch between the redundant
    /// shadow copies of `OCEN`, `OCTSEL`, or `OCDGT`.
    #[bits(1, default = true)]   pub ocmm: bool,
    /// Drive undervoltage latch (VDRUV). Set on a DRIVE undervoltage event on the VDD supply.
    #[bits(1, default = true)]   pub vdruv: bool,
    /// Reserved. Table 73 marks these two bits RESERVED rather than as must-write-zero.
    #[bits(2, default = 0)]      _reserved0: u8,
    // FLAG1! second byte of the register group.
    /// OC2ADC overcurrent latch (OC2L). Set when an OC2 threshold event was detected and passed
    /// deglitching.
    #[bits(1, default = true)]   pub oc2l: bool,
    /// Majority voter B overcurrent latch (OCBL). Set when an overcurrent event passed majority
    /// voter B.
    #[bits(1, default = true)]   pub ocbl: bool,
    /// OCB gate/drain mismatch latch (OCBGD). Set when an unexpected readback from the OCB output
    /// was detected while `OCEN` was 1.
    #[bits(1, default = true)]   pub ocbgd: bool,
    /// OC fault latch (REFFLT).
    #[bits(1, default = true)]   pub refflt: bool,
    /// No clock fault latch (NOCLK). Set when an OSC1 stuck event was detected.
    #[bits(1, default = true)]   pub noclk: bool,
    /// VDD undervoltage latch (VDDUV).
    #[bits(1, default = true)]   pub vdduv: bool,
    /// Reserved. Table 73 marks these two bits RESERVED rather than as must-write-zero.
    #[bits(2, default = 0)]      _reserved1: u8,
    // FLAG2! third byte of the register group.
    /// Upper five bits of the conversion counter, `I1CNT[10:6]`. Read only.
    ///
    /// This is one half of a field that the register map splits across two bytes. Prefer the
    /// `i1cnt()` accessor.
    #[bits(5, default = 0)]      pub i1cnt_upper: u8,
    /// I2ADC/VB2ADC conversion counter (I2CNT). Three-bit field. Read only.
    ///
    /// Counts conversions while in continuous mode. Resets on an `ADI1` with `RD = 1` and on
    /// `ADI2` commands. Rolls over.
    #[bits(3, default = 0)]      pub i2cnt: u8,
    // FLAG3! fourth byte of the register group.
    /// I1ADC/VB1ADC phase counter (I1PHA). Two-bit field. Read only.
    ///
    /// Increments four times per sample. Resets on `ADI1` commands. Rolls over.
    #[bits(2, default = 0)]      pub i1pha: u8,
    /// Lower six bits of the conversion counter, `I1CNT[5:0]`. Read only.
    ///
    /// This is one half of a field that the register map splits across two bytes. Prefer the
    /// `i1cnt()` accessor.
    #[bits(6, default = 0)]      pub i1cnt_lower: u8,
    // FLAG4! fifth byte of the register group.
    /// NVM2 multi-bit ECC latch (MED2).
    #[bits(1, default = true)]   pub med2: bool,
    /// NVM2 one-bit ECC latch (SED2).
    #[bits(1, default = true)]   pub sed2: bool,
    /// NVM1 multi-bit ECC latch (MED1).
    #[bits(1, default = true)]   pub med1: bool,
    /// NVM1 one-bit ECC latch (SED1).
    #[bits(1, default = true)]   pub sed1: bool,
    /// VDIG undervoltage latch (VDIGUV).
    #[bits(1, default = true)]   pub vdiguv: bool,
    /// VDIG overvoltage latch (VDIGOV).
    #[bits(1, default = true)]   pub vdigov: bool,
    /// VREG undervoltage latch (VREGUV).
    #[bits(1, default = true)]   pub vreguv: bool,
    /// VREG overvoltage latch (VREGOV).
    #[bits(1, default = true)]   pub vregov: bool,
    // FLAG5! sixth byte of the register group.
    /// Oscillator frequency fault latch (OSCFLT). Set on an OSC1 versus OSC2 frequency comparison
    /// fault.
    #[bits(1, default = true)]   pub oscflt: bool,
    /// Test mode indicator latch (TMODE). Set when activation of factory test mode was detected.
    #[bits(1, default = true)]   pub tmode: bool,
    /// Thermal shutdown indicator latch (THSD).
    ///
    /// Unlike every other latch in this group this resets to `0`, and it is **not** cleared by
    /// `SRST`. See `injts` in CFGA for the logging requirement that comes with forcing it.
    #[bits(1, default = false)]  pub thsd: bool,
    /// Reset indicator latch (RESET). Set when a reset event occurred since the last clear.
    #[bits(1, default = true)]   pub reset: bool,
    /// SPI read fault latch (SPIFLT). Set on an SPI SDO mismatch.
    #[bits(1, default = true)]   pub spiflt: bool,
    /// Reserved; must be written as 0.
    #[bits(1, default = 0)]      _reserved2: u8,
    /// Voltage domain event latch (VDE). Set on a mismatch between the internal VREG or GND
    /// domains.
    #[bits(1, default = true)]   pub vde: bool,
    /// Voltage domain diagnostic latch (VDEL). Set when *all* VREG and GND domain comparators
    /// report a mismatch. See `injmon` in CFGA.
    #[bits(1, default = true)]   pub vdel: bool,
    // Padding to fill out the u64 backing value; not part of the 6 wire bytes.
    #[bits(16, default = 0)]     _padding: u16,
}

impl Flag {
    /// The full 11-bit I1ADC/VB1ADC conversion counter (`I1CNT[10:0]`).
    ///
    /// The register map splits this field across bytes 2 and 3 with other fields in between, so
    /// it can't be a single bitfield; this reassembles the two halves. Counts conversions while in
    /// continuous mode, resets on `ADI1` commands, and rolls over.
    pub const fn i1cnt(&self) -> u16 {
        ((self.i1cnt_upper() as u16) << 6) | self.i1cnt_lower() as u16
    }

    /// The combined 13-bit counter `I1CNTPHA[12:0]`, which is `[I1CNT, I1PHA]`.
    ///
    /// The datasheet notes in Table 74 that `I1CNT` and `I1PHA` can be treated as one counter,
    /// which is what this returns. Useful when you want the phase as sub-sample resolution rather
    /// than a separate value.
    pub const fn i1cntpha(&self) -> u16 {
        (self.i1cnt() << 2) | self.i1pha() as u16
    }
}
