//! Mapping for the Command Codes seen in Table 50 of the datasheet (page 57).

#![allow(dead_code)]

use bitfield_struct::bitfield;

/// CC[10:0] - Command Code. 11-bit field.
/// 
/// See Table 50 on page 57 of the datasheet.
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

/// ADBMS6830B Command. See Table 50 on page 57 of the datasheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Command {
    /// Whether or not the command counter increments for the command. Corresponds to the INC value from Table 50 in the datasheet.
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

    /// Whether or not this command
    pub const fn increments(&self) -> bool {
        self.command.increments()
    }

    /// Whether this command resets the devices' command counters to 0.
    pub const fn resets_counter(&self) -> bool {
        self.command.resets_counter()
    }
}

impl Command {
    /// Allows you to define a command. Ideally should be called in a const context.
    /// ## Prammies
    /// - `inc`: Whether or not the command counter increments for the command. Corresponds to the INC value from Table 50 in the datasheet.
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
    /// RSTCC does it for obvious reasons. SRST does it by putting the devices to sleep. See the "COMMAND COUNTER" section on page 53 of the datasheet.
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

/// Cell voltage result commands.
pub mod cell_voltage {
    use super::Command;

    /// Read Cell Voltage Register Group A
    pub const fn rdcva() -> Command { Command::define(false, 0b00000000100) }
    /// Read Cell Voltage Register Group B
    pub const fn rdcvb() -> Command { Command::define(false, 0b00000000110) }
    /// Read Cell Voltage Register Group C
    pub const fn rdcvc() -> Command { Command::define(false, 0b00000001000) }
    /// Read Cell Voltage Register Group D
    pub const fn rdcvd() -> Command { Command::define(false, 0b00000001010) }
    /// Read Cell Voltage Register Group E
    pub const fn rdcve() -> Command { Command::define(false, 0b00000001001) }
    /// Read Cell Voltage Register Group F
    pub const fn rdcvf() -> Command { Command::define(false, 0b00000001011) }
    /// Read All Cell Results
    pub const fn rdcvall() -> Command { Command::define(false, 0b00000001100) }
}

/// Average cell voltage result commands.
pub mod avg_cell_voltage {
    use super::Command;

    /// Read Averaged Cell Voltage Register Group A
    pub const fn rdaca() -> Command { Command::define(false, 0b00001000100) }
    /// Read Averaged Cell Voltage Register Group B
    pub const fn rdacb() -> Command { Command::define(false, 0b00001000110) }
    /// Read Averaged Cell Voltage Register Group C
    pub const fn rdacc() -> Command { Command::define(false, 0b00001001000) }
    /// Read Averaged Cell Voltage Register Group D
    pub const fn rdacd() -> Command { Command::define(false, 0b00001001010) }
    /// Read Averaged Cell Voltage Register Group E
    pub const fn rdace() -> Command { Command::define(false, 0b00001001001) }
    /// Read Averaged Cell Voltage Register Group F
    pub const fn rdacf() -> Command { Command::define(false, 0b00001001011) }
    /// Read All Avg Cell Results
    pub const fn rdacall() -> Command { Command::define(false, 0b00001001100) }   
}

/// S voltage result commands.
pub mod s_voltage {
    use super::Command;

    /// Read S Voltage Register Group A
    pub const fn rdsva() -> Command { Command::define(false, 0b00000000011) }
    /// Read S Voltage Register Group B
    pub const fn rdsvb() -> Command { Command::define(false, 0b00000000101) }
    /// Read S Voltage Register Group C
    pub const fn rdsvc() -> Command { Command::define(false, 0b00000000111) }
    /// Read S Voltage Register Group D
    pub const fn rdsvd() -> Command { Command::define(false, 0b00000001101) }
    /// Read S Voltage Register Group E
    pub const fn rdsve() -> Command { Command::define(false, 0b00000001110) }
    /// Read S Voltage Register Group F
    pub const fn rdsvf() -> Command { Command::define(false, 0b00000001111) }
    /// Read All S Results
    pub const fn rdsall() -> Command { Command::define(false, 0b00000010000) }
}

/// Miscellaneous Commands.
pub mod misc {
    use super::Command;

