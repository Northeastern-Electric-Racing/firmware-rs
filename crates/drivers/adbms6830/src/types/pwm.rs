//! PWM Register Groups A & B (`WRPWMA` / `RDPWMA` / `WRPWMB` / `RDPWMB`).
//! A PWM value ranges from 0 to 100 percent over 0b000 to 0b1111.

use bitfield_struct::bitfield;

use crate::registers::{AdbmsReadableRegister, AdbmsWritableRegister};

/// PWM Register Group A.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct PwmA {
    #[bits(4)]
    pub pwm1: u8,
    #[bits(4)]
    pub pwm2: u8,
    #[bits(4)]
    pub pwm3: u8,
    #[bits(4)]
    pub pwm4: u8,
    #[bits(4)]
    pub pwm5: u8,
    #[bits(4)]
    pub pwm6: u8,
    #[bits(4)]
    pub pwm7: u8,
    #[bits(4)]
    pub pwm8: u8,
    #[bits(4)]
    pub pwm9: u8,
    #[bits(4)]
    pub pwm10: u8,
    #[bits(4)]
    pub pwm11: u8,
    #[bits(4)]
    pub pwm12: u8,
    #[bits(16)]
    __: u16,
}

impl AdbmsWritableRegister for PwmA {
    const WRITE_CMD: [u8; 2] = [0x00, 0x20];
}
impl AdbmsReadableRegister for PwmA {
    const READ_CMD: [u8; 2] = [0x00, 0x22];
}

/// PWM Register Group B.
#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct PwmB {
    #[bits(4)]
    pub pwm13: u8,
    #[bits(4)]
    pub pwm14: u8,
    #[bits(4)]
    pub pwm15: u8,
    #[bits(4)]
    pub pwm16: u8,
    #[bits(32)]
    __: u32,
    #[bits(16)]
    __: u16,
}

impl AdbmsWritableRegister for PwmB {
    const WRITE_CMD: [u8; 2] = [0x00, 0x21];
}
impl AdbmsReadableRegister for PwmB {
    const READ_CMD: [u8; 2] = [0x00, 0x23];
}
