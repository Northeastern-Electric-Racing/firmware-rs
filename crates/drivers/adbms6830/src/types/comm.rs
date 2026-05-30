//! Communication register (`WRCOMM` / `RDCOMM`) and commands.

use bitfield_struct::bitfield;

use crate::registers::{AdbmsCommand, AdbmsReadableRegister, AdbmsWritableRegister};

/// The STCOMM command initiates I2C and SPI communication on the GPIO ports.
/// The COMM register contains three bytes of data to be transmitted to the
/// slave. During this command, the data bytes stored in the COMM register are
/// transmitted to the slave I2C or SPI device, and the data received from the
/// I2C or SPI device is stored in the COMM register. This command uses
/// GPIO4 (SDA) and GPIO5 (SCL) for I2C communication or GPIO3 (CSBM),
/// GPIO4 (SDIOM), and GPIO5 (SCKM) for SPI communication.
///
/// Note: As of now there is no function to add 24 clock cycles per data byte
pub struct Stcomm {}

impl AdbmsCommand for Stcomm {
    fn get_command(&self) -> u16 {
        0b11100100011
    }
}

/// COMM Register Group.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct Comm {
    /// Final control nibble of byte 0. See `codes` when writing.
    #[bits(4)]
    pub fcom0: u8,
    /// Initial control nibble of byte 0. See `codes` when writing.
    #[bits(4)]
    pub icom0: u8,
    /// Data byte 0.
    pub d0: u8,
    #[bits(4)]
    pub fcom1: u8,
    #[bits(4)]
    pub icom1: u8,
    pub d1: u8,
    #[bits(4)]
    pub fcom2: u8,
    #[bits(4)]
    pub icom2: u8,
    pub d2: u8,
    #[bits(16)]
    __: u16,
}

impl AdbmsWritableRegister for Comm {
    const WRITE_CMD: [u8; 2] = [0x07, 0x21];
}
impl AdbmsReadableRegister for Comm {
    const READ_CMD: [u8; 2] = [0x07, 0x22];
}

// Named 4-bit codes from datasheet Tables 33–34. The same code means
// different things depending on master mode, so they are grouped here
// rather than baked into a single enum.
/// Named 4-bit `ICOM`/`FCOM` codes from datasheet Tables 33–34. The same
/// code value means different things depending on whether the chip is
/// acting as an I²C or SPI master.  Use with `fcomX` and `icomX` when writing.
pub mod codes {
    // I²C master send ICOM codes (Table 33)
    pub const I2C_ICOM_START: u8 = 0b0110;
    pub const I2C_ICOM_STOP: u8 = 0b0001;
    pub const I2C_ICOM_BLANK: u8 = 0b0000;
    pub const I2C_ICOM_NO_TX: u8 = 0b0111;
    // I²C master FCOM codes (Table 33)
    pub const I2C_FCOM_ACK: u8 = 0b0000;
    pub const I2C_FCOM_NACK: u8 = 0b1000;
    pub const I2C_FCOM_NACK_STOP: u8 = 0b1001;
    // SPI master ICOM codes (Table 34)
    pub const SPI_ICOM_CSBM_LOW: u8 = 0b1000;
    pub const SPI_ICOM_CSBM_FALL: u8 = 0b1010;
    pub const SPI_ICOM_CSBM_HIGH: u8 = 0b1001;
    pub const SPI_ICOM_NO_TX: u8 = 0b1111;
    // SPI master FCOM codes (Table 34)
    pub const SPI_FCOM_CSBM_LOW: u8 = 0b0000;
    pub const SPI_FCOM_CSBM_HIGH: u8 = 0b1001;
}
