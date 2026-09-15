//! COMM Register!
//! 
//! See the "COMM REGISTER" section on page 42 of the datasheet, as well as the "I2C/SPI MASTER USING GPIOS" section directly above it.

use bitfield_struct::{bitfield, bitenum};
use adbms6830b_macros::BitfieldEnumDefault;

use super::register_group;
use super::super::commands;

/// Field types relavent to the Communication register. See Tables 32-35 on pages 42-43 of the datasheet.
/// 
/// Note: This doesn't use Table 116 (Communication Register Bit Descriptions) because it seems inconsistent with Tables 32 - 35. Weird!
pub mod types {
    use super::{bitenum, BitfieldEnumDefault};

    /// Implements `from_bits()`/`into_bits()` (plus `DEFAULT` and `Default`) for the `...ReadCode` enums,
    /// filling the role that `#[bitenum]` plays for the `...WriteCode` enums.
    ///
    /// The read codes can't use `#[bitenum]` because bitenum doesn't support enum variants that carry an inner, which we use for `Unknown(u8)`.
    /// We need to use `Unknown(u8)` instead of a plain `Unknown` so a malformed code can be logged for debugging.
    ///
    /// ### Syntax
    /// ```ignore
    /// impl_readcode!(
    ///     /// Doc comment for the enum itself.
    ///     MyReadCode,
    ///     /// Doc comment for a documented code.
    ///     SomeVariant = 0b0110,
    ///     /// Doc comment for the Unknown.
    ///     Unknown(u8),
    ///     default = Self::Unknown(0b1000)
    /// );
    /// ```
    ///
    /// ### Parameters
    /// - `$name`: the name of the enum to declare. It is always declared `pub` and derives
    ///   `Copy, Clone, Debug, PartialEq, Eq` (plus `defmt::Format` when the `defmt` feature is on).
    /// - `$variant = $code`: the documented codes as laid out in the datasheet Tables 33-35. Codes not specified here
    ///   will become `Unknown`.
    /// - `Unknown(u8),`: written out explicitly so the variant can carry its own doc comment. The literal
    ///   `Unknown(u8),` is what terminates the variant list.
    /// - `default = ...`: the value used by `DEFAULT`/`Default`. Should be code that isn't documented for that
    ///   field, so that the default resolves to an `Unknown` instead of decoding as a real code.
    ///
    /// ### Why this is a tt-muncher
    /// A rule can't contain two adjacent `$(#[$m:meta])*` repetitions because at a `#[doc]` token the matcher can't tell whether it belongs to the current
    /// variant or to the trailing `Unknown` variant, and rejects the macro with a local ambiguity error.
    /// Munching one variant at a time fixes that since separate rules are attempted in order.
    macro_rules! impl_readcode {
        (
            $(#[$enum_meta:meta])*
            $name:ident,
            $($body:tt)*
        ) => {
            impl_readcode!(@munch [$(#[$enum_meta])*] $name [] $($body)*);
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
                /// Any value not documented for this field becomes `Unknown` with the raw value as the inner (should be useful for debugging).
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

        // General rule: accumulate one documented `Variant = code,` and recurse on the rest.
        (@munch
            [$(#[$enum_meta:meta])*] $name:ident
            [$($acc:tt)*]
            $(#[$variant_meta:meta])*
            $variant:ident = $code:literal,
            $($rest:tt)*
        ) => {
            impl_readcode!(@munch
                [$(#[$enum_meta])*] $name
                [$($acc)* { $(#[$variant_meta])* $variant = $code }]
                $($rest)*
            );
        };
    }

    /// Write code for initial communication control bits (`ICOMx[3:0]`) on I2C Master. Four-bit field.
    /// 
    /// See Table 33 on page 42 of the datasheet.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum IcomI2cWriteCode {
        /// Action: Start.
        /// 
        /// Generates a start signal on the I2C port followed by data transmission.
        Start = 0b0110,
        /// Action: Stop.
        /// 
        /// Generates a stop signal on the I2C port.
        Stop = 0b0001,
        /// Action: Blank.
        /// 
        /// Proceeds directly to data transmission on I2C port.
        #[default]
        #[fallback]
        Blank = 0b0000,
        /// Action: No transmit.
        /// 
        /// Releases SDA and SCL and ignores the rest of the data.
        NoTransmit = 0b0111,
    }

    /// Write code for final communication control bits (`FCOMx[3:0]`) on I2C Master. Four-bit field.
    /// 
    /// See Table 33 on page 42 of the datasheet.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum FcomI2cWriteCode {
        /// Action: Master acknowledge.
        /// 
        /// Master generates an acknowledge signal on the ninth clock cycle.
        #[default]
        #[fallback]
        MasterAcknowledge = 0b0000,
        /// Action: Master no acknowledge.
        /// 
        /// Master generates a no acknowledge signal on the ninth clock cycle.
        MasterNoAcknowledge = 0b1000,
        /// Action: Master no acknowledge and stop.
        /// 
        /// Master generates a no acknowledge signal followed by a stop signal.
        MasterNoAcknowledgeAndStop = 0b1001,
    }

    /// Write code for initial communication control bits (`ICOMx[3:0]`) on SPI Master. Four-bit field.
    /// 
    /// See Table 34 on page 42 of the datasheet.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum IcomSpiWriteCode {
        /// Action: CSBM low.
        /// 
        /// Generates a CSBM low signal on SPI port (GPIO3).
        #[default]
        #[fallback]
        CsbmLow = 0b1000,
        /// Action: CSBM falling edge.
        /// 
        /// Drives CSBM (GPIO3) high, then low.
        CsbmFallingEdge = 0b1010,
        /// Action: CSBM high.
        /// 
        /// Generates a CSBM high signal on SPI port (GPIO3).
        CsbmHigh = 0b1001,
        /// Action: No transmit.
        /// 
        /// Releases the SPI port and ignores the rest of the data.
        NoTransmit = 0b1111,
    }

    /// Write code for final communication control bits (`FCOMx[3:0]`) on SPI Master. Four-bit field.
    /// 
    /// See Table 34 on page 42 of the datasheet.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum FcomSpiWriteCode {
        /// Action: CSBM low.
        /// 
        /// Holds CSBM low at the end of the byte transmission.
        /// 
        /// Note: This enum variant uses `0b0000` to represent this code, but based on the datasheet it is technically `0bx000` (i.e., the leftmost bit is a wildcard).
        /// This doesn't really matter since this is a write code, but why not leave this note here
        #[default]
        #[fallback]
        CsbmLow = 0b0000,
        /// Action: CSBM high.
        /// 
        /// Transitions CSBM high at the end of the byte transmission.
        CsbmHigh = 0b1001,
    }

    impl_readcode!(
        /// Read code for initial communication control bits (`ICOMx[3:0]`) on I2C Master. Four-bit field.
        ///
        /// See Table 35 on page 43 of the datasheet.
        IcomI2cReadCode,
        /// Master generated a start signal.
        Start = 0b0110,
        /// Master generated a stop signal.
        Stop = 0b0001,
        /// Blank, SDA held low between bytes.
        BlankSdaLow = 0b0000,
        /// Blank, SDA held high between bytes.
        BlankSdaHigh = 0b0111,
        /// Unknown ICOMx read code on I2C Master. The inner `u8` is whatever four-bit value was read in (hopefully useful for debugging).
        ///
        /// This probably indicates some kind of communication error happened, or that this enum somehow wasn't constructed correctly in software and is still set to its default value.
        Unknown(u8),
        default = Self::Unknown(0b1000)
    );

    impl_readcode!(
        /// Read code for final communication control bits (`FCOMx[3:0]`) on I2C Master. Four-bit field.
        ///
        /// See Table 35 on page 43 of the datasheet.
        FcomI2cReadCode,
        /// Master generated an acknowledge signal.
        MasterAck = 0b0000,
        /// Slave generated an acknowledge signal.
        SlaveAck = 0b0111,
        /// Slave generated a no acknowledge signal.
        SlaveNack = 0b1111,
        /// Slave generated an acknowledge signal, master generated a stop signal.
        SlaveAckMasterStop = 0b0001,
        /// Slave generated a no acknowledge signal, master generated a stop signal.
        SlaveNackMasterStop = 0b1001,
        /// Unknown FCOMx read code on I2C Master. The inner `u8` is whatever four-bit value was read in (hopefully useful for debugging).
        ///
        /// This probably indicates some kind of communication error happened, or that this enum somehow wasn't constructed correctly in software and is still set to its default value.
        Unknown(u8),
        default = Self::Unknown(0b1000)
    );

    impl_readcode!(
        /// Read code for initial communication control bits (`ICOMx[3:0]`) on SPI Master. Four-bit field.
        ///
        /// This should always be `0b0111` according to the top-right of page 43 of the datasheet.
        IcomSpiReadCode,
        /// Normal ICOM code for SPI reads.
        ///
        /// ICOMx[3:0] should only ever be this value (0b0111) for SPI reads according to the datasheet (near the top-right of page 43).
        Expected = 0b0111,
        /// Unknown ICOMx read code on SPI Master. The inner `u8` is whatever four-bit value was read in (hopefully useful for debugging).
        ///
        /// This probably indicates some kind of communication error happened, or that this enum somehow wasn't constructed correctly in software and is still set to its default value.
        Unknown(u8),
        default = Self::Unknown(0b0000)
    );

    impl_readcode!(
        /// Read code for final communication control bits (`FCOMx[3:0]`) on SPI Master. Four-bit field.
        ///
        /// This should always be `0b1111` according to the top-right of page 43 of the datasheet.
        FcomSpiReadCode,
        /// Normal FCOM code for SPI reads.
        ///
        /// FCOMx[3:0] should only ever be this value (0b1111) for SPI reads according to the datasheet (near the top-right of page 43).
        Expected = 0b1111,
        /// Unknown FCOMx read code on SPI Master. The inner `u8` is whatever four-bit value was read in (hopefully useful for debugging).
        ///
        /// This probably indicates some kind of communication error happened, or that this enum somehow wasn't constructed correctly in software and is still set to its default value.
        Unknown(u8),
        default = Self::Unknown(0b0000)
    );
}

/// Write COMM Register for I2C Master.
/// 
/// See Table 32 on page 42 of the datasheet, and Table 33 on page 42 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::comm::wrcomm().frame()),
    read = None,
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct WriteCommI2c {
    /// FCOM0[3:0]
    #[bits(4, default = types::FcomI2cWriteCode::DEFAULT)]  pub fcom0: types::FcomI2cWriteCode,
    /// ICOM0[3:0]
    #[bits(4, default = types::IcomI2cWriteCode::DEFAULT)]  pub icom0: types::IcomI2cWriteCode,
    /// Data Byte 0 (D0[7:0])
    #[bits(8, default = 0)]                                 pub data0: u8,

    /// FCOM1[3:0]
    #[bits(4, default = types::FcomI2cWriteCode::DEFAULT)]  pub fcom1: types::FcomI2cWriteCode,
    /// ICOM1[3:0]
    #[bits(4, default = types::IcomI2cWriteCode::DEFAULT)]  pub icom1: types::IcomI2cWriteCode,
    /// Data Byte 1 (D1[7:0])
    #[bits(8, default = 0)]                                 pub data1: u8,

    /// FCOM2[3:0]
    #[bits(4, default = types::FcomI2cWriteCode::DEFAULT)]  pub fcom2: types::FcomI2cWriteCode,
    /// ICOM2[3:0]
    #[bits(4, default = types::IcomI2cWriteCode::DEFAULT)]  pub icom2: types::IcomI2cWriteCode,
    /// Data Byte 2 D2[7:0]
    #[bits(8, default = 0)]                                 pub data2: u8,

    #[bits(16, default = 0)]                                _padding: u16,
}

/// Write COMM Register for SPI Master.
/// 
/// See Table 32 on page 42 of the datasheet, and Table 34 on page 42 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::comm::wrcomm().frame()),
    read = None,
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct WriteCommSpi {
    /// FCOM0[3:0]
    #[bits(4, default = types::FcomSpiWriteCode::DEFAULT)]  pub fcom0: types::FcomSpiWriteCode,
    /// ICOM0[3:0]
    #[bits(4, default = types::IcomSpiWriteCode::DEFAULT)]  pub icom0: types::IcomSpiWriteCode,
    /// Data Byte 0 (D0[7:0])
    #[bits(8, default = 0)]                                 pub data0: u8,

    /// FCOM1[3:0]
    #[bits(4, default = types::FcomSpiWriteCode::DEFAULT)]  pub fcom1: types::FcomSpiWriteCode,
    /// ICOM1[3:0]
    #[bits(4, default = types::IcomSpiWriteCode::DEFAULT)]  pub icom1: types::IcomSpiWriteCode,
    /// Data Byte 1 (D1[7:0])
    #[bits(8, default = 0)]                                 pub data1: u8,

    /// FCOM2[3:0]
    #[bits(4, default = types::FcomSpiWriteCode::DEFAULT)]  pub fcom2: types::FcomSpiWriteCode,
    /// ICOM2[3:0]
    #[bits(4, default = types::IcomSpiWriteCode::DEFAULT)]  pub icom2: types::IcomSpiWriteCode,
    /// Data Byte 2 D2[7:0]
    #[bits(8, default = 0)]                                 pub data2: u8,

    #[bits(16, default = 0)]                                _padding: u16,
}

/// Read COMM Register for I2C Master.
/// 
/// See Table 32 on page 42 of the datasheet, and Table 35 on page 43 of the datasheet.
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::comm::rdcomm().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct ReadCommI2c {
    /// FCOM0[3:0]
    #[bits(4, default = types::FcomI2cReadCode::DEFAULT)]  pub fcom0: types::FcomI2cReadCode,
    /// ICOM0[3:0]
    #[bits(4, default = types::IcomI2cReadCode::DEFAULT)]  pub icom0: types::IcomI2cReadCode,
    /// Data Byte 0 (D0[7:0])
    #[bits(8, default = 0)]                                pub data0: u8,

    /// FCOM1[3:0]
    #[bits(4, default = types::FcomI2cReadCode::DEFAULT)]  pub fcom1: types::FcomI2cReadCode,
    /// ICOM1[3:0]
    #[bits(4, default = types::IcomI2cReadCode::DEFAULT)]  pub icom1: types::IcomI2cReadCode,
    /// Data Byte 1 (D1[7:0])
    #[bits(8, default = 0)]                                pub data1: u8,

    /// FCOM2[3:0]
    #[bits(4, default = types::FcomI2cReadCode::DEFAULT)]  pub fcom2: types::FcomI2cReadCode,
    /// ICOM2[3:0]
    #[bits(4, default = types::IcomI2cReadCode::DEFAULT)]  pub icom2: types::IcomI2cReadCode,
    /// Data Byte 2 D2[7:0]
    #[bits(8, default = 0)]                                pub data2: u8,

    #[bits(16, default = 0)]                               _padding: u16,
}

/// Read COMM Register for SPI Master.
/// 
/// See Table 32 on page 42 of the datasheet. There's no table specifically for the SPI read ICOM/FCOM values
/// since the datasheet says that they are always ICOMx[3:0]=0b0111 and FCOMx[3:0]=0b1111 respectively for SPI
/// reads (see near the top right of page 43 of the datasheet).
#[register_group(
    bytes = 6,
    write = None,
    read = Some(commands::comm::rdcomm().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct ReadCommSpi {
    /// FCOM0[3:0]
    #[bits(4, default = types::FcomSpiReadCode::DEFAULT)]  pub fcom0: types::FcomSpiReadCode,
    /// ICOM0[3:0]
    #[bits(4, default = types::IcomSpiReadCode::DEFAULT)]  pub icom0: types::IcomSpiReadCode,
    /// Data Byte 0 (D0[7:0])
    #[bits(8, default = 0)]                                pub data0: u8,

    /// FCOM1[3:0]
    #[bits(4, default = types::FcomSpiReadCode::DEFAULT)]  pub fcom1: types::FcomSpiReadCode,
    /// ICOM1[3:0]
    #[bits(4, default = types::IcomSpiReadCode::DEFAULT)]  pub icom1: types::IcomSpiReadCode,
    /// Data Byte 1 (D1[7:0])
    #[bits(8, default = 0)]                                pub data1: u8,

    /// FCOM2[3:0]
    #[bits(4, default = types::FcomSpiReadCode::DEFAULT)]  pub fcom2: types::FcomSpiReadCode,
    /// ICOM2[3:0]
    #[bits(4, default = types::IcomSpiReadCode::DEFAULT)]  pub icom2: types::IcomSpiReadCode,
    /// Data Byte 2 D2[7:0]
    #[bits(8, default = 0)]                                pub data2: u8,

    #[bits(16, default = 0)]                               _padding: u16,
}

/// A complete protocol frame for the `STCOMM` command, which starts the I2C/SPI transaction that
/// `WRCOMM` staged in the COMM register.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StCommFrame {
    command: commands::CommandFrame,
}

impl StCommFrame {
    /// Number of data byte slots in the COMM register (`D0`, `D1`, `D2`).
    pub const DATA_SLOTS: usize = 3;
    /// Clock cycles the device needs to shift each data byte out of the GPIO port.
    /// See page 43 of the datasheet, and figure 25/26.
    pub const CLOCKS_PER_DATA_SLOT: usize = 24;
    /// Bytes of trailing clocking that must follow the command: `3 * 24 / 8` = 9.
    pub const TRAILING_BYTES: usize = Self::DATA_SLOTS * Self::CLOCKS_PER_DATA_SLOT / 8;
    /// Total length of the frame. It is four command bytes plus the trailing clocking.
    pub const BYTES: usize = 4 + Self::TRAILING_BYTES;

    /// Builds an `StCommFrame`.
    pub const fn new() -> Self {
        Self { command: commands::comm::stcomm().frame() }
    }

    /// Serializes the frame into bytes, for sending.
    ///
    /// Transmit all of these in a single transfer with CSB held low, and then raise CSB.
    pub const fn to_bytes(self) -> [u8; Self::BYTES] {
        let [cmd0, cmd1, pec0, pec1] = self.command.to_bytes();
        [cmd0, cmd1, pec0, pec1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    }

    /// The command portion of this frame.
    pub const fn command(&self) -> commands::CommandFrame { self.command }
}

impl Default for StCommFrame {
    fn default() -> Self { Self::new() }
}
