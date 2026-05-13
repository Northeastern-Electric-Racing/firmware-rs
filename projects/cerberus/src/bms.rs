use embassy_futures::select::select;
use embassy_stm32::can::Frame;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

use crate::FaultCode;

/// time from no BMS DCL message until Cerberus will fault
const BMS_WATCHDOG_FAULT_TIME: u64 = 4;

#[embassy_executor::task]
/// triggers a fault if a BMS message is not received for 4 seconds
/// Takes in the BMS signal and sends out a fault signal
pub async fn bms_handler(
    bms_recv: &'static Signal<CriticalSectionRawMutex, Frame>,
    fault_send: &'static Signal<CriticalSectionRawMutex, FaultCode>,
) {
    loop {
        match select(bms_recv.wait(), Timer::after_secs(BMS_WATCHDOG_FAULT_TIME)).await {
            embassy_futures::select::Either::First(_) => continue,
            embassy_futures::select::Either::Second(_) => {
                fault_send.signal(FaultCode::BmsCanMonitorFault)
            }
        }
    }
}
