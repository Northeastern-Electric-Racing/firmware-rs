//! Register Layouts and Bit Descriptions for the COMM Register Group.
//!
//! The COMM register drives the chip's SPI/I2C controller port, letting the host tunnel bus
//! traffic to devices hanging off the ADBMS2950B's GPIOs. One COMM write carries up to three
//! bytes, each with a control action before it (`ICOMx`) and after it (`FCOMx`).
//!
//! The awkward part, and the reason this module has four register groups rather than one: the
//! `ICOMx` and `FCOMx` nibbles mean **completely different things** depending on both the bus
//! mode and the direction. The same code `0b1000` is "drive CSBM low" when writing in SPI mode
//! but "controller NACK on the 9th clock" when writing in I2C mode. And on a read they are not
//! control actions at all, they are *results* -- what the bus actually did. So the write view and
//! the read view are separate types, per bus mode.
//!
//! Only some of the sixteen codes are assigned; the datasheet marks the rest "reserved,
//! undetermined behavior". Those decode to `Unknown(raw)` rather than being coerced into a
//! neighbouring variant, so a surprising value survives to the logs intact.
//!
//! A COMM write only stages the transaction. Actually clocking it out takes a separate `STCOMM`
//! command followed by 72 clock cycles; see `StCommFrame`.
//!
//! For more info see Table 83 on page 66 of the datasheet (the register map) and Table 84 on
//! page 67 (the bit descriptions).

use bitfield_struct::bitfield;

use super::super::commands;
use super::register_group;

