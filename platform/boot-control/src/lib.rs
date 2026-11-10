#![no_std]

//! Application startup confirmation and CAN bootloader entry for STM32H563ZI.
//! CAN initialization, receive filters, and frame dispatch stay with the application.

use core::cell::RefCell;

use embassy_boot::{AlignedBuffer, BlockingFirmwareState, FirmwareUpdaterConfig};
use embassy_stm32::Peri;
use embassy_stm32::flash::{Config, Flash, WRITE_SIZE};
use embassy_stm32::mode::Blocking;
use embassy_stm32::peripherals::FLASH;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

pub use embassy_boot::FirmwareUpdaterError;

const BOOT_REQUEST_DATA: &[u8] = &[0xB0, 0x07, 0x10, 0xAD];

/// Returns whether boot-request frame data is a valid bootloader entry request.
/// The caller must match the request ID and reject remote frames first.
pub fn is_valid_request(data: &[u8]) -> bool {
    data == BOOT_REQUEST_DATA
}

/// Own flash access for startup confirmation and persistent bootloader requests.
pub struct Bootloader<'d> {
    flash: Mutex<NoopRawMutex, RefCell<Flash<'d, Blocking>>>,
}

impl<'d> Bootloader<'d> {
    /// Own flash access for boot confirmation and entry requests.
    pub fn new(flash: Peri<'d, FLASH>) -> Self {
        Self {
            flash: Mutex::new(RefCell::new(Flash::new_blocking(flash, Config::default()))),
        }
    }

    /// Confirm successful application initialization to prevent rollback on reset.
    /// Call only after the application's required startup checks have succeeded.
    pub fn confirm_boot(&mut self) -> Result<(), FirmwareUpdaterError> {
        self.write_state(false)
    }

    /// Persist the bootloader entry request and reset.
    /// Check the request with `is_valid_request` and run any cleanup first.
    /// Resets even if the state write fails, so the application restarts after its cleanup.
    pub fn enter_bootloader(&mut self) -> ! {
        // A failed write leaves no entry request, so the reset starts the application.
        let _ = self.write_state(true);
        cortex_m::peripheral::SCB::sys_reset();
    }

    // Read the partitions from the linker symbols in memory.x on each write,
    // so the state does not borrow from self.
    fn write_state(&mut self, detach: bool) -> Result<(), FirmwareUpdaterError> {
        let config = FirmwareUpdaterConfig::from_linkerfile_blocking(&self.flash, &self.flash);
        let mut aligned = AlignedBuffer([0; WRITE_SIZE]);
        let mut state = BlockingFirmwareState::from_config(config, aligned.as_mut());
        if detach {
            state.mark_dfu()
        } else {
            state.mark_booted()
        }
    }
}
