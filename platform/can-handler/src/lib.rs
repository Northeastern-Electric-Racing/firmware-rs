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
//!
//! # `defmt-monitor` feature
//!
//! Enabling the `defmt-monitor` feature publishes TX/RX counters from [`can_tx`]
//! and [`can_rx`] through `defmt_monitor::monitor!()` under the `CanDebug/`
//! topic, and adds a `can_props` task that samples FDCAN health registers:
//!
//! ```ignore
//! spawner.spawn(can_props(embassy_stm32::pac::FDCAN2)
//!     .expect("Failed to spawn can_handler::can_props()."));
//! ```
//!
//! With the feature off, none of this is compiled in. With it on, the calls can
//! still be compiled out by building with `DEFMT_MONITOR=off`.
#![no_std]
use defmt::warn;
use embassy_futures::select::{Either, select};
use embassy_stm32::can::filter::FilterType::{DedicatedDual, DedicatedSingle};
use embassy_stm32::can::filter::{
    Action, EXTENDED_FILTER_MAX, ExtendedFilter, ExtendedFilterSlot, STANDARD_FILTER_MAX,
    StandardFilter, StandardFilterSlot,
};
use embassy_stm32::can::{CanConfigurator, CanRx, CanTx, Frame, Properties};
use embassy_sync::channel::{DynamicReceiver, DynamicSender, TrySendError};
use embassy_time::Timer;
use embedded_can::{ExtendedId, StandardId};

use heapless::Vec;

/// Forwards to `defmt_monitor::monitor!()` when the `defmt-monitor` feature is
/// enabled. Otherwise it expands to nothing but still borrows its arguments, so
/// counters that only exist to be monitored don't trip unused warnings.
#[cfg(feature = "defmt-monitor")]
macro_rules! monitor {
    ($($tt:tt)*) => { defmt_monitor::monitor!($($tt)*) };
}
#[cfg(not(feature = "defmt-monitor"))]
macro_rules! monitor {
    ($topic:expr, desc = $desc:literal, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        $( let _ = &$arg; )*
    }};
}

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
    let mut send_count: u32 = 0;
    let mut dropped_due_to_outgoing_full_count: u32 = 0;
    let mut dropped_due_to_stalled_tx_count: u32 = 0;

    loop {
        let frame = outgoing_rx.receive().await;

        match select(tx.write(&frame), Timer::after_millis(50)).await {
            // Case: frame was dropped
            Either::First(Some(dropped)) => {
                // If we dropped a frame, try to send it back to OUTGOING so it can get sent again.
                // We can't do a normal `send().await` since this task is the one that drains OUTGOING, so doing
                // that could probably cause a deadlock somehow.
                match outgoing_tx.try_send(dropped) {
                    Ok(_) => (),
                    Err(_) => {
                        dropped_due_to_outgoing_full_count += 1;
                        warn!(
                            "Had to drop an outgoing CAN frame because OUTGOING was full! Not good."
                        );
                    }
                }
            }

            // Case: frame was sent successfully
            Either::First(None) => send_count += 1,

            // Case: The Timer::after await returned before tx.write(), so CAN TX has stalled and we drop the frame.
            // the "stall" shouldn't be a permanant thing, we just need to make sure this task can't sleep forever.
            Either::Second(_) => {
                dropped_due_to_stalled_tx_count += 1;
                warn!(
                    "Had to drop an outgoing CAN frame because CAN TX stalled! Probably not good."
                );
            }
        }

        monitor!(
            "CanDebug/send_count",
            desc = "Send count",
            "{=u32}",
            send_count
        );
        monitor!(
            "CanDebug/dropped_due_to_outgoing_full_count",
            desc = "Frames dropped due to the OUTGOING channel being full.",
            "{=u32}",
            dropped_due_to_outgoing_full_count
        );
        monitor!(
            "CanDebug/dropped_due_to_stalled_tx_count",
            desc = "Frames dropped due to TX stalling.",
            "{=u32}",
            dropped_due_to_stalled_tx_count
        );
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
    let mut rx_count: u32 = 0;
    let mut rx_err_count: u32 = 0;

    loop {
        match rx.read().await {
            Ok(envelope) => {
                incoming_tx.send(envelope.frame).await;
                rx_count += 1;
            }
            Err(err) => {
                warn!("Bus error! {}", err);
                rx_err_count += 1;
            }
        }

        monitor!("CanDebug/rx_count", desc = "RX count", "{=u32}", rx_count);
        monitor!(
            "CanDebug/rx_err_count",
            desc = "RX err count",
            "{=u32}",
            rx_err_count
        );
    }
}

