//! A record kept in two slots, so one of them is always intact.
//!
//! One slot, as it sits on the part:
//!
//! ```text
//! seq      u32 le   0 .. 4
//! len      u8       4    payload length, so a changed value is rejected
//! payload  LEN      5 .. 5 + LEN
//! crc32    u32 le   5 + LEN .. 9 + LEN
//! ```
//!
//! The check value covers the crate magic and the slot index as well as the body,
//! neither of them stored: a blank page cannot validate, nor can a record in the
//! wrong slot, and bumping the magic retires every record on the part.
//!
//! One record per page is what makes a write atomic - one page is one stop condition
//! is one write cycle. Slots are addressed by page index, so a record starting part
//! way into a page cannot be expressed.
//!
//! This is the only scheme the driver has. A second one - an append only log, say -
//! would be a sibling module over the same [`Storable`](crate::Storable), not a change
//! here.

use embedded_storage_async::Storage;

use crate::{Storable, StoreError, crc32};

/// Sequence, payload length and CRC.
pub const HEADER: usize = 9;
const PAYLOAD: usize = 5;

/// Two is the smallest ring that always has a slot which is not the live one.
const SLOTS: usize = 2;

/// What the store remembers about a value's two slots between accesses - not the
/// slots themselves, which are on the part.
///
/// Lives in the store rather than the handle, so the cached value survives from one
/// access to the next and makes [`get`](DoubleBuffered::get) free.
pub struct SlotState<T> {
    live: Option<T>,
    seq: u32,
    next: u8,
}

impl<T> Default for SlotState<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SlotState<T> {
    /// Nothing read yet.
    pub const fn new() -> Self {
        Self {
            live: None,
            seq: 0u32,
            next: 0u8,
        }
    }
}

/// A `T` that outlives power, in pages `FIRST_PAGE` and `FIRST_PAGE + 1`.
///
/// Handed out by a store accessor, borrowing the part and the cached state for one
/// access.
pub struct DoubleBuffered<'a, S, T, const PAGE_SIZE: usize, const FIRST_PAGE: usize> {
    storage: &'a mut S,
    slots: &'a mut SlotState<T>,
}

impl<'a, S, T, const PAGE_SIZE: usize, const FIRST_PAGE: usize>
    DoubleBuffered<'a, S, T, PAGE_SIZE, FIRST_PAGE>
{
    /// Not public API: the generated accessors build these.
    #[doc(hidden)]
    pub fn __new(storage: &'a mut S, slots: &'a mut SlotState<T>) -> Self {
        Self { storage, slots }
    }

    /// Address where a slot begins, for logging and bring-up.
    pub const fn address(slot: u8) -> u32 {
        ((FIRST_PAGE + slot as usize) * PAGE_SIZE) as u32
    }

    /// The value on the part, from the copy in RAM. Not a bus access.
    pub const fn get(&self) -> Option<&T> {
        self.slots.live.as_ref()
    }

    /// Where the next commit will land.
    pub const fn next_address(&self) -> u32 {
        Self::address(self.slots.next)
    }
}

impl<S: Storage, T: Storable, const PAGE_SIZE: usize, const FIRST_PAGE: usize>
    DoubleBuffered<'_, S, T, PAGE_SIZE, FIRST_PAGE>
{
    /// Bytes one slot occupies on the part.
    const RECORD_LEN: usize = T::LEN + HEADER;

    fn encode(seq: u32, slot: u8, value: &T, buf: &mut [u8; PAGE_SIZE]) -> usize {
        let body = PAYLOAD + T::LEN;
        buf[0usize..4usize].copy_from_slice(&seq.to_le_bytes());
        buf[4usize] = T::LEN as u8;
        value.encode(&mut buf[PAYLOAD..body]);
        let crc = crc32(&[&[slot], &buf[..body]]);
        buf[body..body + 4usize].copy_from_slice(&crc.to_le_bytes());
        body + 4usize
    }

    /// `None` if the record is blank, torn, written for a different length, or
    /// sitting in a slot it was not written for.
    fn decode(slot: u8, buf: &[u8; PAGE_SIZE]) -> Option<(u32, T)> {
        let body = PAYLOAD + T::LEN;
        let stored = u32::from_le_bytes(buf[body..body + 4usize].try_into().ok()?);
        if crc32(&[&[slot], &buf[..body]]) != stored {
            return None;
        }
        if buf[4usize] as usize != T::LEN {
            return None;
        }
        let seq = u32::from_le_bytes(buf[0usize..4usize].try_into().ok()?);
        Some((seq, T::decode(&buf[PAYLOAD..body])?))
    }

    /// Read both slots and settle which one is live. Call once, at startup.
    pub async fn open(&mut self) -> Result<(), StoreError<S::Error>> {
        debug_assert!(
            T::LEN + HEADER <= PAGE_SIZE,
            "value does not fit in one page of this part"
        );

        let mut buf = [0u8; PAGE_SIZE];
        let len = Self::RECORD_LEN;

        // Highest sequence wins. Scanning in order gives a tie to the lower slot, and
        // a part where nothing validates leaves the next write aimed at slot zero.
        let mut found: Option<u8> = None;
        self.slots.seq = 0u32;
        self.slots.live = None;
        for slot in 0u8..SLOTS as u8 {
            self.storage
                .read(Self::address(slot), &mut buf[..len])
                .await
                .map_err(StoreError::Storage)?;
            if let Some((seq, value)) = Self::decode(slot, &buf)
                && (found.is_none() || seq > self.slots.seq)
            {
                self.slots.seq = seq;
                self.slots.live = Some(value);
                found = Some(slot);
            }
        }
        self.slots.next = found.map_or(0u8, |slot| ((slot as usize + 1usize) % SLOTS) as u8);

        Ok(())
    }

    /// Put `value` on the part, returning whether it cost a write cycle.
    ///
    /// The same value again is a no-op, so call it on any cadence. A real write goes
    /// to the slot that is not live, so a failure leaves the previous value current.
    pub async fn commit(&mut self, value: &T) -> Result<bool, StoreError<S::Error>> {
        if self.slots.live.as_ref() == Some(value) {
            return Ok(false);
        }

        let slot = self.slots.next;
        let seq = self.slots.seq.wrapping_add(1u32);
        let address = Self::address(slot);

        let mut buf = [0u8; PAGE_SIZE];
        let len = Self::encode(seq, slot, value, &mut buf);
        self.storage
            .write(address, &buf[..len])
            .await
            .map_err(StoreError::Storage)?;

        // I2C carries no checksum and the part acknowledges a byte it may still have
        // stored wrong. Decoding the read back rather than comparing it to what went
        // out reuses the buffer and asks whether the next open will see this record.
        self.storage
            .read(address, &mut buf[..len])
            .await
            .map_err(StoreError::Storage)?;
        match Self::decode(slot, &buf) {
            Some((back, ref v)) if back == seq && v == value => (),
            _ => return Err(StoreError::Verify),
        }

        self.slots.seq = seq;
        self.slots.live = Some(value.clone());
        self.slots.next = ((slot as usize + 1usize) % SLOTS) as u8;
        Ok(true)
    }
}