    /// Read all AUX/Status Registers
    pub const fn rdasall() -> Command { Command::define(false, 0b00000110101) }
    /// Read all C and S Results
    pub const fn rdcsall() -> Command { Command::define(false, 0b00000010001) }
    /// Read all Average C and S Results
    pub const fn rdacsall() -> Command { Command::define(false, 0b00001010001) }
    /// Read Serial ID Register Group
    pub const fn rdsid() -> Command { Command::define(false, 0b00000101100) }
    /// Reset Command Counter
    pub const fn rstcc() -> Command { Command::define(false, 0b00000101110) }
    /// Soft Reset
    pub const fn srst() -> Command { Command::define(false, 0b00000100111) }
}

/// Filtered cell voltage result commands.
pub mod filtered_cell_voltage {
    use super::Command;

    /// Read Filter Cell Voltage Register Group A
    pub const fn rdfca() -> Command { Command::define(false, 0b00000010010) }
    /// Read Filter Cell Voltage Register Group B
    pub const fn rdfcb() -> Command { Command::define(false, 0b00000010011) }
    /// Read Filter Cell Voltage Register Group C
    pub const fn rdfcc() -> Command { Command::define(false, 0b00000010100) }
    /// Read Filter Cell Voltage Register Group D
    pub const fn rdfcd() -> Command { Command::define(false, 0b00000010101) }
    /// Read Filter Cell Voltage Register Group E
    pub const fn rdfce() -> Command { Command::define(false, 0b00000010110) }
    /// Read Filter Cell Voltage Register Group F
    pub const fn rdfcf() -> Command { Command::define(false, 0b00000010111) }
    /// Read All Filter Cell Results
    pub const fn rdfcall() -> Command { Command::define(false, 0b00000011000) }
}

/// Auxiliary result commands.
pub mod aux {
    use super::Command;

    /// Read Auxiliary Register Group A
    pub const fn rdauxa() -> Command { Command::define(false, 0b00000011001) }
    /// Read Auxiliary Register Group B
    pub const fn rdauxb() -> Command { Command::define(false, 0b00000011010) }
    /// Read Auxiliary Register Group C
    pub const fn rdauxc() -> Command { Command::define(false, 0b00000011011) }
    /// Read Auxiliary Register Group D
    pub const fn rdauxd() -> Command { Command::define(false, 0b00000011111) }
}

/// Redundant Auxiliary result commands.
pub mod redundant_aux {
    use super::Command;

    /// Read Redundant Auxiliary Register Group A
    pub const fn rdraxa() -> Command { Command::define(false, 0b00000011100) }
    /// Read Redundant Auxiliary Register Group B
    pub const fn rdraxb() -> Command { Command::define(false, 0b00000011101) }
    /// Read Auxiliary Redundant Register Group C
    pub const fn rdraxc() -> Command { Command::define(false, 0b00000011110) }
    /// Read Auxiliary Redundant Register Group D
    pub const fn rdraxd() -> Command { Command::define(false, 0b00000100101) }
}

/// Status register commands.
pub mod status {
    use super::Command;

    /// Read Status Register Group A
    pub const fn rdstata() -> Command { Command::define(false, 0b00000110000) }
    /// Read Status Register Group B
    pub const fn rdstatb() -> Command { Command::define(false, 0b00000110001) }
    /// Read Status Register Group C
    pub const fn rdstatc() -> Command { Command::define(false, 0b00000110010) }
    /// Read Status Register Group C, but with the ERR bit set.
    /// This is is basically a self-test that deliberately induces the fault condition.
    pub const fn rdstatcerr() -> Command { Command::define(false, 0b00001110010) }
    /// Read Status Register Group D
    pub const fn rdstatd() -> Command { Command::define(false, 0b00000110011) }
    /// Read Status Register Group E
    pub const fn rdstate() -> Command { Command::define(false, 0b00000110100) }
}

/// PWM registers commands.
pub mod pwm {
    use super::Command;

    /// Write PWM Register Group A
    pub const fn wrpwma() -> Command { Command::define(true, 0b00000100000) }
    /// Read PWM Register Group A
    pub const fn rdpwma() -> Command { Command::define(false, 0b00000100010) }
    /// Write PWM Register Group B
    pub const fn wrpwmb() -> Command { Command::define(true, 0b00000100001) }
    /// Read PWM Register Group B
    pub const fn rdpwmb() -> Command { Command::define(false, 0b00000100011) }
}

/// Clear commands.
pub mod clear {
    use super::Command;

