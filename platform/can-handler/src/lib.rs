//! Generic CAN handler for NER STM32H5 firmware projects.
//!
//! This crate wraps Embassy's `embassy-stm32` FDCAN peripheral to provide a
//! ready-to-use Classical CAN configuration and a pair of [`embassy_executor`]
//! tasks ([`can_tx`] and [`can_rx`]) that bridge the CAN bus with the rest of a
//! user program over [`embassy_sync`] channels.
//!
//! The bus is configured for Classical CAN at 500 kbit/s.
//!
//! # Usage
//!
//! Declare the filter set as a const table next to the project's CAN ids, then
//! build, filter and start the peripheral in one chain:
//!
//! ```ignore
//! const STD: [StdFilter; 1] = [StdFilter {
//!     slot: StandardFilterSlot::_0,
//!     id1: 0x37,
//!     id2: Some(0x01E),
//! }];
//! const EXT: [ExtFilter; 1] = [ExtFilter {
//!     slot: ExtendedFilterSlot::_0,
//!     id1: 0x0CA,
//!     id2: None,
//! }];
//!
//! static INCOMING: Channel<ThreadModeRawMutex, Frame, 64> = Channel::new();
//! static OUTGOING: Channel<ThreadModeRawMutex, Frame, 256> = Channel::new();
//!
//! let (tx, rx, _props) = NerCan::init(configurator)
//!     .with_standard_filters(&STD)
//!     .with_extended_filters(&EXT)
//!     .start();
//!
//! spawner.spawn(can_tx(tx, OUTGOING.dyn_receiver(), OUTGOING.dyn_sender())
//!     .expect("Failed to spawn can_handler::can_tx()."));
//! spawner.spawn(can_rx(rx, INCOMING.dyn_sender())
//!     .expect("Failed to spawn can_handler::can_rx()."));
//! ```
#![no_std]
use defmt::warn;
use embassy_stm32::can::filter::FilterType::{DedicatedDual, DedicatedSingle};
use embassy_stm32::can::filter::{
    Action, EXTENDED_FILTER_MAX, ExtendedFilter, ExtendedFilterSlot, STANDARD_FILTER_MAX,
    StandardFilter, StandardFilterSlot,
};
use embassy_stm32::can::{CanConfigurator, CanRx, CanTx, Frame, Properties};
use embassy_sync::channel::{DynamicReceiver, DynamicSender, TrySendError};
use embedded_can::{ExtendedId, StandardId};

use heapless::Vec;

/// Number of standard filter elements in FDCAN message RAM.
const STANDARD_FILTER_SLOTS: usize = STANDARD_FILTER_MAX as usize;
/// Number of extended filter elements in FDCAN message RAM.
const EXTENDED_FILTER_SLOTS: usize = EXTENDED_FILTER_MAX as usize;

/// A standard (11-bit) CAN filter, declared independently of the peripheral.
///
/// `id2` selects the filter type: `Some` gives a dedicated-dual filter matching
/// both ids, `None` gives a dedicated-single filter matching only `id1`.
#[derive(Clone, Copy)]
pub struct StdFilter {
    /// Message RAM slot this filter occupies.
    pub slot: StandardFilterSlot,
    /// First id to match.
    pub id1: u16,
    /// Optional second id to match.
    pub id2: Option<u16>,
}

/// An extended (29-bit) CAN filter, declared independently of the peripheral.
///
/// `id2` selects the filter type: `Some` gives a dedicated-dual filter matching
/// both ids, `None` gives a dedicated-single filter matching only `id1`.
#[derive(Clone, Copy)]
pub struct ExtFilter {
    /// Message RAM slot this filter occupies.
    pub slot: ExtendedFilterSlot,
    /// First id to match.
    pub id1: u32,
    /// Optional second id to match.
    pub id2: Option<u32>,
}

pub struct NerCan {
    pub can_configurator: CanConfigurator<'static>,
    used_std_slots: Vec<StandardFilterSlot, STANDARD_FILTER_SLOTS>,
    used_ext_slots: Vec<ExtendedFilterSlot, EXTENDED_FILTER_SLOTS>,
}

impl NerCan {
    /// This is the CAN configuration to be used by most NER Projects.
    /// This is for optional use to pass into the can tasks to facilitate initialization
    ///
    /// The configuration sets:
    /// - Automatic bus-off recovery enabled.
    /// - Automatic retransmission disabled.
    /// - Classical CAN framing only (no CAN FD).
    /// - A clock divider of 1 and the data bit timing required for 500 kbit/s.
    /// - Transmit pause enabled.
    /// - A global filter that rejects all frames by default.
    ///
    /// ** It is expected that the user configures the CAN Std and Extended Filters before calling [`NerCan::start`]
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

