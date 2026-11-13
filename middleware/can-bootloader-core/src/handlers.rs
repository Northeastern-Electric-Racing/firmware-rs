//! Implement host commands, packet acknowledgements, and staged-image activation.

use embassy_boot::BlockingFirmwareUpdater;
use embedded_storage::nor_flash::NorFlash;

use crate::flash;
use crate::image::{self, active_size};
use crate::link::Link;
use crate::protocol::*;
use crate::transport::{Frame, Transport};
use crate::types::{BootAction, Context, UpdateState};

// Request lengths include the opcode byte.
const WRITE_REQUEST_LEN: usize = 7;
const START_UPDATE_LEN: usize = 8;

// Remember the last committed block so a lost final ACK does not cause a rewrite.
#[derive(Clone, Copy)]
struct CompletedWrite {
    valid: bool,
    offset: u32,
    count: u32,
}

impl CompletedWrite {
    const fn new() -> Self {
        Self {
            valid: false,
            offset: 0,
            count: 0,
        }
    }
}

/// Keep retry bookkeeping across commands within one bootloader session.
pub struct Handlers {
    completed: CompletedWrite,
}

impl Handlers {
    /// Start with no completed block available for duplicate detection.
    pub const fn new() -> Self {
        Self {
            completed: CompletedWrite::new(),
        }
    }

    /// ACK the ping, then advertise the version and supported command opcodes.
    pub async fn get_info<T: Transport>(&mut self, link: &mut Link<'_, T>) {
        // Split the capability list across two classical CAN frames.
        let first = [
            GET_INFO,
            VERSION,
            COMMAND_COUNT,
            GET_INFO,
            GET_STATUS,
            START_APP,
            WRITE_MEMORY,
            ACTIVATE,
        ];
        let second = [GET_INFO, COMPUTE_CRC, START_UPDATE];

        if link.ack(GET_INFO).await {
            let _ = link.send(&first).await;
            let _ = link.send(&second).await;
        }
    }

    /// Return version and session state using the existing GET_STATUS wire format.
    pub async fn get_status<T: Transport>(&mut self, link: &mut Link<'_, T>, context: &Context) {
        let response = [GET_STATUS, VERSION, context.state.code()];
        if link.ack(GET_STATUS).await {
            let _ = link.send(&response).await;
        }
    }

    /// Accept image metadata before any DFU blocks are written.
    pub async fn start_update<T: Transport>(
        &mut self,
        frame: &Frame,
        link: &mut Link<'_, T>,
        context: &mut Context,
    ) {
        let data = frame.data();
        if data.len() != START_UPDATE_LEN {
            let _ = link.nack(START_UPDATE).await;
            return;
        }

        // Payload: opcode, 24-bit image length, then 32-bit CRC, all big-endian.
        let image_size = u24(&data[1..4]);
        let expected_crc = u32be(&data[4..8]);

        if image_size == 0 || image_size > active_size() {
            let _ = link.nack(START_UPDATE).await;
            return;
        }

        context.start(image_size, expected_crc); // A new transfer invalidates verification.
        self.completed = CompletedWrite::new();
        let _ = link.ack(START_UPDATE).await;
    }

    /// Receive sequenced packets, commit one DFU block, then ACK completion.
    pub async fn write_memory<T, DFU, STATE>(
        &mut self,
        frame: &Frame,
        link: &mut Link<'_, T>,
        updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
        context: &mut Context,
    ) where
        T: Transport,
        DFU: NorFlash,
        STATE: NorFlash,
    {
        let data = frame.data();
        if data.len() != WRITE_REQUEST_LEN || context.state == UpdateState::Idle {
            let _ = link.nack(WRITE_MEMORY).await;
            return;
        }

        // Payload: opcode, 32-bit DFU offset, then 16-bit byte count.
        let offset = u32be(&data[1..5]);
        let count = u16be(&data[5..7]) as u32;

        // A repeated completed request needs an ACK, not another flash write.
        if self.completed.valid && self.completed.offset == offset && self.completed.count == count
        {
            let _ = link.ack(WRITE_MEMORY).await;
            return;
        }

        if !write_valid(context, offset, count, DFU::WRITE_SIZE as u32) {
            let _ = link.nack(WRITE_MEMORY).await;
            return;
        }

        self.completed = CompletedWrite::new(); // Retain retry state for only one block.
        if !link.ack(WRITE_MEMORY).await {
            return;
        }

        let mut buffer = [0u8; MAX_WRITE]; // Stage the whole block before touching flash.
        let mut received = 0usize;
        let mut expected_sequence = 0u8;

        // Sequence numbers restart at zero for each block; each packet gets an ACK.
        while received < count as usize {
            let Some(frame) = link.receive_timeout(WRITE_TIMEOUT_MS).await else {
                let _ = link.nack(WRITE_MEMORY).await;
                return;
            };

            let id = frame.id();
            let payload = frame.data();

            if id == link.ids.data && payload.len() >= 2 {
                // Data frames carry one sequence byte and up to seven image bytes.
                let sequence = payload[0];
                let available = payload.len() - 1;
                let remaining = count as usize - received; // Reject packets past this block.

                if sequence == expected_sequence && available <= remaining {
                    buffer[received..received + available].copy_from_slice(&payload[1..]);
                    received += available; // Duplicate packets must not advance this count.
                    expected_sequence = expected_sequence.wrapping_add(1);

                    if !link.data_ack(sequence).await {
                        return;
                    }
                } else if received > 0 && sequence == expected_sequence.wrapping_sub(1) {
                    // Re-ACK a frame whose previous ACK was lost.
                    if !link.data_ack(sequence).await {
                        return;
                    }
                } else {
                    // Out-of-order or oversized packets invalidate this block transfer.
                    let _ = link.nack(WRITE_MEMORY).await;
                    return;
                }
            } else if received == 0
                && id == link.ids.request
                && payload.len() == WRITE_REQUEST_LEN
                && payload[0] == WRITE_MEMORY
            {
                let retry_offset = u32be(&payload[1..5]);
                let retry_count = u16be(&payload[5..7]) as u32;

                if retry_offset == offset && retry_count == count {
                    // Re-ACK WRITE_MEMORY if the first ACK was lost.
                    if !link.ack(WRITE_MEMORY).await {
                        return;
                    }
                } else {
                    let _ = link.nack(WRITE_MEMORY).await;
                    return;
                }
            }
        }

        // Packet ACKs confirm receipt; the final command ACK confirms persistence.
        if !flash::write(updater, offset, &buffer[..count as usize]) {
            let _ = link.nack(WRITE_MEMORY).await;
            return;
        }

        context.next_offset += count; // Advance only after the flash write succeeds.
        context.state = UpdateState::Updating;
        self.completed = CompletedWrite {
            valid: true,
            offset,
            count,
        };
        let _ = link.ack(WRITE_MEMORY).await;
    }

