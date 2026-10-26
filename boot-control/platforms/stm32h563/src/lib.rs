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
use embedded_can::{Frame, Id};

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
    /// Configure an exact CAN data-frame match without initializing CAN.
    pub fn new(flash: Peri<'d, FLASH>, request_id: Id) -> Self {
        let state = FlashState {
            flash: Mutex::new(RefCell::new(Flash::new_blocking(flash))),
        };
        Self {
            control: BootControl::new(state, request_id),
        }
    }

    /// Confirm successful application initialization to prevent rollback on reset.
    /// Call only after the application's required startup checks have succeeded.
    pub fn confirm_boot(&mut self) -> Result<(), FirmwareUpdaterError> {
        self.control.confirm_boot()
    }

    /// Check the ID, frame type, and complete payload before accepting a request.
    pub fn is_request(&self, frame: &impl Frame) -> bool {
        self.control.is_request(frame)
    }

    /// Ignore unrelated frames; persist a matching request and reset into the bootloader.
    /// Returns an error without resetting if the flash operation fails.
    /// Call from a task, after any application-specific shutdown needed before reset.
    pub fn handle_frame(&mut self, frame: &impl Frame) -> Result<(), FirmwareUpdaterError> {
        self.control.handle_frame(frame)
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
