//! Generic CAN handler for NER STM32H5 firmware projects.
//!
//! This crate wraps Embassy's `embassy-stm32` FDCAN peripheral to provide a
//! ready-to-use Classical CAN configuration and an [`embassy_executor`] task
//! ([`can_handler`]) that bridges the CAN bus with the rest of a user program
//! over [`embassy_sync`] channels.
//!
//! The bus is configured for Classical CAN at 500 kbit/s.  
#![no_std]
use defmt::warn;
use embassy_futures::select::{Either, select};
use embassy_stm32::can::filter::FilterType::{DedicatedDual, DedicatedSingle};
use embassy_stm32::can::filter::{
    Action, ExtendedFilter, ExtendedFilterSlot, StandardFilter, StandardFilterSlot,
};
use embassy_stm32::can::{CanConfigurator, Frame};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embedded_can::{ExtendedId, StandardId};

use heapless::Vec;

// Ripped from shep3 code because it works there
pub mod interrupts {
    use core::sync::atomic::{AtomicU32, Ordering};

    /// Number of times the FDCAN2 IT0 interrupt has fired.
    pub static IT0_IRQ_COUNT: AtomicU32 = AtomicU32::new(0);
    /// Number of times the FDCAN2 IT1 interrupt has fired.
    pub static IT1_IRQ_COUNT: AtomicU32 = AtomicU32::new(0);