    /// Clear Cell Voltage Register Groups
    pub const fn clrcell() -> Command { Command::define(true, 0b11100010001) }
    /// Clear Filtered Cell Voltage Register Groups
    pub const fn clrfc() -> Command { Command::define(true, 0b11100010100) }
    /// Clear Auxiliary Register Groups
    pub const fn clraux() -> Command { Command::define(true, 0b11100010010) }
    /// Clear S-Voltage Register Groups
    pub const fn clrspin() -> Command { Command::define(true, 0b11100010110) }
    /// Clear Flags
    pub const fn clrflag() -> Command { Command::define(true, 0b11100010111) }
    /// Clear OVUV
    pub const fn clovuv() -> Command { Command::define(true, 0b11100010101) }
}

/// Low Power Cell Monitoring (LPCM) commands.
pub mod lpcm {
    use super::Command;

    /// LPCM Disable
    pub const fn cmdis() -> Command { Command::define(true, 0b00001000000) }
    /// LPCM Enable
    pub const fn cmen() -> Command { Command::define(true, 0b00001000001) }
    /// LPCM Heartbeat
    pub const fn cmhb2() -> Command { Command::define(false, 0b00001000011) }
    /// Write LPCM Configuration Register
    pub const fn wrcmcfg() -> Command { Command::define(true, 0b00001011000) }
    /// Read LPCM Configuration Register
    pub const fn rdcmcfg() -> Command { Command::define(false, 0b00001011001) }
    /// Write LPCM Cell Threshold
    pub const fn wrcmcellt() -> Command { Command::define(true, 0b00001011010) }
    /// Read LPCM Cell Threshold
    pub const fn rdcmcellt() -> Command { Command::define(false, 0b00001011011) }
    /// Write LPCM GPIO Threshold
    pub const fn wrcmgpiot() -> Command { Command::define(true, 0b00001011100) }
    /// Read LPCM GPIO Threshold
    pub const fn rdcmgpiot() -> Command { Command::define(false, 0b00001011101) }
    /// Clear LPCM Flags
    pub const fn clrcmflag() -> Command { Command::define(true, 0b00001011110) }
    /// Read LPCM Flags
    pub const fn rdcmflag() -> Command { Command::define(false, 0b00001011111) }
}

/// ADC Commands.
pub mod adc {
    use super::Command;
    use super::bitfield;

    /// Poll Any ADC Status
    pub const fn pladc() -> Command { Command::define(true, 0b11100011000) }
    /// Poll C-ADC
    pub const fn plcadc() -> Command { Command::define(true, 0b11100011100) }
    /// Poll S-ADC
    pub const fn plsadc() -> Command { Command::define(true, 0b11100011101) }
    /// Poll AUX ADC
    pub const fn plaux() -> Command { Command::define(true, 0b11100011110) }
    /// Poll AUX2 ADC
    pub const fn plaux2() -> Command { Command::define(true, 0b11100011111) }

    /// Command Bit Description for `Selection for AUX Inputs ADAX` (CH[4:0]) function.
    /// See Table 52 on page 59 of the datasheet.
    /// This is a 5-bit field.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Aux1InputSelection {
        /// AUX Input = ALL
        All = 0b00000,
        /// AUX Input = GPIO1
        Gpio1 = 0b00001,
        /// AUX Input = GPIO2
        Gpio2 = 0b00010,
        /// AUX Input = GPIO3
        Gpio3 = 0b00011,
        /// AUX Input = GPIO4
        Gpio4 = 0b00100,
        /// AUX Input = GPIO5
        Gpio5 = 0b00101,
        /// AUX Input = GPIO6
        Gpio6 = 0b00110,
        /// AUX Input = GPIO7
        Gpio7 = 0b00111,
        /// AUX Input = GPIO8
        Gpio8 = 0b01000,
        /// AUX Input = GPIO9
        Gpio9 = 0b01001,
        /// AUX Input = GPIO10
        Gpio10 = 0b01010,
        /// AUX Input = VREF2
        Vref2 = 0b10000,
        /// AUX Input = VD
        Vd = 0b10001,
        /// AUX Input = VA
        Va = 0b10010,
        /// AUX Input = ITEMP
        Itemp = 0b10011,
        /// AUX Input = VPV
        Vpv = 0b10100,
        /// AUX Input = VMV
        Vmv = 0b10101,
        /// AUX Input = RES
        Res = 0b10110,
    }

