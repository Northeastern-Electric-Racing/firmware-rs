//! SPI timing requirements, Table 11. All values are minimums.
//!
//! t5 spans *between* transactions, so [`crate::line::Line`] enforces it. t6 and t7 sit inside a
//! transaction around chip select, which this driver never touches -- they belong to the
//! `SpiDevice`, and `embedded-hal-bus`'s `ExclusiveDevice` provides neither.
//!
//! Footnote 1: t_CLK, t6 and t7 already budget 50 ns of cable delay each direction (~10 m of
//! Cat-5); derate them for a longer harness. t5 does not.

/// t5: CSB rising to CSB falling, the idle between transactions.
pub const CS_IDLE_NS: u32 = 2_000;

/// t7: CSB falling to the first SCK rising edge.
pub const CS_SETUP_NS: u32 = 500;

/// t6: the last SCK rising edge to CSB rising.
pub const CS_HOLD_NS: u32 = 500;

/// Fastest SCK this chip accepts, from t_CLK's 0.5 us minimum period.
pub const MAX_SCK_HZ: u32 = 2_000_000;

/// Holds the bus idle for [`CS_IDLE_NS`].
///
/// Not a general `DelayNs` on purpose: with no duration parameter, a long wait cannot be routed
/// through a spin implementation by mistake. Any [`embedded_hal::delay::DelayNs`] satisfies it.
pub trait BusIdle {
    /// Spins for at least [`CS_IDLE_NS`]. Must not yield.
    fn idle(&mut self);
}

impl<T: embedded_hal::delay::DelayNs> BusIdle for T {
    fn idle(&mut self) {
        self.delay_ns(CS_IDLE_NS);
    }
}
