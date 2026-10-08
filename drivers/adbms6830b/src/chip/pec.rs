//! Packet Error Code (PEC) calculations for the ADBMS6830B.

#![allow(dead_code)]

/// A calculated command PEC (CRC-15). See Table 41 of the datasheet.
/// 
/// This is something we send! We never recieve this.
///
/// Wraps the doubled 16-bit remainder (the 15-bit CRC in bits `[15:1]` with a
/// `0` in the LSB). The two bytes that get sent on the wire are simply its high and low bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CommandPec{ raw: u16 }

impl CommandPec {
    /// Creates a new `CommandPec`.
    pub const fn new(data: &[u8]) -> Self {
        Self { raw: pec15_calc(data) }
    }

    /// Command Packet Error Code Byte 0 (`PEC[14:7]`). See Table 41 of the datasheet.
    pub const fn pec0(&self) -> u8 { (self.raw >> 8) as u8 }

    /// Command Packet Error Code Byte 1 (`PEC[7:0]`). Bit `0` will always be a trailing zero. See Table 41 of the datasheet.
    pub const fn pec1(&self) -> u8 { self.raw as u8 }

    /// The PEC's raw doubled 16-bit value.
    pub const fn bits(&self) -> u16 { self.raw }
}

/// A calculated data PEC (CRC-10). See Table 42 (write) / Table 43 (read).
///
/// Wraps the raw 10-bit CRC value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct DataPec { raw: u16 }

impl DataPec {
    /// Creates a new `DataPec`.
    pub const fn new(value: u16) -> Self {
        Self { raw: value }
    }

    /// Data Packet Error Code Byte 0 (`PEC[9:8]`).
    /// 
    /// See Table 42 of the datasheet. `PEC[9:8]` is stored in the first two bits of PEC0. The remaining 6 bits are zeroed on TX,
    /// or carry `CCNT[5:0]` on RX.
    pub const fn pec0(&self) -> u8 { (self.raw >> 8) as u8 }

    /// Data Packet Error Code Byte 1 (`PEC[7:0]`).
    /// 
    /// See Table 42 of the datasheet.
    pub const fn pec1(&self) -> u8 { self.raw as u8 }

    /// The raw 10-bit CRC value backing this PEC.
    pub const fn bits(&self) -> u16 { self.raw }
}

/// A data PEC for TX. This is something we calculate before transmitting. See Table 42 on page 53 of the datasheet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DataPecTx { inner: DataPec }
impl DataPecTx {
    /// Creates a new `DataPecTx`.
    /// 
    /// ### Prams
    /// - `data`: The data payload you are about to transmit (i.e., a `ConfigA` instance). This PEC is calculated from that data.
    pub const fn new(data: &[u8]) -> Self {
        let calculated: u16 = pec10_calc(data, None);
        Self { inner: DataPec::new(calculated) }
    }

    /// Data Packet Error Code Byte 0 (`PEC[9:8]`).
    /// 
    /// See Table 42 of the datasheet. `PEC[9:8]` is stored in the first two bits of PEC0.
    pub const fn pec0(&self) -> u8 { self.inner.pec0() & 0b00000011 } // get first 2 bits

    /// Data Packet Error Code Byte 1 (`PEC[7:0]`).
    /// 
    /// See Table 42 of the datasheet.
    pub const fn pec1(&self) -> u8 { self.inner.pec1() }

    /// The raw 10-bit CRC value backing this PEC.
    pub const fn bits(&self) -> u16 { self.inner.bits() }
}

/// A data PEC for RX. This is something we construct when we recieve a frame. See Table 43 on page 53 of the datasheet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DataPecRx { inner: DataPec }
impl DataPecRx {
    /// Creates a `DataPecRx` from the two received PEC bytes (in wire order).
    /// 
    /// ### Pram
    /// - `bytes[0]` = PEC0 (received first. carries `CCNT[5:0]` and `PEC[9:8]`)
    /// - `bytes[1]` = PEC1 (`PEC[7:0]`)
    ///
    /// See Table 43 of the datasheet.
    pub const fn from_bytes(bytes: [u8; 2]) -> Self {
        let raw = ((bytes[0] as u16) << 8) | bytes[1] as u16;
        Self { inner: DataPec::new(raw) }
    }
    
    /// Checks whether or not this PEC matches what was expected based on the data.
    /// If this is a valid PEC, this function will return `true`.
    /// If this was an invalid PEC, this function will return `false`, indicating some
    /// kind of communication error occured during reception.
    /// 
    /// ### Prams
    /// - `data`: The data from the recieved response.
    pub const fn verify(&self, data: &[u8]) -> bool {
        let expected: u16 = pec10_calc(data, Some(self.ccnt()));
        self.pec0() == (expected >> 8) as u8 & 0b11 && self.pec1() == expected as u8
    }

