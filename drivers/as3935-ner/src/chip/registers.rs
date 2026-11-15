/// AS3935 register addresses (datasheet "Register Table").
///
/// Each register is one byte. The doc comment on each variant lists the
/// fields packed into that byte, from bit 7 (most significant) down to bit 0.
/// `[6:4]` means "bits 6 down to 4", i.e. a 3-bit field. "Reserved" bits have
/// no function and must be left unchanged when writing the register.
///
/// The lightning detection lookup table (0x09–0x32) is not used by this driver.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Register {
    /// - [7:6] Reserved
    /// - [5:1] AFE gain boost (indoor/outdoor setting)
    /// - [0]   Power down
    Config = 0x00,
    /// - [7]   Reserved
    /// - [6:4] Noise floor level
    /// - [3:0] Watchdog threshold
    NoiseWatchdog = 0x01,
    /// - [7]   Reserved
    /// - [6]   Clear statistics
    /// - [5:4] Minimum number of lightning strikes
    /// - [3:0] Spike rejection
    Statistics = 0x02,
    /// - [7:6] Frequency division ratio for antenna tuning
    /// - [5]   Mask disturber
    /// - [4]   Reserved
    /// - [3:0] Interrupt reason (read-only)
    Frequency = 0x03,
    /// - [7:0] Energy of the single lightning, least significant byte (read-only)
    EnergyLsb = 0x04,
    /// - [7:0] Energy of the single lightning, middle byte (read-only)
    EnergyMsb = 0x05,
    /// - [7:5] Reserved
    /// - [4:0] Energy of the single lightning, most significant bits (read-only)
    EnergyMmsb = 0x06,
    /// - [7:6] Reserved
    /// - [5:0] Distance estimation (read-only)
    DistanceEstimation = 0x07,
    /// - [7]   Display LCO on IRQ pin
    /// - [6]   Display SRCO on IRQ pin
    /// - [5]   Display TRCO on IRQ pin
    /// - [4]   Reserved
    /// - [3:0] Internal tuning capacitors (0–120 pF in steps of 8 pF)
    Display = 0x08,
    /// - [7]   TRCO calibration done
    /// - [6]   TRCO calibration unsuccessful
    /// - [5:0] Reserved
    ///
    /// Read-only.
    TrcoCalibration = 0x3A,
    /// - [7]   SRCO calibration done
    /// - [6]   SRCO calibration unsuccessful
    /// - [5:0] Reserved
    ///
    /// Read-only.
    SrcoCalibration = 0x3B,
    /// Direct command: write `0x96` to reset all registers to their defaults.
    PresetDefault = 0x3C,
    /// Direct command: write `0x96` to calibrate the internal RC oscillators.
    CalibRco = 0x3D,
}
