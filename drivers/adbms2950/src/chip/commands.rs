//! Mapping for the Command Codes seen in Table 33 of the datasheet (page 25).

#![allow(dead_code)]

use bitfield_struct::bitfield;

/// CC[10:0] - Command Code. 11-bit field.
///
/// See Table 29 on page 23 of the datasheet.
#[bitfield(u16, defmt = cfg(feature = "defmt"))]
#[derive(PartialEq, Eq)]
pub struct CommandCode {
    /// CC[10:0]
    #[bits(11)]
    pub code: u16,
    /// Reserved bits, since this is only an 11-bit field.
    #[bits(5)]
    _reserved: u8,
}

/// ADBMS2950B Command. See Table 33 on page 25 of the datasheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Command {
    /// Whether or not the command counter increments for the command.
    ///
    /// Unlike the ADBMS6830B datasheet, Table 33 has no INC column. The rule is stated in prose in
    /// the "Command Counter" section on page 24 instead: the counter increments when the device
    /// receives "a command without data, which are the 4-byte commands like SNAP, UNSNAP, and
    /// ADI1, or ... a command with write data like WRCFGA and CLRFLAG". So reads don't increment,
    /// and neither do RSTCC and SRST, which reset the counter to 0 instead.
    inc: bool,
    /// The 11-bit CC[10:0] field for the command.
    code: CommandCode,
}

/// A Command Frame (the command code plus its command PEC). This is four bytes total.
/// (technically six bytes because `Command` has the inc metadata but to_bytes() serializes it into four bytes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CommandFrame {
    command: Command,
    pec: super::pec::CommandPec,
}
#[rustfmt::skip]
impl CommandFrame {
    /// Creates a `CommandFrame` from a `Command`, computing its command PEC.
    pub const fn from_command(command: &Command) -> Self {
        let pec = super::pec::CommandPec::new(&[command.cmd0(), command.cmd1()]);
        Self { command: *command, pec }
    }

    /// The command code (`CC[10:0]`).
    pub const fn code(&self) -> CommandCode { self.command.code() }

    /// The command PEC.
    pub const fn pec(&self) -> super::pec::CommandPec { self.pec }

    /// Converts a `CommandFrame` into bytes.
    pub const fn to_bytes(self) -> [u8; 4] {
        let code = self.command.code().into_bits();
        let cmd0 = (code >> 8) as u8 & 0x07;
        let cmd1 = (code & 0xFF) as u8;
        [cmd0, cmd1, self.pec.pec0(), self.pec.pec1()]
    }

    /// Whether or not this command increments the devices' command counters.
    pub const fn increments(&self) -> bool {
        self.command.increments()
    }

    /// Whether this command resets the devices' command counters to 0.
    pub const fn resets_counter(&self) -> bool {
        self.command.resets_counter()
    }
}

#[rustfmt::skip]
impl Command {
    /// Allows you to define a command. Ideally should be called in a const context.
    /// ## Parameters
    /// - `inc`: Whether or not the command counter increments for the command.
    /// - `code`: The 11-bit CC[10:0] field for the command.
    const fn define(inc: bool, code: u16) -> Self {
        Self {
            inc,
            code: CommandCode(code),
        }
    }

    /// Returns the 11-bit code associated with this command.
    pub const fn code(&self) -> CommandCode { self.code }

    /// Returns the CC[10:8] portion of the command code.
    pub const fn cmd0(&self) -> u8 {
        let code: u16 = self.code().into_bits();
        let cmd0: u8 = (code >> 8) as u8 & 0x07;
        cmd0
    }

    /// Returns the CC[7:0] portion of the command code.
    pub const fn cmd1(&self) -> u8 {
        let code: u16 = self.code().into_bits();
        let cmd1: u8 = (code & 0xFF) as u8;
        cmd1
    }

    /// Returns whether or not the command counter increments for this command.
    pub const fn increments(&self) -> bool { self.inc }

