use uom::si::f32::Length;
use uom::si::length::kilometer;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Distance {
    Overhead,
    OutOfRange,
    Estimate(Length),
}

impl defmt::Format for Distance {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Distance::Overhead => defmt::write!(f, "overhead"),
            Distance::OutOfRange => defmt::write!(f, "out of range"),
            Distance::Estimate(d) => defmt::write!(f, "{} km", d.get::<kilometer>()),
        }
    }
}
