#![no_std]

//! Application startup confirmation and CAN bootloader entry for STM32H563ZI.
//! CAN initialization, receive filters, and frame dispatch stay with the application.

use core::cell::RefCell;

use boot_control_core::{BootControl, Platform};
use embassy_boot_stm32::{AlignedBuffer, BlockingFirmwareState};
use embassy_embedded_hal::flash::partition::BlockingPartition;
use embassy_stm32::Peri;
use embassy_stm32::flash::{Blocking, Flash, WRITE_SIZE};
use embassy_stm32::peripherals::FLASH;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

pub use embassy_boot_stm32::FirmwareUpdaterError;

// Reserve an 8 KiB boot-state sector at 0x081FA000 in the bootloader layout.
// Offsets below are relative to the start of flash.
const STATE_OFFSET: u32 = 0x001F_A000;
const STATE_SIZE: u32 = 0x2000;

/// Own flash access for startup confirmation and persistent bootloader requests.
pub struct Bootloader<'d> {
    control: BootControl<FlashState<'d>>,
}

impl<'d> Bootloader<'d> {
    /// Own flash access for boot confirmation and entry requests.
    pub fn new(flash: Peri<'d, FLASH>) -> Self {
        let state = FlashState {
            flash: Mutex::new(RefCell::new(Flash::new_blocking(flash))),
        };
        Self {
            control: BootControl::new(state),
        }
    }

    /// Confirm successful application initialization to prevent rollback on reset.
    /// Call only after the application's required startup checks have succeeded.
    pub fn confirm_boot(&mut self) -> Result<(), FirmwareUpdaterError> {
        self.control.confirm_boot()
    }

    /// Validate boot-request data, persist the request, and reset.
    /// The caller must match the request ID and reject remote frames first.
    /// Invalid data is ignored; a failed state write returns without resetting.
    /// Call after any application-specific shutdown needed before reset.
    pub fn handle_request(&mut self, data: &[u8]) -> Result<(), FirmwareUpdaterError> {
        self.control.handle_request(data)
    }
}

struct FlashState<'d> {
    flash: Mutex<NoopRawMutex, RefCell<Flash<'d, Blocking>>>,
}

impl Platform for FlashState<'_> {
    type Error = FirmwareUpdaterError;

    fn confirm_boot(&mut self) -> Result<(), Self::Error> {
        self.write_state(false)
    }

    fn request_boot(&mut self) -> Result<(), Self::Error> {
        self.write_state(true)
    }

    fn reset() -> ! {
        cortex_m::peripheral::SCB::sys_reset()
    }
}

impl FlashState<'_> {
    // Construct the short-lived partition and aligned buffer without self-references.
    fn write_state(&mut self, detach: bool) -> Result<(), FirmwareUpdaterError> {
        let partition = BlockingPartition::new(&self.flash, STATE_OFFSET, STATE_SIZE);
        let mut aligned = AlignedBuffer([0; WRITE_SIZE]);
        let mut state = BlockingFirmwareState::new(partition, aligned.as_mut());
        if detach {
            state.mark_dfu()
        } else {
            state.mark_booted()
        }
    }
}
