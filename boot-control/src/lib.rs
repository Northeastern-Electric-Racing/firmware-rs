#![no_std]

//! Shared CAN boot-request matching and application boot control.

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
}

impl<P: Platform> BootControl<P> {
    /// Use the platform's boot-state storage and reset implementation.
    pub fn new(platform: P) -> Self {
        Self { platform }
    }

    /// Confirm startup after the application's required checks have succeeded.
    pub fn confirm_boot(&mut self) -> Result<(), P::Error> {
        self.platform.confirm_boot()
    }

    /// Validate boot-request data, persist the request, and reset.
    /// The caller must match the request ID and reject remote frames first.
    /// Invalid data is ignored; a failed state write returns without resetting.
    pub fn handle_request(&mut self, data: &[u8]) -> Result<(), P::Error> {
        if data == BOOT_REQUEST_DATA {
            self.platform.request_boot()?;
            P::reset();
        }
        Ok(())
    }
}
