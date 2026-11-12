//! SPI timing, from ADBMS6830B Table 11 and ADBMS6822 Table 9.
//!
//! Both bind the pins we drive: the 6822 transports chip select rather than terminating it, so the
//! CS-high the chip sees is the CS-high we drove. Each constant is the larger of the two tables --
//! which matters for t5, where Table 9 asks 1 us and Table 11 asks 2 us.
//!
//! `ExclusiveDevice` enforces none of them. t5 spans *between* transactions, so [`BusIdle`] covers
//! it; t6 and t7 sit inside one, around a chip select this driver never touches, so they go in as
//! [`cs_setup`] and [`cs_hold`] operations at either end.
//!
//! t_CLK, t6 and t7 budget 50 ns of cable delay each direction (~10 m of Cat-5) and need derating
//! on a longer harness; t5 does not.

use embedded_hal_async::spi::Operation;

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

/// t7, as the opening operation of a transaction.
///
/// Inside the transaction because chip select is already low by then and this driver never touches
/// it. The `SpiDevice` serves the wait, so `ExclusiveDevice::new_no_delay(...)` panics on it.
pub const fn cs_setup<'a>() -> Operation<'a, u8> {
    Operation::DelayNs(CS_SETUP_NS)
}

/// t6, as the closing operation of a transaction.
///
/// Transactions that clock nothing, such as a wake pulse, need neither this nor [`cs_setup`].
pub const fn cs_hold<'a>() -> Operation<'a, u8> {
    Operation::DelayNs(CS_HOLD_NS)
}
