/// Analog front-end gain setting, chosen for where the sensor is mounted.
/// Passed to [`As3935::set_afe`](crate::As3935::set_afe).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Afe {
    /// Higher gain, for use inside a building or enclosure.
    Indoor = 0b10010,
    /// Lower gain, for use outdoors.
    Outdoor = 0b01110,
}

/// Why the IRQ pin fired. Returned by [`As3935::interrupt`](crate::As3935::interrupt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    /// No interrupt is pending.
    None,
    /// The background noise is too high for the sensor to detect lightning.
    NoiseTooHigh,
    /// A disturber (a signal that looked like lightning but wasn't) was detected and rejected.
    Disturber,
    /// Lightning was detected. Read [`distance`](crate::As3935::distance) and
    /// [`energy`](crate::As3935::energy) next.
    Lightning,
}

/// Estimated distance to the front of the storm.
/// Returned by [`As3935::distance`](crate::As3935::distance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distance {
    /// The storm is directly overhead.
    Overhead,
    /// The storm is too far away to estimate (beyond about 40 km).
    OutOfRange,
    /// Estimated distance in kilometers.
    Km(u8),
}
