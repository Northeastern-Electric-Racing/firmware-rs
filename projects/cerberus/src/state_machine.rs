use core::sync::atomic::{AtomicBool, AtomicI32};

use defmt::warn;
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex},
    channel::Sender,
    signal::Signal,
};

use crate::{FunctionalType, NeroType, PduCommand, StateTransition};

#[embassy_executor::task]
/// Handles the state (via PDU outputs) using a variety of inputs
pub async fn state_handler(
    state_recv: &'static Signal<CriticalSectionRawMutex, StateTransition>,
    pdu_cmd_send: Sender<'static, ThreadModeRawMutex, PduCommand, 10>,
    speed: &'static AtomicI32,
    brake_state: &'static AtomicBool,
    tsms_status: &'static AtomicBool,
) {
    let mut prev_func_state = FunctionalType::READY;
    //let mut prev_nero_state = NeroType::OFF;

    loop {
        let new_state = state_recv.wait().await;
        match new_state {
            StateTransition::Functional(state) => match state {
                crate::FunctionalType::READY => {
                    if speed.load(core::sync::atomic::Ordering::Acquire) > 1 {
                        warn!("Cannot move to ready, moving!");
                        continue;
                    }

                    pdu_cmd_send.send(PduCommand::WritePump(false)).await;
                    pdu_cmd_send.send(PduCommand::WriteFault(true)).await;
                }
                crate::FunctionalType::FPit
                | crate::FunctionalType::FPerformance
                | crate::FunctionalType::FEfficiency => {
                    if prev_func_state != FunctionalType::REVERSE {
                        if speed.load(core::sync::atomic::Ordering::Acquire) > 1 {
                            warn!("Cannot move to active, moving!");
                            continue;
                        }
                        if !brake_state.load(core::sync::atomic::Ordering::Acquire)
                            || !tsms_status.load(core::sync::atomic::Ordering::Acquire)
                        {
                            warn!("Cannot move to active, no brake or TS!");
                            continue;
                        }
                        pdu_cmd_send.send(PduCommand::SoundRtds).await;
                    }

                    pdu_cmd_send.send(PduCommand::WritePump(true)).await;
                    pdu_cmd_send.send(PduCommand::WriteFault(true)).await;
                }
                crate::FunctionalType::REVERSE => {
                    if prev_func_state != FunctionalType::FPit {
                        warn!("Cannot move to reverse out of pit!");
                        continue;
                    }
                }
                crate::FunctionalType::FAULTED => {
                    pdu_cmd_send.send(PduCommand::WritePump(false)).await;
                    pdu_cmd_send.send(PduCommand::WriteFault(false)).await;
                    state_recv.signal(StateTransition::Nero(NeroType::OFF))
                }
            },
            StateTransition::Nero(state) => match state {
                crate::NeroType::OFF => todo!(),
                crate::NeroType::PIT => todo!(),
                crate::NeroType::PERFORMANCE => todo!(),
                crate::NeroType::EFFICIENCY => todo!(),
                crate::NeroType::DEBUG => todo!(),
                crate::NeroType::CONFIGURATION => todo!(),
                crate::NeroType::FlappyBird => todo!(),
                crate::NeroType::EXIT => todo!(),
            },
        }

        match new_state {
            StateTransition::Functional(f) => prev_func_state = f,
            StateTransition::Nero(_) => (), //prev_nero_state = n,
        }
    }
}