    /// Whether this command resets the devices' command counters to 0.
    ///
    /// RSTCC does it for obvious reasons. SRST does it as part of the software reset. See the
    /// "Command Counter" section on page 24 of the datasheet.
    pub const fn resets_counter(&self) -> bool {
        let code = self.code.into_bits();
        code == misc::rstcc().code.into_bits() || code == misc::srst().code.into_bits()
    }

    /// Returns this `Command` as a 4-byte command frame.
    pub const fn frame(&self) -> CommandFrame {
        CommandFrame::from_command(self)
    }
}

/// Configuration Registers Commands.
#[rustfmt::skip]
pub mod config {
    use super::Command;

    /// Write Configuration Register Group A
    pub const fn wrcfga() -> Command { Command::define(true, 0b00000000001) }
    /// Write Configuration Register Group B
    pub const fn wrcfgb() -> Command { Command::define(true, 0b00000100100) }
    /// Read Configuration Register Group A
    pub const fn rdcfga() -> Command { Command::define(false, 0b00000000010) }
    /// Read Configuration Register Group B
    pub const fn rdcfgb() -> Command { Command::define(false, 0b00000100110) }
}

/// Current and battery voltage result commands.
///
/// These read the IxADC and VBxADC result registers. See Table 43 on page 33 of the datasheet.
#[rustfmt::skip]
pub mod current {
    use super::Command;

    /// Read I1ADC and I2ADC Current Register Group
    pub const fn rdi() -> Command { Command::define(false, 0b00000000100) }
    /// Read VB1ADC and VB2ADC Battery Voltage Register Group
    pub const fn rdvb() -> Command { Command::define(false, 0b00000000110) }
    /// Read I1ADC and VB1ADC Register Group (one current plus one battery voltage)
    pub const fn rdivb1() -> Command { Command::define(false, 0b00000001000) }
    /// Read OCxADC Overcurrent Result Register Group
    pub const fn rdoc() -> Command { Command::define(false, 0b00000001011) }
}

/// Accumulated (coulomb counting) result commands.
///
/// See Table 44 on page 33 of the datasheet. The accumulation depth is set by `ACCI` in CFGA.
#[rustfmt::skip]
pub mod accumulated {
    use super::Command;

    /// Read I1ACC and I2ACC Accumulated Current Register Group
    pub const fn rdiacc() -> Command { Command::define(false, 0b00001000100) }
    /// Read VB1ACC and VB2ACC Accumulated Battery Voltage Register Group
    pub const fn rdvbacc() -> Command { Command::define(false, 0b00001000110) }
    /// Read I1ACC and VB1ACC Register Group
    pub const fn rdivb1acc() -> Command { Command::define(false, 0b00001001000) }
}

/// V1ADC voltage result commands. See Table 51 on page 39 of the datasheet.
#[rustfmt::skip]
pub mod voltage {
    use super::Command;

    /// Read V1ADC Voltage Register Group A
    pub const fn rdv1a() -> Command { Command::define(false, 0b00000001010) }
    /// Read V1ADC Voltage Register Group B
    pub const fn rdv1b() -> Command { Command::define(false, 0b00000001001) }
    /// Read V1ADC Voltage Register Group C
    pub const fn rdv1c() -> Command { Command::define(false, 0b00000000011) }
    /// Read V1ADC Voltage Register Group D
    pub const fn rdv1d() -> Command { Command::define(false, 0b00000011011) }
}

/// V2ADC (redundant) voltage result commands. See Table 51 on page 39 of the datasheet.
#[rustfmt::skip]
pub mod redundant_voltage {
    use super::Command;

    /// Read V2ADC Voltage Register Group A (called RDRVA in some ADI sources)
    pub const fn rdv2a() -> Command { Command::define(false, 0b00000000111) }
    /// Read V2ADC Voltage Register Group B
    pub const fn rdv2b() -> Command { Command::define(false, 0b00000001101) }
    /// Read V2ADC Voltage Register Group C
    pub const fn rdv2c() -> Command { Command::define(false, 0b00000000101) }
    /// Read V2ADC Voltage Register Group D
    pub const fn rdv2d() -> Command { Command::define(false, 0b00000011111) }
    /// Read V2ADC Voltage Register Group E
    pub const fn rdv2e() -> Command { Command::define(false, 0b00000100101) }
}

