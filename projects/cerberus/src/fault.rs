use defmt::{debug, unwrap, warn};
use embassy_futures::select::select3;
use embassy_stm32::can::{Frame, StandardId};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex},
    channel::Sender,
    signal::Signal,
};
use embassy_time::{Duration, Ticker};

use crate::{FaultCode, FunctionalType, StateTransition};

const STATUS_MSG_ID: StandardId = StandardId::new(0x502).expect("Cannot parse ID");

/// time at which to unfault the car if the faulting condition has cleared
const UNFAULT_TIME: Duration = Duration::from_secs(5);

/// refresh time for sending fault status message
const SEND_STATUS_MSG_TIME: Duration = Duration::from_millis(200);

#[embassy_executor::task]
/// Receives a fault, then sends it out via CAN and tells the state machine
/// Includes automatic unfaulting
pub async fn fault_handler(
    can_send: Sender<'static, ThreadModeRawMutex, Frame, 25>,
    fault: &'static Signal<CriticalSectionRawMutex, FaultCode>,
    state_send: &'static Signal<CriticalSectionRawMutex, StateTransition>,
) {
    let mut last_fault = FaultCode::FaultsClear;

    let mut fault_bits: [u8; 5] = [0u8; 5];

    let mut fault_cansend_ticker = Ticker::every(SEND_STATUS_MSG_TIME);

    let mut unfault_ticker = Ticker::every(UNFAULT_TIME);

    loop {
        // TODO figure out the expiry paradox here
        last_fault = match select3(
            fault.wait(),
            fault_cansend_ticker.next(),
            unfault_ticker.next(),
        )
        .await
        {
            embassy_futures::select::Either3::First(new_fault) => {
                match new_fault.get_severity() {
                    crate::FaultSeverity::Defcon1
                    | crate::FaultSeverity::Defcon2
                    | crate::FaultSeverity::Defcon3 => {
                        state_send.signal(StateTransition::Functional(FunctionalType::FAULTED));
                        // restart the countdown to unfault
                        unfault_ticker.reset();
                    }
                    crate::FaultSeverity::Defcon4 => warn!("Non critical fault!"),
                    crate::FaultSeverity::Defcon5 => debug!("Faults clear!"),
                }
                new_fault
            }
            embassy_futures::select::Either3::Second(_) =>
            // this is just to send a periodic CAN message
            {
                last_fault
            }
            embassy_futures::select::Either3::Third(_) => {
                // the countdown has been reached, unfault
                state_send.signal(StateTransition::Functional(FunctionalType::READY));
                FaultCode::FaultsClear
            }
        };

        fault_bits[3..4].copy_from_slice(&(last_fault.get_severity() as u8).to_be_bytes());
        fault_bits[0..3].copy_from_slice(&(last_fault as u32).to_be_bytes());

        can_send
            .send(unwrap!(Frame::new_data(STATUS_MSG_ID, &fault_bits)))
            .await;
    }
}
