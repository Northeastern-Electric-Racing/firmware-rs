//! What a scheme is, and the ones a layout field can pick.
//!
//! | scheme | pages | overhead | tells a blank page | survives a torn commit |
//! |---|---|---|---|---|
//! | [`Unchecked`] | 1 | 0 | no | no |
//! | [`RingBuffered<T, N>`](RingBuffered) | N | 8 | yes | yes |
//!
//! Each is a framing and nothing more; [`Access`](crate::Access) does the reading,
//! writing and caching for all of them. A third is an `encode`/`decode` pair here and
//! no change anywhere else. `record` throughout is exactly
//! [`MemoryScheme::RECORD_BYTES`] long.

use crate::Storable;
use core::marker::PhantomData;
use crc::{CRC_32_ISO_HDLC, Crc};

/// How a field frames its value on a page, and how many pages that takes.
pub trait MemoryScheme {
    type Value: Storable;
    const RECORD_BYTES: usize;
    const PAGES: usize;
    /// Where the next commit goes, and what the last update found.
    type Selector: PageSelector;

    fn encode(seq: u32, value: &Self::Value, record: &mut [u8]);
    fn decode(record: &[u8]) -> Option<(u32, Self::Value)>;
}

pub trait PageSelector {
    fn new(pages: usize) -> Self;

    fn seq(&self) -> u32;

    fn next_page(&self) -> usize;

    /// What [`Access::update`](crate::Access::update) found: the live record's sequence and page, and how many pages held a record at all.
    fn updated(&mut self, seq: u32, live_page: Option<usize>, intact: usize);

    /// A commit that landed on [`next_page`](Self::next_page).
    fn committed(&mut self, seq: u32);
}

impl PageSelector for () {
    fn new(_: usize) -> Self {}
    fn seq(&self) -> u32 {
        0u32
    }
    fn next_page(&self) -> usize {
        0usize
    }
    fn updated(&mut self, _: u32, _: Option<usize>, _: usize) {}
    fn committed(&mut self, _: u32) {}
}

/// Pages taking turns: which one is next, and how many hold a record at all.
pub struct Ring {
    seq: u32,
    next_page: u8,
    intact: u8,
    pages: u8,
}

impl Ring {
    /// Commits since the field was blank.
    pub const fn seq(&self) -> u32 {
        self.seq
    }

    /// Pages holding a record, as of the last update or commit.
    pub const fn intact(&self) -> usize {
        self.intact as usize
    }

    const fn wrap(&self, page: usize) -> u8 {
        (page % self.pages as usize) as u8
    }
}

impl PageSelector for Ring {
    fn new(pages: usize) -> Self {
        Self {
            seq: 0u32,
            next_page: 0u8,
            intact: 0u8,
            pages: pages as u8,
        }
    }

    fn seq(&self) -> u32 {
        Ring::seq(self)
    }

    fn next_page(&self) -> usize {
        self.next_page as usize
    }

    fn updated(&mut self, seq: u32, live_page: Option<usize>, intact: usize) {
        self.seq = seq;
        // Nothing decoding leaves the next write aimed at the first page.
        self.next_page = live_page.map_or(0u8, |live| self.wrap(live + 1usize));
        self.intact = intact as u8;
    }

    fn committed(&mut self, seq: u32) {
        self.seq = seq;
        self.next_page = self.wrap(self.next_page as usize + 1usize);
        // Only an `update` counts back down: a commit knows the page it wrote holds a
        // record, not whether it already did.
        self.intact = self.intact.saturating_add(1u8).min(self.pages);
    }
}

const SEQ_BYTES: usize = size_of::<u32>();
const CRC_BYTES: usize = size_of::<u32>();

fn crc<T: Storable>(body: &[u8]) -> u32 {
    const MAGIC: u32 = 0x1234_5678;
    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

    let mut digest = CRC.digest();
    digest.update(&MAGIC.to_le_bytes());
    digest.update(&(T::LEN as u32).to_le_bytes());
    digest.update(body);
    digest.finalize()
}

/// A `T` in one page, as bare bytes without any checks.
pub struct Unchecked<T>(PhantomData<T>);

impl<T: Storable> MemoryScheme for Unchecked<T> {
    type Value = T;
    type Selector = ();
    const PAGES: usize = 1;
    const RECORD_BYTES: usize = T::LEN;

    fn encode(_seq: u32, value: &T, record: &mut [u8]) {
        value.encode(record);
    }

    fn decode(record: &[u8]) -> Option<(u32, T)> {
        Some((0u32, T::decode(record)?))
    }
}

/// A `T` across `N` pages, each a whole record, so one of them is always intact.
///
/// ```text
/// seq      u32 le   0 .. 4
/// payload  LEN      4 .. 4 + LEN
/// crc32    u32 le   4 + LEN .. 8 + LEN
/// ```
///
/// A commit goes to the page after the live one, so losing power part way through costs
/// that page and nothing else; the sequence number picks the survivor. `N` is a wear
/// dial as much as a safety one, since each page sees about `seq / N` write cycles.
pub struct RingBuffered<T, const N: usize>(PhantomData<T>);

/// Ringbuffer with 2 pages
pub type DoubleBuffered<T> = RingBuffered<T, 2>;

impl<T: Storable, const N: usize> MemoryScheme for RingBuffered<T, N> {
    type Value = T;
    type Selector = Ring;

    const PAGES: usize = {
        assert!(N >= 2usize, "a ring needs a page that is not the live one");
        N
    };
    const RECORD_BYTES: usize = SEQ_BYTES + T::LEN + CRC_BYTES;

    fn encode(seq: u32, value: &T, record: &mut [u8]) {
        let body = SEQ_BYTES + T::LEN;
        record[..SEQ_BYTES].copy_from_slice(&seq.to_le_bytes());
        value.encode(&mut record[SEQ_BYTES..body]);
        let crc = crc::<T>(&record[..body]).to_le_bytes();
        record[body..].copy_from_slice(&crc);
    }

    fn decode(record: &[u8]) -> Option<(u32, T)> {
        let body = SEQ_BYTES + T::LEN;
        let stored = u32::from_le_bytes(record[body..].try_into().ok()?);
        if crc::<T>(&record[..body]) != stored {
            return None;
        }
        Some((
            u32::from_le_bytes(record[..SEQ_BYTES].try_into().ok()?),
            T::decode(&record[SEQ_BYTES..body])?,
        ))
    }
}
