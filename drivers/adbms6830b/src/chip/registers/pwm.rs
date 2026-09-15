//! PWM registers....

use bitfield_struct::{bitfield, bitenum};
use adbms6830b_macros::BitfieldEnumDefault;

use super::register_group;
use super::super::commands;

/// Field types relavent to the PWM registers. See Table 111 on page 74 of the datasheet.
pub mod types {
    use super::{bitenum, BitfieldEnumDefault};

    /// PWM configuration (PWMCx). Four-bit field. Defaults to `0b0000` (disabled).
    /// 
    /// This field refers to the PWM duty cycle configuration for a cell (`x`).
    /// 
    /// Note: The PWM period is 937 ms.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum PwmDutyCycleConfig {
        /// Disabled (default).
        #[default]
        #[fallback]
        Disabled = 0,
        /// 6.6% duty cycle.
        Pct06_6 = 1,
        /// 13.2% duty cycle.
        Pct13_2 = 2,
        /// 19.8% duty cycle.
        Pct19_8 = 3,
        /// 26.4% duty cycle.
        Pct26_4 = 4,
        /// 33.0% duty cycle.
        Pct33_0 = 5,
        /// 39.6% duty cycle.
        Pct39_6 = 6,
        /// 46.2% duty cycle.
        Pct46_2 = 7,
        /// 52.8% duty cycle.
        Pct52_8 = 8,
        /// 59.4% duty cycle.
        Pct59_4 = 9,
        /// 66.0% duty cycle.
        Pct66_0 = 10,
        /// 72.6% duty cycle.
        Pct72_6 = 11,
        /// 79.2% duty cycle.
        Pct79_2 = 12,
        /// 85.8% duty cycle.
        Pct85_8 = 13,
        /// 92.4% duty cycle.
        Pct92_4 = 14,
        /// ~100% duty cycle.
        Pct100_0 = 15,
    }
    impl PwmDutyCycleConfig {
        /// Whether or not this PWM config configures the cell to be balancing.
        /// 
        /// A cell is balancing if its PwmDutyCycleConfig is anything other than `Disabled`.
        pub const fn is_balancing(&self) -> bool {
            !matches!(*self, PwmDutyCycleConfig::Disabled)
        }
    }
}

/// PWM Register Group A (PWMA). Contains six 1-byte registers (so 6 bytes total).
/// 
/// This contains PWM1 through PWM12
/// 
/// See Table 95 on page 68 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::pwm::wrpwma().frame()),
    read = Some(commands::pwm::rdpwma().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct PwmA {
    /// Cell 1 PWM Duty Cycle config. Corresponds to `PWM1[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm1: types::PwmDutyCycleConfig,
    /// Cell 2 PWM Duty Cycle config. Corresponds to `PWM2[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm2: types::PwmDutyCycleConfig,

    /// Cell 3 PWM Duty Cycle config. Corresponds to `PWM3[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm3: types::PwmDutyCycleConfig,
    /// Cell 4 PWM Duty Cycle config. Corresponds to `PWM4[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm4: types::PwmDutyCycleConfig,

    /// Cell 5 PWM Duty Cycle config. Corresponds to `PWM5[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm5: types::PwmDutyCycleConfig,
    /// Cell 6 PWM Duty Cycle config. Corresponds to `PWM6[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm6: types::PwmDutyCycleConfig,

    /// Cell 7 PWM Duty Cycle config. Corresponds to `PWM7[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm7: types::PwmDutyCycleConfig,
    /// Cell 8 PWM Duty Cycle config. Corresponds to `PWM8[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm8: types::PwmDutyCycleConfig,

    /// Cell 9 PWM Duty Cycle config. Corresponds to `PWM9[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm9: types::PwmDutyCycleConfig,
    /// Cell 10 PWM Duty Cycle config. Corresponds to `PWM10[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm10: types::PwmDutyCycleConfig,

    /// Cell 11 PWM Duty Cycle config. Corresponds to `PWM11[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm11: types::PwmDutyCycleConfig,
    /// Cell 12 PWM Duty Cycle config. Corresponds to `PWM12[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm12: types::PwmDutyCycleConfig,

    #[bits(16, default = 0)]                                  _padding: u16,
}

/// PWM Register Group B (PWMB). Contains six 1-byte registers (so 6 bytes total), but the last 4 bytes are reserved (constant 1s).
/// 
/// This contains PWM13 through PWM16.
/// 
/// See Table 96 on page 69 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::pwm::wrpwmb().frame()),
    read = Some(commands::pwm::rdpwmb().frame()),
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct PwmB {
    /// Cell 13 PWM Duty Cycle config. Corresponds to `PWM13[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm13: types::PwmDutyCycleConfig,
    /// Cell 14 PWM Duty Cycle config. Corresponds to `PWM14[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm14: types::PwmDutyCycleConfig,

    /// Cell 15 PWM Duty Cycle config. Corresponds to `PWM15[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm15: types::PwmDutyCycleConfig,
    /// Cell 16 PWM Duty Cycle config. Corresponds to `PWM16[3:0]`.
    #[bits(4, default = types::PwmDutyCycleConfig::DEFAULT)]  pub pwm16: types::PwmDutyCycleConfig,

    #[bits(8, default = 0xFF)]                                _psr2: u8,
    #[bits(8, default = 0xFF)]                                _psr3: u8,
    #[bits(8, default = 0xFF)]                                _psr4: u8,
    #[bits(8, default = 0xFF)]                                _psr5: u8,

    #[bits(16, default = 0)]                                  _padding: u16,
}