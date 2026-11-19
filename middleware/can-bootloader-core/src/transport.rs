//! CAN controller interface implemented by each platform.

use embedded_can::Id;

/// Bootloader protocol CAN IDs for one ECU, generated from `can-bootloader-config`.
#[derive(Clone, Copy)]
pub struct BootloaderCanIds {
    /// Host command requests.
    pub request: Id,
    /// Host image data packets.
    pub data: Id,
    /// Bootloader responses.
    pub response: Id,
}

/// Classical CAN data frame passed between a transport and the protocol.
#[derive(Clone, Copy)]
pub struct Frame {
    id: Id,
    len: u8,
    data: [u8; 8],
}

impl Frame {
    /// Return None when the payload exceeds eight bytes.
    pub fn new(id: Id, data: &[u8]) -> Option<Self> {
        let mut bytes = [0; 8];
        bytes.get_mut(..data.len())?.copy_from_slice(data);
        Some(Self {
            id,
            len: data.len() as u8,
            data: bytes,
        })
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

/// CAN controller operations used by the bootloader session.
// The bootloader runs on one executor, so returned futures need no Send bound.
#[allow(async_fn_in_trait)]
pub trait Transport {
    /// Wait for the next data frame, skipping remote frames and bus errors.
    async fn receive(&mut self) -> Frame;

    /// Queue a frame; success does not mean it has finished transmitting.
    async fn send(&mut self, frame: &Frame) -> bool;

    /// Wait until queued frames finish transmitting; the caller bounds the wait.
    async fn flush(&mut self);
}