/// AUX ADC result commands. See Table 51 on page 39 of the datasheet.
#[rustfmt::skip]
pub mod aux {
    use super::Command;

    /// Read AUX ADC Register Group A (VREF1P25, TMP1, VREG)
    pub const fn rdxa() -> Command { Command::define(false, 0b00000110000) }
    /// Read AUX ADC Register Group B (VDD, VDIG, EPAD)
    pub const fn rdxb() -> Command { Command::define(false, 0b00000110001) }
    /// Read AUX ADC Register Group C (VDIV, TMP2, OSCCNT)
    pub const fn rdxc() -> Command { Command::define(false, 0b00000110011) }
}

/// "Read all" commands.
///
/// Unlike every other read, these return **20** bytes of register data (plus the command counter
/// and data PEC, for 22 on the wire) rather than 6. See Table 27 on page 22 of the datasheet.
#[rustfmt::skip]
pub mod read_all {
    use super::Command;

    /// Read all IxADC and VBxADC results, plus STATUS and FLAG
    pub const fn rdalli() -> Command { Command::define(false, 0b00000001100) }
    /// Read all IxACC and VBxACC results, plus STATUS and FLAG
    pub const fn rdalla() -> Command { Command::define(false, 0b00001001100) }
    /// Read all configuration registers, plus STATUS and FLAG
    pub const fn rdallc() -> Command { Command::define(false, 0b00000010000) }
    /// Read all V1ADC voltages
    pub const fn rdallv() -> Command { Command::define(false, 0b00000110101) }
    /// Read all V2ADC (redundant) voltages
    pub const fn rdallr() -> Command { Command::define(false, 0b00000010001) }
    /// Read all AUX ADC voltages
    pub const fn rdallx() -> Command { Command::define(false, 0b00001010001) }
}

/// Status and flag register commands.
#[rustfmt::skip]
pub mod status {
    use super::Command;

    /// Read STATUS Register Group
    pub const fn rdstat() -> Command { Command::define(false, 0b00000110100) }
    /// Read FLAG Register Group
    pub const fn rdflag() -> Command { Command::define(false, 0b00000110010) }
    /// Read FLAG Register Group with a deliberately corrupted data PEC.
    ///
    /// Per Table 33 on page 25 of the datasheet this isn't a separate command, but `RDFLAG` with
    /// its `ERR` bit (bit 6) set. Used to check that the host's PEC verification actually rejects
    /// bad frames.
    pub const fn rdflagerr() -> Command { Command::define(false, 0b00001110010) }
}

/// ADBMS6830B-compatibility command codes that have no effect on this chip.
///
/// Table 87 on page 71 of the datasheet lists command codes the ADBMS2950B accepts purely so that
/// host software written for the ADBMS6830B keeps working on a mixed bus. Quoting the datasheet,
/// these codes "do not have an internal effect on ADBMS2950B other than behaving as a read
/// command returning do not care data with a valid DPEC or behaving as a write command with the
/// data written being do not care and incrementing the command counter if the DPEC is valid."
///
/// In other words: they are well-formed no-ops. They *do* still bump the command counter, so a
/// host tracking `CCNT` has to account for them.
///
/// **There is no PWM register on the ADBMS2950B.** The `WRPWM`/`RDPWM` codes below exist only for
/// ADBMS6830B compatibility. What this chip calls PWM is unrelated: it is the duty cycle the chip
/// *drives* on the OCA and OCB pins to report overcurrent status, selected with `OCMODE` in CFGB
/// and decoded by an external timer or capture-compare unit. See Table 60 and Table 61 on pages
/// 47 and 48 of the datasheet.
///
/// Only the four PWM codes are exposed here, since they are the ones most likely to be reached
/// for by mistake. The rest of Table 87 (`CLOVUV`, `WRAO`, `RDAO`, the `CM*` cell-monitor codes,
/// `RDACF`, `RDSVE`) is deliberately left out.
#[rustfmt::skip]
pub mod compatibility {
    use super::Command;