    /// Counts FDCAN2 IT0 entries. Also clears the TCF flag. This runs alongside embassy's internal ISR (it doesn't replace it or anything)
    struct It0Counter;
    impl
        embassy_stm32::interrupt::typelevel::Handler<
            embassy_stm32::interrupt::typelevel::FDCAN2_IT0,
        > for It0Counter
    {
        unsafe fn on_interrupt() {
            // Clear IR.TCF (transmission cancellation finished) interrupt flag.
            // We need to do this because we enable this interrupt manually and embassy doesn't clear this in its internal interrupt handler.
            let regs = embassy_stm32::pac::FDCAN2;
            if regs.ir().read().tcf() {
                regs.ir().write(|w| w.set_tcf(true));
            }

            IT0_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Counts FDCAN2 IT1 entries. This runs alongside embassy's internal ISR (it doesn't replace it or anything)
    struct It1Counter;
    impl
        embassy_stm32::interrupt::typelevel::Handler<
            embassy_stm32::interrupt::typelevel::FDCAN2_IT1,
        > for It1Counter
    {
        unsafe fn on_interrupt() {
            IT1_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    embassy_stm32::bind_interrupts!(pub struct Irqs {
        // NOTE: `It0Counter` is listed first on purpose so it runs before the embassy ISR handler.
        FDCAN2_IT0 => It0Counter, embassy_stm32::can::IT0InterruptHandler<embassy_stm32::peripherals::FDCAN2>;
        FDCAN2_IT1 => embassy_stm32::can::IT1InterruptHandler<embassy_stm32::peripherals::FDCAN2>, It1Counter;
    });
}

pub struct NerCan {
    pub can_configurator: CanConfigurator<'static>,
    used_std_slots: Vec<StandardFilterSlot, 28>,
    used_ext_slots: Vec<ExtendedFilterSlot, 28>,
}

impl NerCan {
    /// This is the CAN configuration to be used by most NER Projects.
    /// This is for optional use to pass into the can_handler task to facilitate initialization
    ///
    /// The configuration sets:
    /// - Automatic bus-off recovery enabled.
    /// - Automatic retransmission disabled.
    /// - Classical CAN framing only (no CAN FD).
    /// - A clock divider of 1 and the data bit timing required for 500 kbit/s.
    /// - Transmit pause enabled.
    /// - A global filter that rejects all frames by default.
    ///
    /// ** It is expected that the user manually configures the CAN Std and Extended Filters before running the can_handler task
    /// ** Hardcodes bitrate to 500 kbit/s, if CAN sampling causes issues, this must be adjusted in this lib
    pub fn init(mut can_configurator: CanConfigurator<'static>) -> Self {
        use embassy_stm32::can::config::*;

        let can_config = FdCanConfig::default()
            .set_automatic_bus_off_recovery(true)
            .set_automatic_retransmit(false)
            .set_frame_transmit(FrameTransmissionConfig::ClassicCanOnly)
            .set_transmit_pause(true)
            .set_global_filter(GlobalFilter::reject_all());
        can_configurator.set_config(can_config);
        can_configurator.set_bitrate(500_000);

        Self {
            can_configurator,
            used_std_slots: Vec::new(),
            used_ext_slots: Vec::new(),
        }
    }
    #[must_use]
    /// Sets adds a new CAN Standard Filter at the given slot
    /// NOTE: will panic if the given slot is already in use
    /// Additionaly, the must_use flag is set, as a new NerCan object is returied. Discarding the
    /// value would be a waste of time
    pub fn add_standard_filter(
        mut self,
        std_filter_slot: StandardFilterSlot,
        std_id1: u16,
        std_id2: Option<u16>,
    ) -> Self {
        if self.used_std_slots.contains(&std_filter_slot) {
            panic!("The selected CAN Standard Filter Slot is already in use.");
        }

        let mut std = StandardFilter::default();
        match std_id2 {
            Some(id2) => {
                std.filter = DedicatedDual(
                    StandardId::new(std_id1).unwrap(),
                    StandardId::new(id2).unwrap(),
                );
            }
            None => {
                std.filter = DedicatedSingle(StandardId::new(std_id1).unwrap());
            }
        }
        std.action = Action::StoreInFifo0;
        self.can_configurator
            .properties()
            .set_standard_filter(std_filter_slot, std);
        let _ = self.used_std_slots.push(std_filter_slot);

        self
    }

    #[must_use]
    /// Sets adds a new CAN Extended Filter at the given slot
    /// NOTE: will panic if the given slot is already in use
    /// Additionaly, the must_use flag is set, as a new NerCan object is returied. Discarding the
    /// value would be a waste of time
    pub fn add_extended_filter(
        mut self,
        ext_filter_slot: ExtendedFilterSlot,
        ext_id1: u32,
        ext_id2: Option<u32>,
    ) -> Self {
        if self.used_ext_slots.contains(&ext_filter_slot) {
            panic!("The selected CAN Extended Filter Slot is already in use.");
        }

        let mut ext = ExtendedFilter::default();
        match ext_id2 {
            Some(id2) => {
                ext.filter = DedicatedDual(
                    ExtendedId::new(ext_id1).unwrap(),
                    ExtendedId::new(id2).unwrap(),
                );
            }
            None => {
                ext.filter = DedicatedSingle(ExtendedId::new(ext_id1).unwrap());
            }
        }
        ext.action = Action::StoreInFifo0;
        self.can_configurator
            .properties()
            .set_extended_filter(ext_filter_slot, ext);
        let _ = self.used_ext_slots.push(ext_filter_slot);

        self
    }
}

/// CAN handler Embassy task for generic use in STM32H5 projects.
///
/// Puts the configurator into normal mode and then services the bus in a loop,
///
/// **The `sender` and `receiver` are not intended to derive from the same channel
///
/// - `sender` passes on CAN frames received from the bus so they can be parsed
///   by the user program.
/// - `receiver` dispatches CAN frames queued by other threads in the user
///   program for transmission onto the bus.
///
#[embassy_executor::task]
pub async fn can_handler(
    can_configurator: CanConfigurator<'static>,
    sender: Sender<'static, ThreadModeRawMutex, Frame, 16>,
    receiver: Receiver<'static, ThreadModeRawMutex, Frame, 16>,
) {
    // Starts Classical CAN transmission and receival
    let mut can = can_configurator.into_normal_mode();

    // Loop to handle both receiving and sending
    loop {
        match select(receiver.receive(), can.read()).await {
            // Handle sending out a CAN message
            Either::First(frame) => {
                if can.write(&frame).await.is_some() {
                    warn!("Dequeing can frames!");
                }
            }
            // Handle receiving and CAN message and passing to another task
            Either::Second(res) => match res {
                Ok(can_recv) => {
                    let frame = can_recv.frame;
                    let _ = sender.send(frame).await;
                }
                Err(err) => warn!("Bus error! {}", err),
            },
        }
    }
}
