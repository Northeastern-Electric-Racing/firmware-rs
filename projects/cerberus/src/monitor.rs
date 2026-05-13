use core::sync::atomic::AtomicBool;

use bitfield::{Bit, BitMut};
use defmt::unwrap;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_futures::select::{self, select3, select_array};
use embassy_stm32::{
    adc::RingBufferedAdc,
    can::{Frame, StandardId},
    exti::ExtiInput,
    mode::Async,
    peripherals::ADC1,
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Receiver, Sender},
};
use embassy_time::{Duration, Instant, Ticker, Timer};
use pca9539_ner::{Pca9539, Pin};

use crate::{PduCommand, SharedI2c};

const LV_SENSE_MSG_ID: StandardId = StandardId::new(0x503).expect("Cannot parse ID");
const LV_SENSE_REFRESH_TIME: Duration = Duration::from_millis(750);

#[embassy_executor::task]
/// Read and send LV sense
pub async fn lv_sense_handler(
    mut adc1: RingBufferedAdc<'static, ADC1>,
    can_send: Sender<'static, ThreadModeRawMutex, Frame, 25>,
) {
    let mut measurements: [u16; 20] = [0u16; 40 / 2];

    loop {
        adc1.read_latest(&mut measurements);
        // 8.97 is mahic no. bc nobody remembers the exact resistor config
        let v_in = (measurements[0] as f32 * 8.967 * 10f32) as u32;
        // TODO transform measurements
        can_send
            .send(unwrap!(Frame::new_data(
                LV_SENSE_MSG_ID,
                &v_in.to_be_bytes()
            )))
            .await;

        Timer::after(LV_SENSE_REFRESH_TIME).await;
    }
}

const RTDS_SOUND_TIME: Duration = Duration::from_millis(1750);
const CTRL_EXPANDER_I2C_ADDR: u8 = 0x76;

/// also queries RTDS state, so RTDS_SOUND_TIME accuracy is +/- TSMS_REFRESH_TIME
const TSMS_REFRESH_TIME: Duration = Duration::from_millis(100);
const FUSE_REFRESH_TIME: Duration = Duration::from_millis(800);