    /// ADBMS6830B `WRPWMA`. No effect on the ADBMS2950B; the written data is ignored.
    pub const fn wrpwma() -> Command { Command::define(true, 0b00000100000) }
    /// ADBMS6830B `RDPWMA`. No effect on the ADBMS2950B; returns do-not-care data.
    pub const fn rdpwma() -> Command { Command::define(false, 0b00000100010) }
    /// ADBMS6830B `WRPWMB`. No effect on the ADBMS2950B; the written data is ignored.
    pub const fn wrpwmb() -> Command { Command::define(true, 0b00000100001) }
    /// ADBMS6830B `RDPWMB`. No effect on the ADBMS2950B; returns do-not-care data.
    pub const fn rdpwmb() -> Command { Command::define(false, 0b00000100011) }
}

/// Clear commands.
#[rustfmt::skip]
pub mod clear {
    use super::Command;

    /// Clear IxADC and VBxADC result registers
    pub const fn clri() -> Command { Command::define(true, 0b11100010001) }
    /// Clear V1ADC, V2ADC, and AUX ADC result registers
    pub const fn clrvx() -> Command { Command::define(true, 0b11100010010) }
    /// Clear OCxADC result registers
    pub const fn clro() -> Command { Command::define(true, 0b11100010011) }
    /// Clear accumulator (IxACC, VBxACC) registers
    pub const fn clra() -> Command { Command::define(true, 0b11100010100) }
    /// Clear the conversion counters
    pub const fn clrc() -> Command { Command::define(true, 0b11100010110) }
    /// Clear the FLAG register
    pub const fn clrflag() -> Command { Command::define(true, 0b11100010111) }
}

/// ADC conversion status polling commands.
#[rustfmt::skip]
pub mod poll {
    use super::Command;

    /// Poll any ADC conversion status
    pub const fn pladc() -> Command { Command::define(true, 0b11100011000) }
    /// Poll I1ADC conversion status
    pub const fn pli1() -> Command { Command::define(true, 0b11100011100) }
    /// Poll I2ADC conversion status
    pub const fn pli2() -> Command { Command::define(true, 0b11100011101) }
    /// Poll V1ADC/V2ADC conversion status
    pub const fn plv() -> Command { Command::define(true, 0b11100011110) }
    /// Poll AUX ADC conversion status
    pub const fn plx() -> Command { Command::define(true, 0b11100011111) }
}

/// I2C/SPI master (COMM register) commands.
#[rustfmt::skip]
pub mod comm {
    use super::Command;

    /// Write COMM Register Group
    pub const fn wrcomm() -> Command { Command::define(true, 0b11100100001) }
    /// Read COMM Register Group
    pub const fn rdcomm() -> Command { Command::define(false, 0b11100100010) }
    /// Start I2C/SPI Communication
    pub const fn stcomm() -> Command { Command::define(true, 0b11100100011) }
}

/// Control commands (serial ID, resets, snapshotting).
#[rustfmt::skip]
pub mod misc {
    use super::Command;

    /// Read Serial ID Register Group
    pub const fn rdsid() -> Command { Command::define(false, 0b00000101100) }
    /// Reset Command Counter
    pub const fn rstcc() -> Command { Command::define(false, 0b00000101110) }
    /// Software reset, including a command counter reset
    pub const fn srst() -> Command { Command::define(false, 0b00000100111) }
    /// Freeze the result registers so a set of them can be read coherently
    pub const fn snap() -> Command { Command::define(true, 0b00000101101) }
    /// Unfreeze the result registers
    pub const fn unsnap() -> Command { Command::define(true, 0b00000101111) }
}

/// ADC conversion start commands.
///
/// These are the only commands with option bits folded into the opcode. Table 33 on page 25 of the
/// datasheet spells out the bit positions directly: for `ADI1` the code reads
/// `0, 1, RD, OPT[3], 1, 1, OPT[2], 0, x, OPT[1], OPT[0]`, so `RD` is bit 8, `OPT[3]` is bit 7,
/// `OPT[2]` is bit 4, and `OPT[1:0]` are bits 1 and 0.
///
/// `OPT` is a four-bit code, but the datasheet describes it by its parts -- `OPT[3]` is CONT,
/// `OPT[2]` selects the diagnostic path, and `OPT[1:0]` is open wire -- so that's how it is
/// modelled here rather than as one 16-variant enum. Open wire requires the diagnostic bit,
/// which is why a nonzero open wire selection with `Diagnostic::Normal` is not a valid code.
pub mod adc {
    use super::Command;
    use bitfield_struct::bitfield;