    /// Verify the complete staged image before allowing activation.
    pub async fn compute_crc<T, DFU, STATE>(
        &mut self,
        link: &mut Link<'_, T>,
        updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
        context: &mut Context,
    ) where
        T: Transport,
        DFU: NorFlash,
        STATE: NorFlash,
    {
        // A CRC request cannot verify an unannounced or partially received image.
        if context.state == UpdateState::Idle || context.next_offset != context.image_size {
            let _ = link.nack(COMPUTE_CRC).await;
            return;
        }

        let Some(value) = flash::crc(updater, context.image_size) else {
            let _ = link.nack(COMPUTE_CRC).await;
            return;
        };

        if value != context.expected_crc || !flash::image_valid(updater) {
            context.state = UpdateState::Updating; // Revoke any earlier verification.
            let _ = link.nack(COMPUTE_CRC).await;
            return;
        }

        context.state = UpdateState::Verified; // ACTIVATE is permitted from this point.
        let bytes = value.to_be_bytes();
        let response = [COMPUTE_CRC, bytes[0], bytes[1], bytes[2], bytes[3]];

        if link.ack(COMPUTE_CRC).await {
            let _ = link.send(&response).await;
        }
    }

    /// Persist the swap request and reset only after transmitting the ACK.
    pub async fn activate<T, DFU, STATE>(
        &mut self,
        link: &mut Link<'_, T>,
        updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
        context: &Context,
    ) -> Option<BootAction>
    where
        T: Transport,
        DFU: NorFlash,
        STATE: NorFlash,
    {
        if context.state != UpdateState::Verified {
            let _ = link.nack(ACTIVATE).await;
            return None;
        }

        // BootLoader::prepare performs the swap on the next boot.
        if updater.mark_updated().is_err() {
            let _ = link.nack(ACTIVATE).await;
            return None;
        }

        // Resetting while the ACK is queued would leave the host unsure of success.
        if link.ack(ACTIVATE).await && link.flush().await {
            Some(BootAction::Reset)
        } else {
            None
        }
    }

    /// ACK and launch the current active image without activating staged firmware.
    pub async fn start_app<T: Transport>(&mut self, link: &mut Link<'_, T>) -> Option<BootAction> {
        if !image::active_valid() {
            let _ = link.nack(START_APP).await;
            return None;
        }

        if link.ack(START_APP).await && link.flush().await {
            Some(BootAction::StartApp)
        } else {
            None
        }
    }
}

// Accept sequential, bounded blocks with aligned starts and final-block padding.
fn write_valid(context: &Context, offset: u32, count: u32, write_size: u32) -> bool {
    // Disallow holes and rewrites; the duplicate-block path handles lost ACKs.
    if count == 0 || count as usize > MAX_WRITE || offset != context.next_offset {
        return false;
    }

    if !offset.is_multiple_of(write_size) || offset + count > context.image_size {
        return false;
    }

    // Only the final block may need padding to the flash write size.
    offset + count == context.image_size || count.is_multiple_of(write_size)
}

// Callers validate request lengths before decoding these big-endian fields.
fn u32be(data: &[u8]) -> u32 {
    ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
}

fn u24(data: &[u8]) -> u32 {
    ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32
}

fn u16be(data: &[u8]) -> u16 {
    ((data[0] as u16) << 8) | data[1] as u16
}
