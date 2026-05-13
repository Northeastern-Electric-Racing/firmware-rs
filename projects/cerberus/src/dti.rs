use core::{f32::consts::PI, sync::atomic::AtomicI32};

use embassy_stm32::can::Frame;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use crate::can_handler::DTI_RPM_MSG_ID;

const TIRE_DIAMETER: f32 = 16.0;
const GEAR_RATIO: f32 = 47.0 / 13.0;
const POLE_PAIRS: i32 = 10;

#[embassy_executor::task]
/// Receives rpm from can handler, then computes and sends mph
pub async fn dti_handler(
    rpm_recv: &'static Signal<CriticalSectionRawMutex, Frame>,
    speed: &'static AtomicI32,
) {
    loop {
        let rpm_frame = rpm_recv.wait().await;
        match rpm_frame.id() {
            embassy_stm32::can::Id::Standard(id) => if id == &DTI_RPM_MSG_ID {
                // TODO fat chance this works
                let erpm = ((rpm_frame.data()[0] as i32) << 24u32)
                    + ((rpm_frame.data()[1] as i32) << 16)
                    + ((rpm_frame.data()[2] as i32) << 8u32)
                    + (rpm_frame.data()[3] as i32);
                let mph = (erpm / POLE_PAIRS) as f32 / GEAR_RATIO
                    * 60.0
                    * (TIRE_DIAMETER / 63360.0)
                    * PI;
                // TODO add precision
                speed.store(mph as i32, core::sync::atomic::Ordering::Release);
            },
            embassy_stm32::can::Id::Extended(_) => (),
        }
    }
}
