//! Configuration register groups A and B.

use bitfield_struct::bitfield;

use crate::registers::{AdbmsReadableRegister, AdbmsWritableRegister};
use crate::types::Threshold;

// =============================================================================
// Configuration Register Group A
// =============================================================================

/// Configuration Register Group A
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct ConfigA {
    /// C-ADC vs S-ADC comparison voltage threshold (datasheet Table 102).
    #[bits(3, default = CompThreshold::Mv8_1)]
    pub cth: CompThreshold,
    #[bits(4)]
    __: u8,
    /// Reference powered up
    /// if true, reference remains powered on until watchdog timeout,
    /// if false, shuts down after conv.
    pub refon: bool,
    /// forces oscillator counter fast.
    pub flag_d_osccnt_fast: bool,
    /// forces oscillator counter slow.
    pub flag_d_osccnt_slow: bool,
    /// forces supply error detection (ED).
    pub flag_d_ed: bool,
    /// selects supply OV and delta detection. 0 = selects UV.
    pub flag_d_uvov: bool,
    /// sets THSD
    pub flag_d_thsd: bool,
    /// forces nonvolatile memory (NVM) error detection (ED). Sets CED and SED.
    pub flag_d_nvm: bool,
    /// forces NVM multiple error detection (MED). Sets CMED and SMED.
    pub flag_d_nvm_med: bool,
    /// forces TMODCHK
    pub flag_d_tmodchk: bool,
    #[bits(3)]
    __: u8,
    /// Open-wire soak time.
    /// If OWRNG = 0, soak time = 2^(6 + OWA[2:0]) clocks (32 us to 4.1 ms).
    /// If OWRNG = 1, soak time = 2^(13 + OWA[2:0]) clocks (4.1 ms to 524 ms).
    #[bits(3)]
    pub owa: u8,
    /// Open-wire soak time range (`1` = long, `0` = short).
    pub owrng: bool,
    /// Enable soak on AUX ADCs.
    pub soakon: bool,
    /// GPIO pull-down disable mask (`1` = GPIO drives high / pull-down off).
    /// Power-on default is all 1s.
    #[bits(10, default = 0x3FF)]
    pub gpo: u16,
    #[bits(6)]
    __: u8,
    /// IIR filter corner frequency.
    #[bits(3)]
    pub fc: IirCorner,
    /// Communication break (`1` = stop forwarding daisy-chain traffic).
    pub comm_bk: bool,
    /// Mute status (`1` = discharging disabled).
    pub mute_st: bool,
    /// Snapshot status (`1` = result registers frozen).
    pub snap_st: bool,
    #[bits(2)]
    __: u8,
    #[bits(16)]
    __: u16,
}

impl AdbmsWritableRegister for ConfigA {
    const WRITE_CMD: [u8; 2] = [0x00, 0x01];
}
impl AdbmsReadableRegister for ConfigA {
    const READ_CMD: [u8; 2] = [0x00, 0x02];
}

impl ConfigA {
    /// Power-on style preset: reference always on, IIR disabled, GPO
    /// pull-downs off, comparator threshold at the datasheet default.
    pub const fn boot_default() -> Self {
        Self::new().with_refon(true)
    }
}

/// C-ADC vs S-ADC comparison voltage threshold.
///
/// Defaults to `001` (8.1 mV) per datasheet Table 102.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CompThreshold {
    Mv5_1 = 0b000,
    #[default]
    Mv8_1 = 0b001,
    Mv9_0 = 0b010,
    Mv10_05 = 0b011,
    Mv15_0 = 0b100,
    Mv19_95 = 0b101,
    Mv25_05 = 0b110,
    Mv40_05 = 0b111,
}

impl CompThreshold {
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
    pub const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Mv5_1,
            1 => Self::Mv8_1,
            2 => Self::Mv9_0,
            3 => Self::Mv10_05,
            4 => Self::Mv15_0,
            5 => Self::Mv19_95,
            6 => Self::Mv25_05,
            _ => Self::Mv40_05,
        }
    }
}

/// IIR filter parameter (3-bit `fc` field of [`ConfigA`]).
///
/// Values are the filter parameter `a` from datasheet Table 21 paired
/// with the corresponding -3 dB corner frequency. The chip samples the
/// C-ADC at 1 kHz, so the corner frequencies are referenced to that.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IirCorner {
    /// `000`: filter disabled.
    Disabled = 0b000,
    /// `001`: a = 2,   -3 dB corner ≈ 110 Hz.
    Fpa2 = 0b001,
    /// `010`: a = 4,   -3 dB corner ≈ 45 Hz.
    Fpa4 = 0b010,
    /// `011`: a = 8,   -3 dB corner ≈ 21 Hz.
    Fpa8 = 0b011,
    /// `100`: a = 16,  -3 dB corner ≈ 10 Hz.
    Fpa16 = 0b100,
    /// `101`: a = 32,  -3 dB corner ≈ 5 Hz.
    Fpa32 = 0b101,
    /// `110`: a = 128, -3 dB corner ≈ 1.25 Hz.
    Fpa128 = 0b110,
    /// `111`: a = 256, -3 dB corner ≈ 0.625 Hz.
    Fpa256 = 0b111,
}

impl IirCorner {
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
    pub const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Disabled,
            1 => Self::Fpa2,
            2 => Self::Fpa4,
            3 => Self::Fpa8,
            4 => Self::Fpa16,
            5 => Self::Fpa32,
            6 => Self::Fpa128,
            _ => Self::Fpa256,
        }
    }
}

// =============================================================================
// Configuration Register Group B
// =============================================================================

/// Configuration Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct ConfigB {
    /// Cell undervoltage comparison voltage.
    #[bits(12, default = Threshold(0x800))]
    pub vuv: Threshold,
    /// Cell overvoltage comparison voltage.
    #[bits(12, default = Threshold(0x7FF))]
    pub vov: Threshold,
    /// Discharge timeout value. Units depend on `dtrng`: 1-minute steps
    /// when low, 16-minute steps when high. `0` = timeout / not set.
    #[bits(6)]
    pub dcto: u8,
    /// Discharge timer range: `false` = 0–63 min in 1 min steps,
    /// `true` = 0–16.8 h in 16 min steps.
    pub dtrng: bool,
    /// Enable discharge timer monitor (extended balancing state).
    pub dtmen: bool,
    /// Per-cell discharge enable mask. Bit `n` = cell `n+1` discharge on.
    pub dcc: u16,
    #[bits(16)]
    __: u16,
}

impl AdbmsWritableRegister for ConfigB {
    const WRITE_CMD: [u8; 2] = [0x00, 0x24];
}
impl AdbmsReadableRegister for ConfigB {
    const READ_CMD: [u8; 2] = [0x00, 0x26];
}
