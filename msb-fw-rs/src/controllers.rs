use embassy_stm32::gpio::Output;
use embassy_time::{Duration, Timer};

use crate::DeviceLocation;

const LED_REFRESH_TIME: Duration = Duration::from_millis(250);

#[embassy_executor::task]
pub async fn control_leds(
    mut led1: Output<'static>,
    mut led2: Output<'static>,
    device_loc: DeviceLocation,
) {
    let mut i = 0u8;
    loop {
        if i.is_multiple_of(8) {
            match device_loc {
                DeviceLocation::FrontLeft => {
                    led1.set_low();
                    led2.set_low();
                }
                DeviceLocation::FrontRight => {
                    led1.set_low();
                    led2.set_high();
                }
                DeviceLocation::BackLeft => {
                    led1.set_high();
                    led2.set_low();
                }
                DeviceLocation::BackRight => {
                    led1.set_high();
                    led2.set_high();
                }
            }
        } else {
            match device_loc {
                DeviceLocation::FrontLeft => {
                    led1.set_high();
                    led2.set_high();
                }
                DeviceLocation::FrontRight => {
                    led1.set_high();
                    led2.set_low();
                }
                DeviceLocation::BackLeft => {
                    led1.set_low();
                    led2.set_high();
                }
                DeviceLocation::BackRight => {
                    led1.set_low();
                    led2.set_low();
                }
            }
        }
        i = i.wrapping_add(1);
        Timer::after(LED_REFRESH_TIME).await;
    }
}
