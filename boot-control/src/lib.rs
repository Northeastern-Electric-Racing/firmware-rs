#![no_std]

//! Shared CAN boot-request matching and application boot control.

use embedded_can::{Frame, Id};

const BOOT_REQUEST_DATA: &[u8] = &[0xB0, 0x07, 0x10, 0xAD];

/// Platform operations for persistent boot state and system reset.
pub trait Platform {
    type Error;

    fn confirm_boot(&mut self) -> Result<(), Self::Error>;
    fn request_boot(&mut self) -> Result<(), Self::Error>;
    fn reset() -> !;
}

/// Match CAN boot requests and persist state before resetting.
pub struct BootControl<P> {
    platform: P,
    request_id: Id,
}

impl<P: Platform> BootControl<P> {
    /// Configure an exact CAN data-frame match without initializing CAN.
    pub fn new(platform: P, request_id: Id) -> Self {
        Self {
            platform,
            request_id,
        }
    }

    /// Confirm startup after the application's required checks have succeeded.
    pub fn confirm_boot(&mut self) -> Result<(), P::Error> {
        self.platform.confirm_boot()
    }

    /// Check the ID, frame type, and complete payload before accepting a request.
    pub fn is_request(&self, frame: &impl Frame) -> bool {
        !frame.is_remote_frame()
            && frame.id() == self.request_id
            && frame.data() == BOOT_REQUEST_DATA
    }

    /// Persist a matching request and reset; ignore unrelated frames.
    /// A failed state write returns an error without resetting.
    /// Call after any application-specific shutdown needed before reset.
    pub fn handle_frame(&mut self, frame: &impl Frame) -> Result<(), P::Error> {
        if self.is_request(frame) {
            self.platform.request_boot()?;
            P::reset();
        }
        Ok(())
    }
}