    /// Redundancy (RD) for the ADI1 command. One-bit field.
    ///
    /// Unlike the ADBMS6830B's C-ADC/S-ADC pair, the ADBMS2950B's second current channel is an
    /// independent path rather than a redundant ADC, so `ADI2` has no RD bit at all.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Redundancy {
        /// Convert on the primary path only.
        Disabled = 0,
        /// Also trigger the redundant path (VB2ADC latches its mux select on this).
        Enabled = 1,
    }

    /// Whether a conversion runs once or repeats. Corresponds to `OPT[3]` (CONT).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Acquisition {
        /// Make a single measurement and then standby. Corresponds to `CONT = 0`.
        SingleShot,
        /// Measure continuously until stopped.
        ///
        /// The result registers update at the ADC's conversion rate (1 ms for the current
        /// registers, 8 ms for the averaged ones). To stop, send the same command again with
        /// `Acquisition::SingleShot`. Corresponds to `CONT = 1`.
        Continuous,
    }
    impl Acquisition {
        /// The `CONT` bit for this acquisition mode.
        const fn cont(self) -> u8 {
            match self {
                Self::Continuous => 1,
                Self::SingleShot => 0,
            }
        }
    }

    /// Whether the conversion runs through the diagnostic signal path. Corresponds to `OPT[2]`.
    ///
    /// Which diagnostic path is taken is selected by `DIAGSEL` in CFGB.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Diagnostic {
        /// Normal measurement.
        Normal = 0,
        /// Route the conversion through the diagnostic path selected by `DIAGSEL`.
        Enabled = 1,
    }

    /// Open wire excitation for the current inputs. Corresponds to `OPT[1:0]`. Two-bit field.
    ///
    /// Only valid with `Diagnostic::Enabled`; ADI marks the combinations with `OPT[2] = 0` and a
    /// nonzero open wire selection as invalid.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OpenWire {
        /// Open wire excitation off.
        Off = 0b00,
        /// Source current into the positive input, sink from the negative input.
        SourcePositiveSinkNegative = 0b01,
        /// Source current into the negative input, sink from the positive input.
        SourceNegativeSinkPositive = 0b10,
    }

    /// Open wire excitation for the voltage inputs, in the ADV command. One-bit-per-direction,
    /// occupying `CC[7:6]`.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OpenWireVoltage {
        /// Open wire excitation off.
        Off = 0b00,
        /// Source current into the positive input, sink from the negative input.
        SourcePositiveSinkNegative = 0b01,
        /// Source current into the negative input, sink from the positive input.
        SourceNegativeSinkPositive = 0b10,
    }

    /// Which voltage channel(s) the ADV command converts (`VCH[3:0]`). Four-bit field.
    ///
    /// The `Single*` variants convert one channel; the `RoundRobin*` variants sweep a range,
    /// converting each channel one after the other.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum VoltageChannel {
        /// Convert V1 only.
        SingleV1 = 0,
        /// Convert V2 only.
        SingleV2 = 1,
        /// Convert V3 only.
        SingleV3 = 2,
        /// Convert V4 only.
        SingleV4 = 3,
        /// Convert V5 only.
        SingleV5 = 4,
        /// Convert V6 only.
        SingleV6 = 5,
        /// Convert V7 (on V1ADC) and V9 (on V2ADC).
        SingleV7AndV9 = 6,
        /// Convert V8 (on V1ADC) and V10 (on V2ADC).
        SingleV8AndV10 = 7,
        /// Convert VREF2 only.
        SingleVref2 = 8,
        /// Round robin over channels 0 through 8.
        RoundRobinCh0ToCh8 = 9,
        /// Round robin over channels 1, 3, and 5.
        RoundRobinCh1Ch3Ch5 = 10,
        /// Round robin over channels 0 through 5.
        RoundRobinCh0ToCh5 = 11,
        /// Round robin over channels 0 through 3.
        RoundRobinCh0ToCh3 = 12,
        /// Round robin over channels 0, 2, and 4.
        RoundRobinCh0Ch2Ch4 = 13,
        /// Round robin over channels 4 through 7.
        RoundRobinCh4ToCh7 = 14,
        /// Round robin over channels 5 through 7.
        RoundRobinCh5ToCh7 = 15,
    }

    /// Start I1ADC (and VB1ADC) Conversion
    ///
    /// The VB1ADC converts alongside I1ADC and latches the `VB1MUX` bit from CFGA when this
    /// command arrives, so a `VB1MUX` change only takes effect on the next `adi1()`.
    pub const fn adi1(rd: Redundancy, acq: Acquisition, diag: Diagnostic, ow: OpenWire) -> Command {
        // Variable bits: [8]=RD, [7]=CONT, [4]=DIAG, [1]=OW[1], [0]=OW[0]
        let base: u16 = 0b01001100000;

        #[bitfield(u16)]
        struct Adi1 {
            #[bits(2)]
            pub b01_ow: u8,
            #[bits(2)]
            _b23: u8,
            #[bits(1)]
            pub b4_diag: u8,
            #[bits(2)]
            _b56: u8,
            #[bits(1)]
            pub b7_cont: u8,
            #[bits(1)]
            pub b8_rd: u8,
            #[bits(7)]
            _reserved: u8,
        }

        let mut base = Adi1::from_bits(base);
        base.set_b01_ow(ow as u8);
        base.set_b4_diag(diag as u8);
        base.set_b7_cont(acq.cont());
        base.set_b8_rd(rd as u8);

        Command::define(true, base.into_bits())
    }

    /// Start I2ADC (and VB2ADC) Conversion
    ///
    /// The second current channel is independent of the first, so this has no redundancy bit.
    pub const fn adi2(acq: Acquisition, diag: Diagnostic, ow: OpenWire) -> Command {
        // Variable bits: [7]=CONT, [4]=DIAG, [1]=OW[1], [0]=OW[0]
        let base: u16 = 0b00101101000;

        #[bitfield(u16)]
        struct Adi2 {
            #[bits(2)]
            pub b01_ow: u8,
            #[bits(2)]
            _b23: u8,
            #[bits(1)]
            pub b4_diag: u8,
            #[bits(2)]
            _b56: u8,
            #[bits(1)]
            pub b7_cont: u8,
            #[bits(8)]
            _reserved: u8,
        }

        let mut base = Adi2::from_bits(base);
        base.set_b01_ow(ow as u8);
        base.set_b4_diag(diag as u8);
        base.set_b7_cont(acq.cont());

        Command::define(true, base.into_bits())
    }

    /// Start V1ADC and V2ADC Conversions
    ///
    /// Response to this command is delayed by the `SOAK` time configured in CFGA.
    pub const fn adv(ow: OpenWireVoltage, vch: VoltageChannel) -> Command {
        // Variable bits: [7]=OW[1], [6]=OW[0], [3:0]=VCH[3:0]
        let base: u16 = 0b10000110000;

        #[bitfield(u16)]
        struct Adv {
            #[bits(4)]
            pub b0123_vch: u8,
            #[bits(2)]
            _b45: u8,
            #[bits(2)]
            pub b67_ow: u8,
            #[bits(8)]
            _reserved: u8,
        }

        let mut base = Adv::from_bits(base);
        base.set_b0123_vch(vch as u8);
        base.set_b67_ow(ow as u8);

        Command::define(true, base.into_bits())
    }

    /// Start AUX ADC Conversions
    ///
    /// This command takes no options. Table 33 on page 25 of the datasheet gives the code as
    /// `1, 0, 1, 0, 0, 1, 1, 0, x, x, x` -- the low three bits are don't-care, so there is no
    /// channel selection field to expose.
    pub const fn adx() -> Command {
        Command::define(true, 0b10100110000)
    }
}
