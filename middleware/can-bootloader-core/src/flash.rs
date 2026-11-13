//! Write and validate staged firmware without modifying the active application.

use crc::{CRC_32_MPEG_2, Crc};
use embassy_boot::BlockingFirmwareUpdater;
use embedded_storage::nor_flash::NorFlash;

use crate::image::{self, active_size};
use crate::protocol::MAX_WRITE;

// Largest flash programming unit the fixed staging buffers support.
const MAX_WRITE_SIZE: usize = 32;

// Reject unsupported flash at compile time.
fn write_size<DFU: NorFlash>() -> usize {
    const { assert!(DFU::WRITE_SIZE <= MAX_WRITE_SIZE && MAX_WRITE.is_multiple_of(DFU::WRITE_SIZE)) };
    DFU::WRITE_SIZE
}

/// Stage a validated block of at most MAX_WRITE bytes at a DFU-relative offset.
pub fn write<DFU, STATE>(
    updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
    offset: u32,
    data: &[u8],
) -> bool
where
    DFU: NorFlash,
    STATE: NorFlash,
{
    if offset >= active_size() || data.is_empty() || offset + data.len() as u32 > active_size() {
        return false;
    }

    // Pad the final flash word with erased bytes; the CRC excludes this padding.
    let padded_len = data.len().next_multiple_of(write_size::<DFU>());
    let mut padded = [0xFFu8; MAX_WRITE + MAX_WRITE_SIZE]; // Match the erased flash value.
    padded[..data.len()].copy_from_slice(data);

    // Embassy erases DFU sectors as needed and enforces the persisted boot state.
    updater
        .write_firmware(offset as usize, &padded[..padded_len])
        .is_ok()
}

/// Compute CRC-32/MPEG-2 over exactly the declared image length.
pub fn crc<DFU, STATE>(
    updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>,
    image_size: u32,
) -> Option<u32>
where
    DFU: NorFlash,
    STATE: NorFlash,
{
    if image_size == 0 || image_size > active_size() {
        return None;
    }

    let crc = Crc::<u32>::new(&CRC_32_MPEG_2);
    let mut digest = crc.digest();
    let mut buffer = [0u8; MAX_WRITE];
    let mut offset = 0u32;

    while offset < image_size {
        let count = core::cmp::min(buffer.len() as u32, image_size - offset) as usize;
        // Align the flash read while hashing only actual image bytes.
        let read_len = count.next_multiple_of(write_size::<DFU>());

        if updater.read_dfu(offset, &mut buffer[..read_len]).is_err() {
            return None;
        }

        digest.update(&buffer[..count]); // Ignore alignment bytes beyond image_size.
        offset += count as u32;
    }

    Some(digest.finalize())
}

/// Require the staged vectors to target SRAM and the final active partition.
pub fn image_valid<DFU, STATE>(updater: &mut BlockingFirmwareUpdater<'_, DFU, STATE>) -> bool
where
    DFU: NorFlash,
    STATE: NorFlash,
{
    // Read whole programming units covering the first two vector entries.
    let mut vector = [0u8; MAX_WRITE_SIZE];
    let len = write_size::<DFU>().max(8);
    if updater.read_dfu(0, &mut vector[..len]).is_err() {
        return false;
    }

    vector.first_chunk::<8>().is_some_and(image::vector_valid)
}