    /// Command Bit Description for `Selection for AUX Inputs ADAX2` (CH[3:0]) function.
    /// See Table 52 on page 59 of the datasheet.
    /// This is a 4-bit field.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Aux2InputSelection {
        /// AUX Input = ALL
        All = 0b00000,
        /// AUX Input = GPIO1
        Gpio1 = 0b00001,
        /// AUX Input = GPIO2
        Gpio2 = 0b00010,
        /// AUX Input = GPIO3
        Gpio3 = 0b00011,
        /// AUX Input = GPIO4
        Gpio4 = 0b00100,
        /// AUX Input = GPIO5
        Gpio5 = 0b00101,
        /// AUX Input = GPIO6
        Gpio6 = 0b00110,
        /// AUX Input = GPIO7
        Gpio7 = 0b00111,
        /// AUX Input = GPIO8
        Gpio8 = 0b01000,
        /// AUX Input = GPIO9
        Gpio9 = 0b01001,
        /// AUX Input = GPIO10
        Gpio10 = 0b01010,
    }

    /// How a conversion runs, and whether PWM discharge continues through it.
    ///
    /// See Table 19 on page 20 and Table 52 on page 59 of the datasheet.
    /// This enum models the possible combinations of DCP and CONT. The invalid
    /// combaintions of those two bits are left out of this enum on purpose.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Acquisition {
        /// Make a single measurement and then standby.
        ///
        /// PWM discharge is interrupted for the duration of any S-ADC measurement this
        /// triggers. This is useful if you want to make sure the result isn't skewed by the discharge current's voltage drop
        /// across the cell cabling.
        /// 
        /// This variant corresponds to `CONT = 0`, `DCP = 0`.
        SingleShot,
        /// Make a single measurement and then standby.
        ///
        /// PWM discharge continues through the measurement. This will keep balancing running,
        /// but the discharge current's drop across the cell cabling may skew the result by an
        /// amount that isn't predictable, so the intended voltage thresholds may not be
        /// checked accurately.
        /// 
        /// This variant corresponds to  `CONT = 0`, `DCP = 1`.
        SingleShotDischarging,
        /// Measure continuously until stopped.
        ///
        /// The result registers will update at the ADC's conversion rate (1 ms for the C-ADCs,
        /// 8 ms for the S-ADCs). To stop, you can send the same command again with
        /// `Acquisition::SingleShot`. The ADC will take one last measurement and then turn off.
        ///
        /// PWM discharge stops and stays off unless a further command re-enables it.
        /// 
        /// This variant corresponds to `CONT = 1`, `DCP = 0`.
        Continuous,
    }

    /// How a conversion runs, and whether PWM discharge continues through it, for the `_autoconvert()` helpers.
    /// 
    /// This enum is just [`Acquisition`], but without the `Continuous` option. This is because continuous conversions
    /// have no ADC completion to poll on, so the `_autoconvert()` helpers are not allowed to accept `Continuous` as
    /// an option in their parameters.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum AutoAcquisition {
        /// Make a single measurement and then standby.
        ///
        /// PWM discharge is interrupted for the duration of any S-ADC measurement this
        /// triggers. This is useful if you want to make sure the result isn't skewed by the discharge current's voltage drop
        /// across the cell cabling.
        /// 
        /// This variant corresponds to `CONT = 0`, `DCP = 0`.
        SingleShot,
        /// Make a single measurement and then standby.
        ///
        /// PWM discharge continues through the measurement. This will keep balancing running,
        /// but the discharge current's drop across the cell cabling may skew the result by an
        /// amount that isn't predictable, so the intended voltage thresholds may not be
        /// checked accurately.
        /// 
        /// This variant corresponds to  `CONT = 0`, `DCP = 1`.
        SingleShotDischarging,
    }
    impl From<AutoAcquisition> for Acquisition {
        fn from(item: AutoAcquisition) -> Self {
            match item {
                AutoAcquisition::SingleShot => Acquisition::SingleShot,
                AutoAcquisition::SingleShotDischarging => Acquisition::SingleShotDischarging,
            }
        }
    }

    impl Acquisition {
        /// The `DCP` bit for this acquisition mode.
        const fn dcp(self) -> u8 {
            match self {
                Self::SingleShotDischarging => 1,
                _ => 0,
            }
        }

        /// The `CONT` bit for this acquisition mode.
        const fn cont(self) -> u8 {
            match self {
                Self::Continuous => 1,
                _ => 0,
            }
        }
    }
    
    /// Command Bit Description for `Open wire on C-ADCS and S-ADCs` (OW[1:0]) function. 
    /// See Table 52 on page 59 of the datasheet.
    /// This is a 2-bit field.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OpenWire {
        /// Open wire detection off on all channels.
        OffForAll = 0b00,
        /// Open wire detection on for even channels, off for odd channels.
        EvenOnOddOff = 0b01,
        /// Open wire detection on for odd channels, off for even channels.
        EvenOffOddOn = 0b10,
        /// Open wire detection on for all channels.
        OnForAll = 0b11,
    }

    /// Command Bit Description for `Open wire on AUX ADCs` (OW) function. 
    /// See Table 52 on page 59 of the datasheet.
    /// This is a 1-bit field.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OpenWireAux {
        /// Off
        Off = 0,
        /// On
        On = 1,
    }

    /// Command Bit Description for `Pull-up and pull down current for open wire conversions` (PUP) function. 
    /// See Table 52 on page 59 of the datasheet.
    /// This is a 1-bit field.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Pull {
        /// Pull-down current during AUX conversions (if OW = 1)
        PullDown = 0,
        /// Pull-up current during AUX conversions (if OW = 1)
        PullUp = 1,
    }

    /// Command Bit Description for `Reset filter` (RSTF) function.
    /// See Table 52 on page 59 of the datasheet.
    /// This is a 1-bit field.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum ResetFilter {
        /// Do not reset IIR filter.
        NoReset = 0,
        /// Reset IIR filter.
        Reset = 1,
    }

    /// Redundancy (RD) for the ADCV command. See Table 19 on page 20 of the datasheet.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum AdcvRedundancy {
        /// C-ADCs only (no redundant comparison).
        Disabled = 0,
        /// Trigger C-ADCs and S-ADCs and compare results (redundant measurement).
        Enabled = 1,
    }

    /// Start Cell Voltage ADC Conversion and Poll Status
    ///
    /// Note that *any* ADCV interrupts the ongoing C-ADC conversions and restarts the
    /// C-ADCs. Because of this, the datasheet recommends triggering periodic redundant
    /// measurements with `adsv()` (`Acquisition::Continuous`) rather than re-issuing this
    /// command. Also note that an ADCV with `AdcvRedundancy::Enabled` resets the open wire
    /// switches to open so that the C-ADC/S-ADC comparison is valid, so redundancy and open
    /// wire excitation can't be combined in a single ADCV.
    ///
    /// See Table 19 on page 20 of the datasheet for what each combination of `rd` and `acq`
    /// does to the ongoing conversions and to PWM discharge.
    pub const fn adcv(rd: AdcvRedundancy, acq: Acquisition, rstf: ResetFilter, ow: OpenWire) -> Command {
        // Variable bits: [8]=RD, [7]=CONT, [4]=DCP, [2]=RSTF, [1]=OW[1], [0]=OW[0]
        let base: u16 = 0b01001100000;
        
        #[bitfield(u16)]
        struct Adcv {
            #[bits(2)]
            pub b01_ow: u8,
            #[bits(1)]
            pub b2_rstf: u8,
            #[bits(1)]
            _b3: u8,
            #[bits(1)]
            pub b4_dcp: u8,
            #[bits(2)]
            _b56: u8,
            #[bits(1)]
            pub b7_cont: u8,
            #[bits(1)]
            pub b8_rd: u8,
            #[bits(7)]
            _reserved: u8,
        }

        let mut base = Adcv::from_bits(base);
        base.set_b01_ow(ow as u8);
        base.set_b2_rstf(rstf as u8);
        base.set_b4_dcp(acq.dcp());
        base.set_b7_cont(acq.cont());
        base.set_b8_rd(rd as u8);

        Command::define(true, base.into_bits())
    }

    /// Start S-ADC Conversion and Poll Status
    ///
    /// Unlike `adcv()`, this doesn't restart the C-ADCs, so it's the datasheet's
    /// recommended way to take periodic redundant measurements. Issuing this with
    /// `Acquisition::Continuous` while the C-ADCs are already converting continuously
    /// synchronizes the S-ADCs to the C-ADC average and compares the two.
    ///
    /// See Table 19 on page 20 of the datasheet for what each `acq` value does to the
    /// ongoing conversions and to PWM discharge.
    pub const fn adsv(acq: Acquisition, ow: OpenWire) -> Command {
        // Variable bits: [7]=CONT, [4]=DCP, [1]=OW[1], [0]=OW[0]
        let base: u16 = 0b00101101000;

        #[bitfield(u16)]
        struct Adsv {
            #[bits(2)]
            pub b01_ow: u8,
            #[bits(2)]
            _b23: u8,
            #[bits(1)]
            pub b4_dcp: u8,
            #[bits(2)]
            _b56: u8,
            #[bits(1)]
            pub b7_cont: u8,
            #[bits(8)]
            _reserved: u8,
        }

        let mut base = Adsv::from_bits(base);
        base.set_b01_ow(ow as u8);
        base.set_b4_dcp(acq.dcp());
        base.set_b7_cont(acq.cont());

        Command::define(true, base.into_bits())
    }

    /// Start AUX ADC Conversions and Poll Status
    pub const fn adax(ow: OpenWireAux, pup: Pull, ch: Aux1InputSelection) -> Command { 
        // Variable bits: [8]=OW, [7]=PUP, [6]=CH[4], [3]=CH[3], [2]=CH[2], [1]=CH[1], [0]=CH[0]
        let base: u16 = 0b10000010000;

        #[bitfield(u8)]
        struct Ch {
            #[bits(4)]
            pub ch0123: u8,
            #[bits(1)]
            pub ch4: u8,
            #[bits(3)]
            _reserved: u8,
        }
        let ch_bits = Ch::from_bits(ch as u8);

        #[bitfield(u16)]
        struct Adax {
            #[bits(4)]
            pub b0123_ch0123: u8,
            #[bits(2)]
            _b45: u8,
            #[bits(1)]
            pub b6_ch4: u8,
            #[bits(1)]
            pub b7_pup: u8,
            #[bits(1)]
            pub b8_ow: u8,
            #[bits(7)]
            _reserved: u8,
        }

        let mut base = Adax::from_bits(base);
        base.set_b0123_ch0123(ch_bits.ch0123());
        base.set_b6_ch4(ch_bits.ch4());
        base.set_b7_pup(pup as u8);
        base.set_b8_ow(ow as u8);

        Command::define(true, base.into_bits())
    }

    /// Start AUX2 ADC Conversions and Poll Status
    pub const fn adax2(ch: Aux2InputSelection) -> Command {
        // Variable bits: [3]=CH[3], [2]=CH[2], [1]=CH[1], [0]=CH[0].
        let base: u16 = 0b10000000000;

        #[bitfield(u16)]
        struct Adax2 {
            #[bits(4)]
            pub b0123_ch: u8,
            #[bits(12)]
            _reserved: u16,
        }

        let mut base = Adax2::from_bits(base);
        base.set_b0123_ch(ch as u8);

        Command::define(true, base.into_bits())
    }
}

