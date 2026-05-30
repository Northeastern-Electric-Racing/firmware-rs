//! Status register groups (`RDSTATx`, all read-only).

use bitfield_struct::bitfield;

use crate::registers::{AdbmsCommand, AdbmsReadableRegister};
use crate::types::{AdbmsCellVoltage, DieTemp};

pub struct ClrFlag {}
impl AdbmsCommand for ClrFlag {
    fn get_command(&self) -> u16 {
        0b11100010111
    }
}

pub struct ClOvUv {}
impl AdbmsCommand for ClOvUv {
    fn get_command(&self) -> u16 {
        0b11100010101
    }
}

// =============================================================================
// Status A
// =============================================================================

/// Status Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct StatusA {
    /// Secondary reference voltage (Cell scaling).
    #[bits(16)]
    pub vref2: AdbmsCellVoltage,
    /// Internal die temperature.
    #[bits(16)]
    pub itmp: DieTemp,
    /// Third reference voltage (Cell scaling). UNDOCUMENTED
    #[bits(16)]
    pub vref3: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for StatusA {
    const READ_CMD: [u8; 2] = [0x00, 0x30];
}

// =============================================================================
// Status B
// =============================================================================

/// Status Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct StatusB {
    /// Digital supply voltage (typically 2.7–3.6 V).
    #[bits(16)]
    pub vd: AdbmsCellVoltage,
    /// Analog supply voltage (typically 4.5–5.5 V).
    #[bits(16)]
    pub va: AdbmsCellVoltage,
    /// VREF2 across the open-wire check series resistor.
    #[bits(16)]
    pub vres: AdbmsCellVoltage,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for StatusB {
    const READ_CMD: [u8; 2] = [0x00, 0x31];
}

// =============================================================================
// Status C
// =============================================================================

/// Status Register Group C (`RDSTATC`).
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct StatusC {
    /// C-ADC vs S-ADC mismatch flags, one bit per channel. Bit `n` is
    /// set when channel `n+1` had a mismatch.
    pub cs_flt: u16,
    pub counter_raw: u16,
    pub smed: bool,
    pub sed: bool,
    pub cmed: bool,
    pub ced: bool,
    pub vd_uv: bool,
    pub vd_ov: bool,
    pub va_uv: bool,
    pub va_ov: bool,
    pub oscchk: bool,
    pub tmodchk: bool,
    pub thsd: bool,
    pub sleep: bool,
    pub spiflt: bool,
    pub comp: bool,
    pub vde: bool,
    pub vdel: bool,
    #[bits(16)]
    __: u16,
}

impl StatusC {
    /// 11-bit C-ADC conversion counter `CT[10:0]`, reconstructed from
    /// the split layout in bytes 2 & 3.
    pub const fn ct(&self) -> u16 {
        let raw = self.counter_raw();
        let ct_hi = raw & 0x001F; // CT[10:6] at u16 bits 0..4 (byte 2 low nibble + 1)
        let ct_lo = (raw >> 10) & 0x003F; // CT[5:0] at u16 bits 10..15 (byte 3 high 6)
        (ct_hi << 6) | ct_lo
    }
    /// 2-bit C-ADC sub-sample counter `CTS[1:0]`.
    pub const fn cts(&self) -> u8 {
        ((self.counter_raw() >> 8) & 0x03) as u8
    }
    /// 13-bit combined counter `CCTS[12:0] = (CT << 2) | CTS`. Increments
    /// 4× per C-ADC sample; resets on every `ADCV`.
    pub const fn ccts(&self) -> u16 {
        (self.ct() << 2) | self.cts() as u16
    }
}

impl AdbmsReadableRegister for StatusC {
    const READ_CMD: [u8; 2] = [0x00, 0x32];
}

// =============================================================================
// Status D
// =============================================================================

/// Status Register Group D.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct StatusD {
    /// Raw 32-bit interleaved OV/UV flags. Bit `2(n-1)` = `CnUV`,
    /// bit `2(n-1)+1` = `CnOV` for cell `n` ∈ `1..=16`.
    pub ov_uv_packed: u32,
    /// Byte 4 reads back as `0xFF` per datasheet.
    #[bits(8)]
    __: u8,
    /// Oscillator check counter (stores most recent or first failing
    /// count; passing range 52..=71).
    pub oc_cntr: u8,
    #[bits(16)]
    __: u16,
}

impl StatusD {
    /// `true` when cell `n` (1..=16) is flagged under-voltage.
    pub const fn cell_uv(&self, n: u8) -> bool {
        let shift = 2 * (n - 1);
        (self.ov_uv_packed() >> shift) & 1 == 1
    }
    /// `true` when cell `n` (1..=16) is flagged over-voltage.
    pub const fn cell_ov(&self, n: u8) -> bool {
        let shift = 2 * (n - 1) + 1;
        (self.ov_uv_packed() >> shift) & 1 == 1
    }
    /// Bitmask of `CxUV` for `x = 1..=16` (bit 0 = cell 1).
    pub const fn uv_mask(&self) -> u16 {
        let p = self.ov_uv_packed();
        let mut out = 0u16;
        let mut i = 0u32;
        while i < 16 {
            out |= (((p >> (2 * i)) & 1) as u16) << i;
            i += 1;
        }
        out
    }
    /// Bitmask of `CxOV` for `x = 1..=16` (bit 0 = cell 1).
    pub const fn ov_mask(&self) -> u16 {
        let p = self.ov_uv_packed();
        let mut out = 0u16;
        let mut i = 0u32;
        while i < 16 {
            out |= (((p >> (2 * i + 1)) & 1) as u16) << i;
            i += 1;
        }
        out
    }
}

impl AdbmsReadableRegister for StatusD {
    const READ_CMD: [u8; 2] = [0x00, 0x33];
}

// =============================================================================
// Status E
// =============================================================================

/// Status Register Group E.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct StatusE {
    /// Reserved (bytes 0..=3 of the register).
    #[bits(32)]
    __: u32,
    /// GPIO input states `GPI[1:10]` as a 10-bit mask. Bit `n` = `GPI[n+1]`.
    #[bits(10)]
    pub gpi: u16,
    #[bits(2)]
    __: u8,
    /// Silicon revision code.
    #[bits(4)]
    pub rev: u8,
    #[bits(16)]
    __: u16,
}
impl AdbmsReadableRegister for StatusE {
    const READ_CMD: [u8; 2] = [0x00, 0x34];
}
