//! Bootloader request filtering and responses over a transport.

use embassy_time::{Duration, with_timeout};

use crate::protocol::*;
use crate::transport::{BootloaderCanIds, Frame, Transport};

/// Pair a transport with the ECU's IDs for one bootloader session.
pub struct Link<'a, T> {
    transport: &'a mut T,
    pub ids: BootloaderCanIds,
}

impl<'a, T: Transport> Link<'a, T> {
    pub fn new(transport: &'a mut T, ids: BootloaderCanIds) -> Self {
        Self { transport, ids }
    }

    /// Wait for a request or data frame, ignoring unrelated traffic.
    pub async fn receive(&mut self) -> Frame {
        loop {
            let frame = self.transport.receive().await;
            if frame.id() == self.ids.request || frame.id() == self.ids.data {
                return frame;
            }
        }
    }

    /// Return None when no matching frame arrives before the deadline.
    pub async fn receive_timeout(&mut self, timeout_ms: u64) -> Option<Frame> {
        // One deadline covers all discarded frames and recoverable bus errors.
        with_timeout(Duration::from_millis(timeout_ms), self.receive())
            .await
            .ok()
    }

    /// Queue a response frame.
    pub async fn send(&mut self, data: &[u8]) -> bool {
        match Frame::new(self.ids.response, data) {
            Some(frame) => self.transport.send(&frame).await,
            None => false,
        }
    }

    /// Queue a command ACK containing the command opcode.
    pub async fn ack(&mut self, command: u8) -> bool {
        self.send(&[ACK, command]).await
    }

    /// Include the packet sequence so the host can distinguish data ACKs.
    pub async fn data_ack(&mut self, sequence: u8) -> bool {
        self.send(&[ACK, WRITE_MEMORY, sequence]).await
    }

    /// Queue a rejection for the given command.
    pub async fn nack(&mut self, command: u8) -> bool {
        self.send(&[NACK, command]).await
    }

    /// Wait for pending transmissions to drain, with a bounded timeout.
    pub async fn flush(&mut self) -> bool {
        // A queued ACK must leave the controller before reset or application handoff.
        with_timeout(Duration::from_millis(TX_TIMEOUT_MS), self.transport.flush())
            .await
            .is_ok()
    }
}