/// GPIO Comm commands.
pub mod comm {
    use super::Command;

    /// Write COMM Register Group
    pub const fn wrcomm() -> Command { Command::define(true, 0b11100100001) }
    /// Read COMM Register Group
    pub const fn rdcomm() -> Command { Command::define(false, 0b11100100010) }
    /// Start I2C/SPI Communication
    pub const fn stcomm() -> Command { Command::define(true, 0b11100100011) }
}

/// Retention register commands.
pub mod retention {
    use super::Command;

    /// Unlock Retention Register
    pub const fn ulrr() -> Command { Command::define(true, 0b00000111000) }
    /// Write Retention Registers
    pub const fn wrrr() -> Command { Command::define(true, 0b00000111001) }
    /// Read Retention Registers
    pub const fn rdrr() -> Command { Command::define(false, 0b00000111010) }
}

/// Snapshot commands.
pub mod snapshot {
    use super::Command;

    /// Snapshot
    pub const fn snap() -> Command { Command::define(true, 0b00000101101) }
    /// Release Snapshot
    pub const fn unsnap() -> Command { Command::define(true, 0b00000101111) }
}

/// Discharge commands.
pub mod discharge {
    use super::Command;

    /// Mute Discharge
    pub const fn mute() -> Command { Command::define(true, 0b00000101000) }
    /// Unmute Discharge
    pub const fn unmute() -> Command { Command::define(true, 0b00000101001) }
}