#[embassy_executor::task]
/// Controls all ctrl expander functionality
/// Can be commanded via the pdu channel, will also send CAN msgs for fuses and update TSMS state
pub async fn ctrl_expander_handler(
    can_send: Sender<'static, ThreadModeRawMutex, Frame, 25>,
    pdu_recv: Receiver<'static, ThreadModeRawMutex, PduCommand, 10>,
    ctrl_expand_i2c: &'static SharedI2c,
    ts_state_send: &'static AtomicBool,
) {
    let i2c_dev = I2cDevice::new(ctrl_expand_i2c);
    let mut pca9539 = Pca9539::new(i2c_dev, CTRL_EXPANDER_I2C_ADDR).unwrap();

    // initial setup
    unwrap!(
        pca9539
            .write_register(
                pca9539_ner::RegisterType::OutputLevel,
                pca9539_ner::Bank::Bank0,
                0b00000010
            )
            .await
    );
    unwrap!(
        pca9539
            .write_register(
                pca9539_ner::RegisterType::OutputLevel,
                pca9539_ner::Bank::Bank1,
                0b00000010
            )
            .await
    );
    unwrap!(
        pca9539
            .write_register(
                pca9539_ner::RegisterType::Direction,
                pca9539_ner::Bank::Bank0,
                0b11110000
            )
            .await
    );
    unwrap!(
        pca9539
            .write_register(
                pca9539_ner::RegisterType::Direction,
                pca9539_ner::Bank::Bank1,
                0b01111111
            )
            .await
    );

    // debounce TSMS sense
    let mut tsms_prev_state = false;
    let mut tsms_state_count = 0u8;

    // for rtds
    let mut active_sounding = false;
    let mut rtds_sound_start = Instant::now();

    let mut tsms_ticker = Ticker::every(TSMS_REFRESH_TIME);
    let mut fuse_ticker = Ticker::every(FUSE_REFRESH_TIME);

    loop {
        match select3(tsms_ticker.next(), fuse_ticker.next(), pdu_recv.receive()).await {
            select::Either3::First(_) => {
                // end RTDS sound if time is up
                if active_sounding && Instant::now() - rtds_sound_start > RTDS_SOUND_TIME {
                    unwrap!(
                        pca9539
                            .write_pin(
                                pca9539_ner::RegisterType::OutputLevel,
                                pca9539_ner::Bank::Bank1,
                                Pin::P07,
                                false
                            )
                            .await
                    );
                    active_sounding = false;
                }
                let state = unwrap!(
                    pca9539
                        .read_pin(
                            pca9539_ner::RegisterType::InputLevel,
                            pca9539_ner::Bank::Bank1,
                            Pin::P06
                        )
                        .await
                );
                if state == tsms_prev_state {
                    if tsms_state_count > 5 {
                        ts_state_send.store(state, core::sync::atomic::Ordering::Release);
                        tsms_state_count = 0;
                    }
                    tsms_state_count += 1;
                }
                tsms_prev_state = state;
            }
            select::Either3::Second(_) => {
                let data_0 = unwrap!(
                    pca9539
                        .read_register(
                            pca9539_ner::RegisterType::InputLevel,
                            pca9539_ner::Bank::Bank0
                        )
                        .await
                );

                let data_1 = unwrap!(
                    pca9539
                        .read_register(
                            pca9539_ner::RegisterType::InputLevel,
                            pca9539_ner::Bank::Bank1
                        )
                        .await
                );

                let mut send_data_1: u8 = 0;
                let mut send_data_2: u8 = 0;

                send_data_1.set_bit(0, data_0.bit(4));
                send_data_1.set_bit(1, data_0.bit(5));
                send_data_1.set_bit(2, data_0.bit(6));
                send_data_1.set_bit(3, data_0.bit(7));
                send_data_1.set_bit(4, data_1.bit(0));
                send_data_1.set_bit(5, data_1.bit(1));
                send_data_1.set_bit(6, data_1.bit(2));
                send_data_1.set_bit(7, data_1.bit(3));
                send_data_2.set_bit(0, data_1.bit(4));

                // reverse bits
                send_data_1 = send_data_1.reverse_bits();
                send_data_2 = send_data_2.reverse_bits();

                let mut send_data_bits: [u8; 2] = [0u8; 2];

                send_data_bits[0..1].copy_from_slice(&send_data_1.to_be_bytes());
                send_data_bits[1..2].copy_from_slice(&send_data_2.to_be_bytes());

                can_send
                    .send(unwrap!(Frame::new_data(
                        unwrap!(StandardId::new(0x111)),
                        &send_data_bits
                    )))
                    .await;
            }
            select::Either3::Third(cmd) => match cmd {
                PduCommand::WritePump(state) => {
                    unwrap!(
                        pca9539
                            .write_pin(
                                pca9539_ner::RegisterType::OutputLevel,
                                pca9539_ner::Bank::Bank0,
                                Pin::P00,
                                state
                            )
                            .await
                    )
                }
                PduCommand::WriteBrakelight(state) => unwrap!(
                    pca9539
                        .write_pin(
                            pca9539_ner::RegisterType::OutputLevel,
                            pca9539_ner::Bank::Bank0,
                            Pin::P02,
                            state
                        )
                        .await
                ),
                PduCommand::WriteFault(state) => unwrap!(
                    pca9539
                        .write_pin(
                            pca9539_ner::RegisterType::OutputLevel,
                            pca9539_ner::Bank::Bank0,
                            Pin::P01,
                            state
                        )
                        .await
                ),
                PduCommand::SoundRtds => {
                    active_sounding = true;
                    rtds_sound_start = Instant::now();
                }
            },
        }
    }
}

#[embassy_executor::task]
pub async fn steeringio_handler(
    _can_send: Sender<'static, ThreadModeRawMutex, Frame, 25>,
    mut button1: ExtiInput<'static, Async>,
    mut button2: ExtiInput<'static, Async>,
    mut button3: ExtiInput<'static, Async>,
    mut button4: ExtiInput<'static, Async>,
    // mut button5: ExtiInput<'static>,
    // mut button6: ExtiInput<'static>,
    mut button7: ExtiInput<'static, Async>,
    mut button8: ExtiInput<'static, Async>,
) {
    loop {
        let ans = select_array([
            button1.wait_for_falling_edge(),
            button2.wait_for_falling_edge(),
            button3.wait_for_falling_edge(),
            button4.wait_for_falling_edge(),
            // button5.wait_for_falling_edge(),
            // button6.wait_for_falling_edge(),
            button7.wait_for_falling_edge(),
            button8.wait_for_falling_edge(),
        ])
        .await;

        let _button_value = match ans.1 {
            0 => button1.get_level(),
            1 => button2.get_level(),
            2 => button3.get_level(),
            3 => button4.get_level(),
            // 4 => button5.get_level(),
            // 5 => button6.get_level(),
            4 => button7.get_level(),
            5 => button8.get_level(),
            _ => panic!("Wtf"),
        };
        // TODO map buttons to functions
    }
}
