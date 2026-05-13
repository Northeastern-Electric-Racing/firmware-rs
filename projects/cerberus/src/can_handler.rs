use defmt::{trace, unwrap, warn};
use embassy_futures::select;
use embassy_futures::select::select;
use embassy_stm32::can::{
    filter::{BankConfig, ListEntry16},
    Can, Frame, StandardId,
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex},
    channel::Receiver,
    signal::Signal,
};

const CAN_BITRATE: u32 = 500_000;

pub const DTI_RPM_MSG_ID: StandardId = StandardId::new(0x416).expect("Cannot parse ID");
const BMS_DCL_MSG_ID: StandardId = StandardId::new(0x156).expect("Cannot parse ID");

#[embassy_executor::task]
/// Handles CAN, giving messages to DTI and BMS as they match via ID
/// Adds filters as appropriate
pub async fn can_handler(
    mut can: Can<'static>,
    bms_callback: &'static Signal<CriticalSectionRawMutex, Frame>,
    dti_callback: &'static Signal<CriticalSectionRawMutex, Frame>,
    recv: Receiver<'static, ThreadModeRawMutex, Frame, 25>,
) {
    can.set_bitrate(CAN_BITRATE);
    can.modify_filters().enable_bank(
        0,
        embassy_stm32::can::Fifo::Fifo0,
        BankConfig::List16([
            ListEntry16::data_frames_with_id(BMS_DCL_MSG_ID),
            ListEntry16::data_frames_with_id(DTI_RPM_MSG_ID),
            ListEntry16::data_frames_with_id(unwrap!(StandardId::new(0x1))), // TODO needed?
            ListEntry16::data_frames_with_id(unwrap!(StandardId::new(0x2))),
        ]),
    );
    can.enable().await;

    loop {
        match select(recv.receive(), can.read()).await {
            select::Either::First(frame) => {
                trace!("Sending frame: {}", frame);
                // trying to figure out how to detect overflow
                if can.write(&frame).await.dequeued_frame().is_some() {
                    warn!("Dequeing can frames!");
                }
            }
            select::Either::Second(res) => match res {
                Ok(can_recv) => match can_recv.frame.id() {
                    embassy_stm32::can::Id::Standard(id) => match *id {
                        DTI_RPM_MSG_ID => dti_callback.signal(can_recv.frame),
                        BMS_DCL_MSG_ID => bms_callback.signal(can_recv.frame),
                        _ => warn!("Ignored message of id {}", id.as_raw()),
                    },
                    embassy_stm32::can::Id::Extended(id) => {
                        warn!("Ignored message of ext. id {}", id.as_raw())
                    }
                },
                Err(err) => warn!("Bus error! {}", err),
            },
        }
    }
}