    /// Data Packet Error Code Byte 0 (`PEC[9:8]`).
    /// 
    /// See Table 43 of the datasheet. `PEC[9:8]` is stored in the first two bits of PEC0.
    /// This does not contain the `CCNT[5:0]` bits (see `ccnt()` for that).
    pub const fn pec0(&self) -> u8 { self.inner.pec0() & 0b00000011 } // get first 2 bits

    /// Returns the CCNT[5:0] data included in a `DataPecRx`. See Table 43 of the datasheet.
    pub const fn ccnt(&self) -> u8 { self.inner.pec0() >> 2 } // get last 6 bits

    /// Data Packet Error Code Byte 1 (`PEC[7:0]`).
    /// 
    /// See Table 43 of the datasheet.
    pub const fn pec1(&self) -> u8 { self.inner.pec1() }

    /// The raw 10-bit CRC value backing this PEC.
    pub const fn bits(&self) -> u16 { self.inner.bits() }
}

/// CRC-15 command PEC.
const fn pec15_calc(data: &[u8]) -> u16 {
    let mut remainder: u16 = 16; // PEC seed
    let mut i = 0;
    while i < data.len() {
        // calculate PEC table address
        let addr = (((remainder >> 7) ^ data[i] as u16) & 0xff) as usize;
        remainder = (remainder << 8) ^ CRC15_TABLE[addr];
        i += 1;
    }
    // The CRC15 has a 0 in the LSB, so the remainder is multiplied by 2.
    remainder << 1
}

/// CRC-10 data PEC (via bits). Produces identical results to `pec10_calc_table()`.
/// 
/// ### Prams
/// - `data`: The data you want to use for the calculation.
/// - `rx_ccnt`: If this is for RX, pass in the recieved CCNT[5:0] value from your frame. If this is for TX, pass in `None`.
const fn pec10_calc(data: &[u8], rx_ccnt: Option<u8>) -> u16 {
    // x10 + x7 + x3 + x2 + x + 1 <- the CRC10 polynomial (100 1000 1111)
    const POLYNOMIAL: u16 = 0x8F;
    let n = data.len();
    let mut remainder: u16 = 16; // PEC seed

    let mut byte_index = 0;
    while byte_index < n {
        // Bring the next byte into the remainder.
        remainder ^= (data[byte_index] as u16) << 2;
        // Perform modulo-2 division, a bit at a time.
        let mut bit_index = 8;
        while bit_index > 0 {
            if remainder & 0x200 != 0 {
                remainder = (remainder << 1) ^ POLYNOMIAL;
            } else {
                remainder <<= 1;
            }
            bit_index -= 1;
        }
        byte_index += 1;
    }

    // If the array is from a received buffer, add the command counter to the CRC.
    if let Some(command_counter) = rx_ccnt {
        remainder ^= ((command_counter as u16) & 0x3F) << 4;
    }

    // Perform modulo-2 division, a bit at a time.
    let mut bit_index = 6;
    while bit_index > 0 {
        if remainder & 0x200 != 0 {
            remainder = (remainder << 1) ^ POLYNOMIAL;
        } else {
            remainder <<= 1;
        }
        bit_index -= 1;
    }

    remainder & 0x3FF
}

/// CRC-10 data PEC (table-based). Produces identical results to `pec10_calc()`.
///
/// ### Prams
/// - `data`: The data you want to use for the calculation.
/// - `rx_ccnt`: If this is for RX, pass in the recieved CCNT[5:0] value from your frame. If this is for TX, pass in `None`.
const fn pec10_calc_table(data: &[u8], rx_ccnt: Option<u8>) -> u16 {
    // x10 + x7 + x3 + x2 + x + 1 <- the CRC10 polynomial (100 1000 1111)
    const POLYNOMIAL: u16 = 0x8F;
    let n = data.len();
    let mut remainder: u16 = 16; // PEC seed

    let mut byte_index = 0;
    while byte_index < n {
        // calculate PEC table address
        let addr = (((remainder >> 2) ^ data[byte_index] as u16) & 0xff) as usize;
        remainder = (remainder << 8) ^ CRC10_TABLE[addr];
        byte_index += 1;
    }

    // If the array is from a received buffer, add the command counter to the CRC.
    if let Some(command_counter) = rx_ccnt {
        remainder ^= ((command_counter as u16) & 0x3F) << 4;
    }

    // Perform modulo-2 division, a bit at a time.
    let mut bit_index = 6;
    while bit_index > 0 {
        if remainder & 0x200 != 0 {
            remainder = (remainder << 1) ^ POLYNOMIAL;
        } else {
            remainder <<= 1;
        }
        bit_index -= 1;
    }

    remainder & 0x3FF
}