/// Publishes FDCAN health diagnostics read straight from the peripheral registers.
///
/// - `regs` is the FDCAN instance the handler was started on, e.g.
///   `embassy_stm32::pac::FDCAN2`.
///
/// Samples every 500 ms. Only available with the `defmt-monitor` feature.
#[cfg(feature = "defmt-monitor")]
#[embassy_executor::task]
pub async fn can_props(regs: embassy_stm32::pac::can::Fdcan) -> ! {
    use embassy_stm32::can::enums::BusErrorMode;

    /// Number of hardware TX mailboxes on STM32H563.
    const TX_MAILBOX_COUNT: usize = 3;
    /// How often this task should run, in ms.
    const PROPS_SAMPLE_PERIOD_MS: u64 = 500;

    loop {
        // Readings from CAN registers.
        let psr = regs.psr().read();
        let ecr = regs.ecr().read();
        let cccr = regs.cccr().read();
        let ir = regs.ir().read();
        let ie = regs.ie().read();
        let ils = regs.ils().read();
        let ile = regs.ile().read();
        let txfqs = regs.txfqs().read();
        let txbrp = regs.txbrp().read();
        let txbto = regs.txbto().read();
        let txbcf = regs.txbcf().read();

        // Find error mode. This is what embassy does internally (at least as of writing this).
        let bus_error_mode = match (psr.bo(), psr.ep()) {
            (false, false) => BusErrorMode::ErrorActive,
            (false, true) => BusErrorMode::ErrorPassive,
            (true, _) => BusErrorMode::BusOff,
        };

        // One bit per hardware TX mailbox.
        let mut pending_mask = 0_u8;
        let mut occurred_mask = 0_u8;
        let mut cancelled_mask = 0_u8;
        let mut pending_count = 0_u8;
        for i in 0..TX_MAILBOX_COUNT {
            if txbrp.trp(i) {
                pending_mask |= 1_u8 << i;
                pending_count += 1_u8;
            }
            if txbto.to(i) {
                occurred_mask |= 1_u8 << i;
            }
            if txbcf.cf(i) {
                cancelled_mask |= 1_u8 << i;
            }
        }

        // Error counters and protocol status.
        monitor!(
            "CanDebug/tx_error_count",
            desc = "FDCAN TEC (ECR.TEC). Climbs by 8 per failed transmission. >255 means bus-off.",
            "{=u8}",
            ecr.tec()
        );
        monitor!(
            "CanDebug/rx_error_count",
            desc = "FDCAN REC (ECR.REC).",
            "{=u8}",
            ecr.rec()
        );
        monitor!(
            "CanDebug/bus_error_mode",
            desc = "FDCAN bus error state, from PSR.BO/PSR.EP.",
            "{}",
            bus_error_mode
        );
        monitor!(
            "CanDebug/error_warning",
            desc = "PSR.EW. An error counter has passed 96.",
            "{=bool}",
            psr.ew()
        );
        monitor!(
            "CanDebug/node_activity",
            desc = "PSR.ACT. SYNC=still synchronizing to the bus, IDLE=neither sending nor receiving, RX/TX=actively on the bus.",
            "{}",
            psr.act()
        );

        // TX mailbox occupancy.
        monitor!(
            "CanDebug/tx_pending_mask",
            desc = "TXBRP, one bit per mailbox. Set bit means a transmission is requested and not yet finished.",
            "{=u8}",
            pending_mask
        );
        monitor!(
            "CanDebug/tx_pending_count",
            desc = "Number of TX mailboxes with a pending request, 0 to 3.",
            "{=u8}",
            pending_count
        );
        monitor!(
            "CanDebug/tx_occurred_mask",
            desc = "TXBTO, one bit per mailbox. Set bit means a frame was successfully transmitted. Stays 0 if nothing has ever reached the bus.",
            "{=u8}",
            occurred_mask
        );
        monitor!(
            "CanDebug/tx_cancelled_mask",
            desc = "TXBCF, one bit per mailbox. Set bit means a transmission was cancelled.",
            "{=u8}",
            cancelled_mask
        );
        monitor!(
            "CanDebug/tx_fifo_full",
            desc = "TXFQS.TFQF. No free mailbox.",
            "{=bool}",
            txfqs.tfqf()
        );
        monitor!(
            "CanDebug/tx_fifo_free_level",
            desc = "TXFQS.TFFL. Number of consecutive free mailboxes. Reads 0 in queue mode (TXBC.TFQM=1).",
            "{=u8}",
            txfqs.tffl()
        );
        monitor!(
            "CanDebug/tx_put_index",
            desc = "TXFQS.TFQPI. The mailbox the next write goes into.",
            "{=u8}",
            txfqs.tfqpi()
        );

        // Interrupts.
        monitor!(
            "CanDebug/ir_tc_latched",
            desc = "IR.TC still set at sample time. Embassy's ISR clears this on entry, so persistently true means the ISR is not running.",
            "{=bool}",
            ir.tc()
        );
        monitor!(
            "CanDebug/ir",
            desc = "Raw FDCAN IR, all latched interrupt flags.",
            "{=u32}",
            ir.0
        );
        monitor!(
            "CanDebug/ie",
            desc = "Raw FDCAN IE, enabled interrupt sources.",
            "{=u32}",
            ie.0
        );
        monitor!(
            "CanDebug/ils",
            desc = "Raw FDCAN ILS, interrupt line select.",
            "{=u32}",
            ils.0
        );
        monitor!(
            "CanDebug/ile",
            desc = "Raw FDCAN ILE, interrupt line enable.",
            "{=u32}",
            ile.0
        );

        // Operating mode.
        monitor!(
            "CanDebug/cccr_init",
            desc = "CCCR.INIT. True means the peripheral is held out of bus traffic, which hardware does on bus-off.",
            "{=bool}",
            cccr.init()
        );
        monitor!(
            "CanDebug/cccr_dar",
            desc = "CCCR.DAR. True means automatic retransmission is disabled, so a failed frame is discarded after one attempt.",
            "{=bool}",
            cccr.dar()
        );
        monitor!(
            "CanDebug/cccr_mon",
            desc = "CCCR.MON. Bus monitoring mode. True means we never drive the bus dominant.",
            "{=bool}",
            cccr.mon()
        );
        monitor!(
            "CanDebug/cccr_test",
            desc = "CCCR.TEST. True in loopback modes.",
            "{=bool}",
            cccr.test()
        );

        Timer::after_millis(PROPS_SAMPLE_PERIOD_MS).await;
    }
}
