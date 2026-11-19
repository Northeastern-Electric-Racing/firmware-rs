#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Afe {
    Indoor = 0b10010,
    Outdoor = 0b01110,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Interrupt {
    None,
    NoiseTooHigh,
    Disturber,
    Lightning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Distance {
    Overhead,
    OutOfRange,
    Km(u8),
}