// Big Compile-time self-checks:

// Seed-only behaviour (empty input) pins the seed and final divisions.
const _: () = assert!(pec15_calc(&[]) == 0x0020);
const _: () = assert!(pec10_calc(&[], None) == 0x008F);

// The bitwise and table-based CRC-10 must always agree (TX and RX paths).
const _: () = assert!(
    pec10_calc(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05], None)
        == pec10_calc_table(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05], None)
);
const _: () = assert!(
    pec10_calc(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66], Some(1))
        == pec10_calc_table(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66], Some(1))
);


// big lookup tables

const CRC15_TABLE: [u16; 256] = [
    0x0000, 0xc599, 0xceab, 0x0b32, 0xd8cf, 0x1d56, 0x1664, 0xd3fd,
    0xf407, 0x319e, 0x3aac, 0xff35, 0x2cc8, 0xe951, 0xe263, 0x27fa,
    0xad97, 0x680e, 0x633c, 0xa6a5, 0x7558, 0xb0c1, 0xbbf3, 0x7e6a,
    0x5990, 0x9c09, 0x973b, 0x52a2, 0x815f, 0x44c6, 0x4ff4, 0x8a6d,
    0x5b2e, 0x9eb7, 0x9585, 0x501c, 0x83e1, 0x4678, 0x4d4a, 0x88d3,
    0xaf29, 0x6ab0, 0x6182, 0xa41b, 0x77e6, 0xb27f, 0xb94d, 0x7cd4,
    0xf6b9, 0x3320, 0x3812, 0xfd8b, 0x2e76, 0xebef, 0xe0dd, 0x2544,
    0x02be, 0xc727, 0xcc15, 0x098c, 0xda71, 0x1fe8, 0x14da, 0xd143,
    0xf3c5, 0x365c, 0x3d6e, 0xf8f7, 0x2b0a, 0xee93, 0xe5a1, 0x2038,
    0x07c2, 0xc25b, 0xc969, 0x0cf0, 0xdf0d, 0x1a94, 0x11a6, 0xd43f,
    0x5e52, 0x9bcb, 0x90f9, 0x5560, 0x869d, 0x4304, 0x4836, 0x8daf,
    0xaa55, 0x6fcc, 0x64fe, 0xa167, 0x729a, 0xb703, 0xbc31, 0x79a8,
    0xa8eb, 0x6d72, 0x6640, 0xa3d9, 0x7024, 0xb5bd, 0xbe8f, 0x7b16,
    0x5cec, 0x9975, 0x9247, 0x57de, 0x8423, 0x41ba, 0x4a88, 0x8f11,
    0x057c, 0xc0e5, 0xcbd7, 0x0e4e, 0xddb3, 0x182a, 0x1318, 0xd681,
    0xf17b, 0x34e2, 0x3fd0, 0xfa49, 0x29b4, 0xec2d, 0xe71f, 0x2286,
    0xa213, 0x678a, 0x6cb8, 0xa921, 0x7adc, 0xbf45, 0xb477, 0x71ee,
    0x5614, 0x938d, 0x98bf, 0x5d26, 0x8edb, 0x4b42, 0x4070, 0x85e9,
    0x0f84, 0xca1d, 0xc12f, 0x04b6, 0xd74b, 0x12d2, 0x19e0, 0xdc79,
    0xfb83, 0x3e1a, 0x3528, 0xf0b1, 0x234c, 0xe6d5, 0xede7, 0x287e,
    0xf93d, 0x3ca4, 0x3796, 0xf20f, 0x21f2, 0xe46b, 0xef59, 0x2ac0,
    0x0d3a, 0xc8a3, 0xc391, 0x0608, 0xd5f5, 0x106c, 0x1b5e, 0xdec7,
    0x54aa, 0x9133, 0x9a01, 0x5f98, 0x8c65, 0x49fc, 0x42ce, 0x8757,
    0xa0ad, 0x6534, 0x6e06, 0xab9f, 0x7862, 0xbdfb, 0xb6c9, 0x7350,
    0x51d6, 0x944f, 0x9f7d, 0x5ae4, 0x8919, 0x4c80, 0x47b2, 0x822b,
    0xa5d1, 0x6048, 0x6b7a, 0xaee3, 0x7d1e, 0xb887, 0xb3b5, 0x762c,
    0xfc41, 0x39d8, 0x32ea, 0xf773, 0x248e, 0xe117, 0xea25, 0x2fbc,
    0x0846, 0xcddf, 0xc6ed, 0x0374, 0xd089, 0x1510, 0x1e22, 0xdbbb,
    0x0af8, 0xcf61, 0xc453, 0x01ca, 0xd237, 0x17ae, 0x1c9c, 0xd905,
    0xfeff, 0x3b66, 0x3054, 0xf5cd, 0x2630, 0xe3a9, 0xe89b, 0x2d02,
    0xa76f, 0x62f6, 0x69c4, 0xac5d, 0x7fa0, 0xba39, 0xb10b, 0x7492,
    0x5368, 0x96f1, 0x9dc3, 0x585a, 0x8ba7, 0x4e3e, 0x450c, 0x8095,
];