    /// Applies a whole table of standard filters, in order.
    ///
    /// Lets a project declare its filter set as a `const` array next to its CAN
    /// ids instead of configuring slots one at a time.
    /// NOTE: will panic if any slot is already in use
    pub fn with_standard_filters(mut self, filters: &[StdFilter]) -> Self {
        for filter in filters {
            if self.used_std_slots.contains(&filter.slot) {
                panic!("The selected CAN Standard Filter Slot is already in use.");
            }

            let mut std = StandardFilter::default();
            match filter.id2 {
                Some(id2) => {
                    std.filter = DedicatedDual(
                        StandardId::new(filter.id1).unwrap(),
                        StandardId::new(id2).unwrap(),
                    );
                }
                None => {
                    std.filter = DedicatedSingle(StandardId::new(filter.id1).unwrap());
                }
            }
            std.action = Action::StoreInFifo0;
            self.can_configurator
                .properties()
                .set_standard_filter(filter.slot, std);
            // Cannot overflow: the Vec is sized to the number of hardware slots
            // and the duplicate check above rejects an already-recorded slot.
            let _ = self.used_std_slots.push(filter.slot);
        }

        self
    }

    /// Applies a whole table of extended filters, in order.
    ///
    /// Lets a project declare its filter set as a `const` array next to its CAN
    /// ids instead of configuring slots one at a time.
    /// NOTE: will panic if any slot is already in use
    pub fn with_extended_filters(mut self, filters: &[ExtFilter]) -> Self {
        for filter in filters {
            if self.used_ext_slots.contains(&filter.slot) {
                panic!("The selected CAN Extended Filter Slot is already in use.");
            }

            let mut ext = ExtendedFilter::default();
            match filter.id2 {
                Some(id2) => {
                    ext.filter = DedicatedDual(
                        ExtendedId::new(filter.id1).unwrap(),
                        ExtendedId::new(id2).unwrap(),
                    );
                }
                None => {
                    ext.filter = DedicatedSingle(ExtendedId::new(filter.id1).unwrap());
                }
            }
            ext.action = Action::StoreInFifo0;
            self.can_configurator
                .properties()
                .set_extended_filter(filter.slot, ext);
            // Cannot overflow: see `with_standard_filters`.
            let _ = self.used_ext_slots.push(filter.slot);
        }

        self
    }

    /// Starts up CAN in normal mode and returns the split objects.
    ///
    /// Call this only once every filter is in place: filters are written through
    /// the configurator, which this consumes.
    pub fn start(self) -> (CanTx<'static>, CanRx<'static>, Properties) {
        self.can_configurator.into_normal_mode().split()
    }
}

/// Add a frame to an outgoing CAN channel, waiting for room if the channel is full.
pub async fn send(out: DynamicSender<'_, Frame>, frame: Frame) {
    match out.try_send(frame) {
        Ok(()) => {}
        Err(TrySendError::Full(frame)) => {
            warn!(
                "Tried to add a frame to the OUTGOING Channel, but the Channel was full. This is not a failure, because we will .await until the Channel is able to accept the frame. However, consider increasing the capacity of the Channel if this is occurring often."
            );
            out.send(frame).await
        }
    }
}

/// Tries to add a frame to an outgoing CAN channel.
///
/// Returns `Err` if the channel is full, in which case the frame is dropped.
pub fn try_send(out: DynamicSender<'_, Frame>, frame: Frame) -> Result<(), ()> {
    match out.try_send(frame) {
        Ok(()) => Ok(()),
        Err(_) => {
            warn!(
                "Tried to add a frame to the OUTGOING Channel, but the Channel was full. Consider increasing the capacity of the Channel if this is occurring often."
            );
            Err(())
        }
    }
}

/// Drains the outgoing channel onto the bus.
///
/// - `outgoing_rx` supplies frames queued by other tasks for transmission.
/// - `outgoing_tx` is the send half of that same channel, used to requeue a
///   frame that the hardware evicted from a mailbox so it is not lost.
#[embassy_executor::task]
pub async fn can_tx(
    mut tx: CanTx<'static>,
    outgoing_rx: DynamicReceiver<'static, Frame>,
    outgoing_tx: DynamicSender<'static, Frame>,
) -> ! {
    loop {
        let frame = outgoing_rx.receive().await;

        // `write` returns a lower-priority frame if it had to bump one out of a
        // mailbox to make room. Put it back on the queue rather than dropping it.
        if let Some(evicted) = tx.write(&frame).await {
            send(outgoing_tx, evicted).await;
        }
    }
}

/// Passes frames received off the bus to the incoming channel.
///
/// - `incoming_tx` passes on CAN frames received from the bus so they can be
///   parsed by the user program.
///
/// **The channel behind `incoming_tx` is not intended to be the one `can_tx` drains.
#[embassy_executor::task]
pub async fn can_rx(mut rx: CanRx<'static>, incoming_tx: DynamicSender<'static, Frame>) -> ! {
    loop {
        match rx.read().await {
            Ok(envelope) => incoming_tx.send(envelope.frame).await,
            Err(err) => warn!("Bus error! {}", err),
        }
    }
}