/// Field types relevant to the COMM register. See Table 84 on page 67 of the datasheet.
pub mod types {
    /// Declares a 4-bit COMM nibble code with an `Unknown` catch-all.
    ///
    /// This is a tt-muncher because it has to accept a variable-length list of documented
    /// variants followed by the fixed `Unknown(u8)` and `default =` tail. Munching one variant at
    /// a time avoids the local-ambiguity error you get from two adjacent `$(#[$m:meta])*`
    /// repetitions, which is what a single non-recursive rule would need.
    macro_rules! impl_commcode {
        (
            $(#[$enum_meta:meta])*
            $name:ident,
            $($body:tt)*
        ) => {
            impl_commcode!(@munch [$(#[$enum_meta])*] $name [] $($body)*);
        };
        (@munch
            [$(#[$enum_meta:meta])*] $name:ident
            [$({ $(#[$variant_meta:meta])* $variant:ident = $code:literal })*]
            $(#[$unknown_meta:meta])*
            Unknown(u8),
            default = $default:expr $(,)?
        ) => {
            $(#[$enum_meta])*
            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            pub enum $name {
                $( $(#[$variant_meta])* $variant, )*
                $(#[$unknown_meta])*
                Unknown(u8),
            }

            impl $name {
                /// The value this code takes when built via `Default` or a register group's `new()`.
                pub const DEFAULT: Self = $default;

                /// Reconstructs the code from its raw 4-bit field value.
                ///
                /// Any value the datasheet does not document for this field becomes `Unknown`
                /// carrying the raw bits, which is more useful than guessing.
                pub const fn from_bits(bits: u8) -> Self {
                    match bits & 0b1111 {
                        $( $code => Self::$variant, )*
                        other => Self::Unknown(other),
                    }
                }

                /// Serializes the code into its raw 4-bit field value.
                pub const fn into_bits(self) -> u8 {
                    match self {
                        $( Self::$variant => $code, )*
                        Self::Unknown(raw) => raw & 0b1111,
                    }
                }
            }

            impl ::core::default::Default for $name {
                fn default() -> Self { Self::DEFAULT }
            }
        };
        (@munch
            [$(#[$enum_meta:meta])*] $name:ident
            [$($collected:tt)*]
            $(#[$variant_meta:meta])*
            $variant:ident = $code:literal,
            $($rest:tt)*
        ) => {
            impl_commcode!(@munch
                [$(#[$enum_meta])*] $name
                [$($collected)* { $(#[$variant_meta])* $variant = $code }]
                $($rest)*
            );
        };
    }

    impl_commcode!(
        /// Control action taken *before* transmitting a byte, in I2C mode (`ICOMx[3:0]`).
        IcomI2cWriteCode,
        /// Hold SDA low between bytes (default).
        Blank = 0b0000,
        /// Generate an I2C STOP.
        Stop = 0b0001,
        /// Generate an I2C START.
        Start = 0b0110,
        /// Do not transmit; hold SDA high between bytes.
        NoTransmit = 0b0111,
        /// A code the datasheet marks reserved with undetermined behavior.
        Unknown(u8),
        default = Self::Blank,
    );

    impl_commcode!(
        /// Control action taken *after* transmitting a byte, in I2C mode (`FCOMx[3:0]`).
        FcomI2cWriteCode,
        /// Controller sends an ACK on the 9th clock (default).
        ControllerAck = 0b0000,
        /// Controller sends a NACK on the 9th clock.
        ControllerNack = 0b1000,
        /// Controller sends a NACK followed by a STOP.
        ControllerNackStop = 0b1001,
        /// A code the datasheet marks reserved with undetermined behavior.
        Unknown(u8),
        default = Self::ControllerAck,
    );

    impl_commcode!(
        /// Control action taken *before* transmitting a byte, in SPI mode (`ICOMx[3:0]`).
        IcomSpiWriteCode,
        /// Drive CSBM low.
        CsbmLow = 0b1000,
        /// Drive CSBM high; clock and data continue.
        CsbmHigh = 0b1001,
        /// Drive CSBM high, then low.
        CsbmHighThenLow = 0b1010,
        /// Do not transmit, release the outputs, and ignore the remaining data (default).
        NoTransmit = 0b1111,
        /// A code the datasheet marks reserved with undetermined behavior.
        Unknown(u8),
        default = Self::NoTransmit,
    );

    impl_commcode!(
        /// Control action taken *after* transmitting a byte, in SPI mode (`FCOMx[3:0]`).
        FcomSpiWriteCode,
        /// Hold CSBM low after the byte is transmitted (default).
        ///
        /// The datasheet lists both `0b0000` and `0b1000` as doing this; this is the `0b0000`
        /// encoding.
        HoldCsbmLow = 0b0000,
        /// Hold CSBM low after the byte is transmitted, via the `0b1000` encoding.
        HoldCsbmLowAlt = 0b1000,
        /// Transition CSBM high after the byte is transmitted.
        CsbmHighAfter = 0b1001,
        /// Read.
        Read = 0b1111,
        /// A code the datasheet marks reserved with undetermined behavior.
        Unknown(u8),
        default = Self::HoldCsbmLow,
    );

    impl_commcode!(
        /// Result of the pre-byte control action, read back in I2C mode (`ICOMx[3:0]`).
        IcomI2cReadCode,
        /// SDA was held low between bytes (default).
        SdaHeldLow = 0b0000,
        /// The controller generated a STOP.
        ControllerStop = 0b0001,
        /// The controller generated a START.
        ControllerStart = 0b0110,
        /// SDA was held high between bytes.
        SdaHeldHigh = 0b0111,
        /// A code the datasheet marks reserved with undetermined behavior.
        Unknown(u8),
        default = Self::SdaHeldLow,
    );

    impl_commcode!(
        /// Result of the post-byte control action, read back in I2C mode (`FCOMx[3:0]`).
        FcomI2cReadCode,
        /// The controller generated an ACK (default).
        ControllerAck = 0b0000,
        /// The peripheral generated an ACK and the controller a STOP.
        PeripheralAckControllerStop = 0b0001,
        /// The peripheral generated an ACK.
        PeripheralAck = 0b0111,
        /// The peripheral generated a NACK and the controller a STOP.
        PeripheralNackControllerStop = 0b1001,
        /// The peripheral generated a NACK.
        PeripheralNack = 0b1111,
        /// A code the datasheet marks reserved with undetermined behavior.
        Unknown(u8),
        default = Self::ControllerAck,
    );
}

/// Staged COMM transaction for I2C mode, written with `WRCOMM`.
///
/// See Table 83 on page 66 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = Some(commands::comm::wrcomm().frame()),
    read = None,
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct WriteCommI2c {
    /// Control action after byte 0 (`FCOM0[3:0]`).
    #[bits(4, default = types::FcomI2cWriteCode::DEFAULT)]  pub fcom0: types::FcomI2cWriteCode,
    /// Control action before byte 0 (`ICOM0[3:0]`).
    #[bits(4, default = types::IcomI2cWriteCode::DEFAULT)]  pub icom0: types::IcomI2cWriteCode,
    /// Transmit data byte 0 (`D0[7:0]`).
    #[bits(8, default = 0)]                                 pub d0: u8,
    /// Control action after byte 1 (`FCOM1[3:0]`).
    #[bits(4, default = types::FcomI2cWriteCode::DEFAULT)]  pub fcom1: types::FcomI2cWriteCode,
    /// Control action before byte 1 (`ICOM1[3:0]`).
    #[bits(4, default = types::IcomI2cWriteCode::DEFAULT)]  pub icom1: types::IcomI2cWriteCode,
    /// Transmit data byte 1 (`D1[7:0]`).
    #[bits(8, default = 0)]                                 pub d1: u8,
    /// Control action after byte 2 (`FCOM2[3:0]`).
    #[bits(4, default = types::FcomI2cWriteCode::DEFAULT)]  pub fcom2: types::FcomI2cWriteCode,
    /// Control action before byte 2 (`ICOM2[3:0]`).
    #[bits(4, default = types::IcomI2cWriteCode::DEFAULT)]  pub icom2: types::IcomI2cWriteCode,
    /// Transmit data byte 2 (`D2[7:0]`).
    #[bits(8, default = 0)]                                 pub d2: u8,
    #[bits(16, default = 0)]                                _padding: u16,
}

/// Staged COMM transaction for SPI mode, written with `WRCOMM`.
///
/// See Table 83 on page 66 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = Some(commands::comm::wrcomm().frame()),
    read = None,
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct WriteCommSpi {
    /// Control action after byte 0 (`FCOM0[3:0]`).
    #[bits(4, default = types::FcomSpiWriteCode::DEFAULT)]  pub fcom0: types::FcomSpiWriteCode,
    /// Control action before byte 0 (`ICOM0[3:0]`).
    #[bits(4, default = types::IcomSpiWriteCode::DEFAULT)]  pub icom0: types::IcomSpiWriteCode,
    /// Transmit data byte 0 (`D0[7:0]`).
    #[bits(8, default = 0)]                                 pub d0: u8,
    /// Control action after byte 1 (`FCOM1[3:0]`).
    #[bits(4, default = types::FcomSpiWriteCode::DEFAULT)]  pub fcom1: types::FcomSpiWriteCode,
    /// Control action before byte 1 (`ICOM1[3:0]`).
    #[bits(4, default = types::IcomSpiWriteCode::DEFAULT)]  pub icom1: types::IcomSpiWriteCode,
    /// Transmit data byte 1 (`D1[7:0]`).
    #[bits(8, default = 0)]                                 pub d1: u8,
    /// Control action after byte 2 (`FCOM2[3:0]`).
    #[bits(4, default = types::FcomSpiWriteCode::DEFAULT)]  pub fcom2: types::FcomSpiWriteCode,
    /// Control action before byte 2 (`ICOM2[3:0]`).
    #[bits(4, default = types::IcomSpiWriteCode::DEFAULT)]  pub icom2: types::IcomSpiWriteCode,
    /// Transmit data byte 2 (`D2[7:0]`).
    #[bits(8, default = 0)]                                 pub d2: u8,
    #[bits(16, default = 0)]                                _padding: u16,
}

/// COMM transaction results for I2C mode, read with `RDCOMM`.
///
/// Here the `ICOMx` and `FCOMx` nibbles report what the bus actually did rather than what to do.
/// See Table 84 on page 67 of the datasheet.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::comm::rdcomm().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct ReadCommI2c {
    /// Result of the post-byte action for byte 0 (`FCOM0[3:0]`).
    #[bits(4, default = types::FcomI2cReadCode::DEFAULT)]  pub fcom0: types::FcomI2cReadCode,
    /// Result of the pre-byte action for byte 0 (`ICOM0[3:0]`).
    #[bits(4, default = types::IcomI2cReadCode::DEFAULT)]  pub icom0: types::IcomI2cReadCode,
    /// Receive data byte 0 (`D0[7:0]`).
    #[bits(8, default = 0)]                                pub d0: u8,
    /// Result of the post-byte action for byte 1 (`FCOM1[3:0]`).
    #[bits(4, default = types::FcomI2cReadCode::DEFAULT)]  pub fcom1: types::FcomI2cReadCode,
    /// Result of the pre-byte action for byte 1 (`ICOM1[3:0]`).
    #[bits(4, default = types::IcomI2cReadCode::DEFAULT)]  pub icom1: types::IcomI2cReadCode,
    /// Receive data byte 1 (`D1[7:0]`).
    #[bits(8, default = 0)]                                pub d1: u8,
    /// Result of the post-byte action for byte 2 (`FCOM2[3:0]`).
    #[bits(4, default = types::FcomI2cReadCode::DEFAULT)]  pub fcom2: types::FcomI2cReadCode,
    /// Result of the pre-byte action for byte 2 (`ICOM2[3:0]`).
    #[bits(4, default = types::IcomI2cReadCode::DEFAULT)]  pub icom2: types::IcomI2cReadCode,
    /// Receive data byte 2 (`D2[7:0]`).
    #[bits(8, default = 0)]                                pub d2: u8,
    #[bits(16, default = 0)]                               _padding: u16,
}

/// COMM transaction results for SPI mode, read with `RDCOMM`.
///
/// In SPI mode the control nibbles carry no information on read: per Table 84 the `ICOMx` fields
/// always read back `0b0111` and the `FCOMx` fields always read back `0b1111`, whatever was
/// written. They are exposed as raw nibbles rather than decoded enums for exactly that reason --
/// only `d0`, `d1`, and `d2` are meaningful here.
#[rustfmt::skip]
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::comm::rdcomm().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct ReadCommSpi {
    /// Always reads back `0b1111` (`FCOM0[3:0]`).
    #[bits(4, default = 0b1111)]  pub fcom0: u8,
    /// Always reads back `0b0111` (`ICOM0[3:0]`).
    #[bits(4, default = 0b0111)]  pub icom0: u8,
    /// Receive data byte 0 (`D0[7:0]`).
    #[bits(8, default = 0)]       pub d0: u8,
    /// Always reads back `0b1111` (`FCOM1[3:0]`).
    #[bits(4, default = 0b1111)]  pub fcom1: u8,
    /// Always reads back `0b0111` (`ICOM1[3:0]`).
    #[bits(4, default = 0b0111)]  pub icom1: u8,
    /// Receive data byte 1 (`D1[7:0]`).
    #[bits(8, default = 0)]       pub d1: u8,
    /// Always reads back `0b1111` (`FCOM2[3:0]`).
    #[bits(4, default = 0b1111)]  pub fcom2: u8,
    /// Always reads back `0b0111` (`ICOM2[3:0]`).
    #[bits(4, default = 0b0111)]  pub icom2: u8,
    /// Receive data byte 2 (`D2[7:0]`).
    #[bits(8, default = 0)]       pub d2: u8,
    #[bits(16, default = 0)]      _padding: u16,
}

/// The full `STCOMM` wire frame: the command, its PEC, and the 72 clock cycles.
///
/// A `WRCOMM` only stages a transaction in the COMM register. `STCOMM` is what actually clocks it
/// out, and the controller has to keep the clock running for 72 cycles (9 bytes) afterwards while
/// the chip drives the bus. Those trailing bytes carry no data, so this type just emits zeros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StCommFrame {
    bytes: [u8; Self::BYTES],
}
impl StCommFrame {
    /// Total wire length: 2 command bytes, 2 PEC bytes, and 9 bytes of clocking.
    pub const BYTES: usize = 13;

    /// Number of trailing bytes needed to produce the 72 clock cycles.
    pub const CLOCK_BYTES: usize = 9;

    /// Builds the frame.
    pub const fn new() -> Self {
        let frame = commands::comm::stcomm().frame().to_bytes();
        let mut bytes = [0u8; Self::BYTES];
        bytes[0] = frame[0];
        bytes[1] = frame[1];
        bytes[2] = frame[2];
        bytes[3] = frame[3];
        Self { bytes }
    }

    /// The frame as bytes, ready to write to the bus in one transaction.
    pub const fn as_bytes(&self) -> &[u8; Self::BYTES] {
        &self.bytes
    }
}
impl Default for StCommFrame {
    fn default() -> Self {
        Self::new()
    }
}