const CRC10_TABLE: [u16; 256] = [
    0x0000, 0x008f, 0x011e, 0x0191, 0x023c, 0x02b3, 0x0322, 0x03ad,
    0x00f7, 0x0078, 0x01e9, 0x0166, 0x02cb, 0x0244, 0x03d5, 0x035a,
    0x01ee, 0x0161, 0x00f0, 0x007f, 0x03d2, 0x035d, 0x02cc, 0x0243,
    0x0119, 0x0196, 0x0007, 0x0088, 0x0325, 0x03aa, 0x023b, 0x02b4,
    0x03dc, 0x0353, 0x02c2, 0x024d, 0x01e0, 0x016f, 0x00fe, 0x0071,
    0x032b, 0x03a4, 0x0235, 0x02ba, 0x0117, 0x0198, 0x0009, 0x0086,
    0x0232, 0x02bd, 0x032c, 0x03a3, 0x000e, 0x0081, 0x0110, 0x019f,
    0x02c5, 0x024a, 0x03db, 0x0354, 0x00f9, 0x0076, 0x01e7, 0x0168,
    0x0337, 0x03b8, 0x0229, 0x02a6, 0x010b, 0x0184, 0x0015, 0x009a,
    0x03c0, 0x034f, 0x02de, 0x0251, 0x01fc, 0x0173, 0x00e2, 0x006d,
    0x02d9, 0x0256, 0x03c7, 0x0348, 0x00e5, 0x006a, 0x01fb, 0x0174,
    0x022e, 0x02a1, 0x0330, 0x03bf, 0x0012, 0x009d, 0x010c, 0x0183,
    0x00eb, 0x0064, 0x01f5, 0x017a, 0x02d7, 0x0258, 0x03c9, 0x0346,
    0x001c, 0x0093, 0x0102, 0x018d, 0x0220, 0x02af, 0x033e, 0x03b1,
    0x0105, 0x018a, 0x001b, 0x0094, 0x0339, 0x03b6, 0x0227, 0x02a8,
    0x01f2, 0x017d, 0x00ec, 0x0063, 0x03ce, 0x0341, 0x02d0, 0x025f,
    0x02e1, 0x026e, 0x03ff, 0x0370, 0x00dd, 0x0052, 0x01c3, 0x014c,
    0x0216, 0x0299, 0x0308, 0x0387, 0x002a, 0x00a5, 0x0134, 0x01bb,
    0x030f, 0x0380, 0x0211, 0x029e, 0x0133, 0x01bc, 0x002d, 0x00a2,
    0x03f8, 0x0377, 0x02e6, 0x0269, 0x01c4, 0x014b, 0x00da, 0x0055,
    0x013d, 0x01b2, 0x0023, 0x00ac, 0x0301, 0x038e, 0x021f, 0x0290,
    0x01ca, 0x0145, 0x00d4, 0x005b, 0x03f6, 0x0379, 0x02e8, 0x0267,
    0x00d3, 0x005c, 0x01cd, 0x0142, 0x02ef, 0x0260, 0x03f1, 0x037e,
    0x0024, 0x00ab, 0x013a, 0x01b5, 0x0218, 0x0297, 0x0306, 0x0389,
    0x01d6, 0x0159, 0x00c8, 0x0047, 0x03ea, 0x0365, 0x02f4, 0x027b,
    0x0121, 0x01ae, 0x003f, 0x00b0, 0x031d, 0x0392, 0x0203, 0x028c,
    0x0038, 0x00b7, 0x0126, 0x01a9, 0x0204, 0x028b, 0x031a, 0x0395,
    0x00cf, 0x0040, 0x01d1, 0x015e, 0x02f3, 0x027c, 0x03ed, 0x0362,
    0x020a, 0x0285, 0x0314, 0x039b, 0x0036, 0x00b9, 0x0128, 0x01a7,
    0x02fd, 0x0272, 0x03e3, 0x036c, 0x00c1, 0x004e, 0x01df, 0x0150,
    0x03e4, 0x036b, 0x02fa, 0x0275, 0x01d8, 0x0157, 0x00c6, 0x0049,
    0x0313, 0x039c, 0x020d, 0x0282, 0x012f, 0x01a0, 0x0031, 0x00be,
];
