//! What the store keeps for each field, and the reading, writing and caching that
//! every scheme shares.
//!
//! The framing itself is [`schemes`](crate::schemes); the part is reached only
//! through [`PagedMemory`].

use crate::{Error, MemoryScheme, PageSelector, PagedMemory, Ring};

#[derive(Default)]
enum CachedValue<T> {
    #[default]
    Cold,
    Blank,
    Live(T),
}

impl<T> CachedValue<T> {
    const fn live(&self) -> Option<&T> {
        match self {
            Self::Live(value) => Some(value),
            _ => None,
        }
    }

    const fn is_cold(&self) -> bool {
        matches!(self, Self::Cold)
    }
}

/// What the store keeps for one field between accesses, whatever its scheme.
pub struct Field<K: MemoryScheme> {
    value: CachedValue<K::Value>,
    selector: K::Selector,
}

impl<K: MemoryScheme> Default for Field<K> {
    fn default() -> Self {
        Self {
            value: CachedValue::Cold,
            selector: K::Selector::new(K::PAGES),
        }
    }
}

/// What a [`commit`](Access::commit) cost.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Commit {
    /// A write cycle, and a read back that proved it landed.
    Written,
    /// Nothing: the part already held this value.
    Unchanged,
}

/// A borrow of one field: the part it lives on, and what the store keeps for it.
pub struct Access<
    'a,
    S: PagedMemory,
    K: MemoryScheme,
    const PAGE_SIZE: usize,
    const FIRST_PAGE: usize,
> {
    part: &'a mut S,
    field: &'a mut Field<K>,
}

impl<'a, S: PagedMemory, K: MemoryScheme, const PAGE_SIZE: usize, const FIRST_PAGE: usize>
    Access<'a, S, K, PAGE_SIZE, FIRST_PAGE>
{
    #[doc(hidden)]
    pub const fn __new(part: &'a mut S, field: &'a mut Field<K>) -> Self {
        Self { part, field }
    }

    pub const fn cached(&self) -> Option<&K::Value> {
        self.field.value.live()
    }

    /// Read every page of the field and settle which record is live. An optimisation
    /// the first time, since [`read`](Self::read) and [`commit`](Self::commit) do it
    /// themselves; after that it is the only way to re-scan one field's pages, which
    /// is what moves [`is_degraded`](Self::is_degraded).
    pub async fn update(&mut self) -> Result<(), Error<S::BusError>> {
        let mut buf = [0u8; PAGE_SIZE];
        let mut best: Option<(u32, K::Value)> = None;
        let mut live_page: Option<usize> = None;
        let mut intact = 0usize;

        // Highest sequence wins, ties to the lower page.
        for page in 0usize..K::PAGES {
            let record = &mut buf[..K::RECORD_BYTES];
            self.part.read_page(FIRST_PAGE + page, record).await?;
            let Some((seq, value)) = K::decode(record) else {
                continue;
            };
            intact += 1usize;
            if best.as_ref().is_none_or(|(best, _)| seq > *best) {
                best = Some((seq, value));
                live_page = Some(page);
            }
        }

        let seq = best.as_ref().map_or(0u32, |(seq, _)| *seq);
        self.field.selector.updated(seq, live_page, intact);
        self.field.value = match best {
            Some((_, value)) => CachedValue::Live(value),
            None => CachedValue::Blank,
        };

        Ok(())
    }

    /// The live value, reading the part first if it has not been read yet. This driver
    /// is the only writer, so once read the cache cannot go stale.
    pub async fn read(&mut self) -> Result<Option<&K::Value>, Error<S::BusError>> {
        if self.field.value.is_cold() {
            self.update().await?;
        }
        Ok(self.field.value.live())
    }

    /// Put `value` on the part. The same value again is [`Unchanged`](Commit::Unchanged),
    /// so call it on any cadence; with more than one page the write goes to a page that
    /// is not live, so a failure leaves the previous value current.
    pub async fn commit(&mut self, value: &K::Value) -> Result<Commit, Error<S::BusError>> {
        // Without this a commit on a cold cache would start again from sequence 1,
        // under whatever is already live, and the next update would pick the older page.
        if self.field.value.is_cold() {
            self.update().await?;
        }
        if self.field.value.live() == Some(value) {
            return Ok(Commit::Unchanged);
        }

        let seq = self.field.selector.seq().wrapping_add(1u32);
        let page = FIRST_PAGE + self.field.selector.next_page();
        let mut buf = [0u8; PAGE_SIZE];
        let record = &mut buf[..K::RECORD_BYTES];

        K::encode(seq, value, record);
        self.part.write_page(page, record).await?;

        // I2C carries no checksum and the part acknowledges a byte it may still have
        // stored wrong. Decoding the read back asks whether the next update will see it.
        self.part.read_page(page, record).await?;
        match K::decode(record) {
            Some((_, ref got)) if got == value => (),
            _ => return Err(Error::Verify),
        }

        self.field.selector.committed(seq);
        self.field.value = CachedValue::Live(value.clone());

        Ok(Commit::Written)
    }
}

/// Only where pages take turns, which is the only time either means anything.
impl<
    S: PagedMemory,
    K: MemoryScheme<Selector = Ring>,
    const PAGE_SIZE: usize,
    const FIRST_PAGE: usize,
> Access<'_, S, K, PAGE_SIZE, FIRST_PAGE>
{
    /// Commits since the field was blank, `None` until it has been read. A wear figure:
    /// pages take turns, so each has seen about `seq / PAGES` cycles.
    pub const fn seq(&self) -> Option<u32> {
        if self.field.value.is_cold() {
            None
        } else {
            Some(self.field.selector.seq())
        }
    }

    /// Fewer pages hold a record than should by now. A page the field has not reached yet
    /// is blank rather than rotted, so until the ring has been round once the bar is how
    /// many commits there have been, not how many pages there are.
    ///
    /// As of the last [`update`](Self::update), which is the only thing that counts a page
    /// back out; a commit only ever counts one in.
    pub const fn is_degraded(&self) -> bool {
        let committed = self.field.selector.seq() as usize;
        let expected = if committed < K::PAGES {
            committed
        } else {
            K::PAGES
        };
        self.field.selector.intact() < expected
    }
}

/// Both of a layout's build-time checks: a field's record against a page, and the whole
/// layout against the part. Here rather than in the macro so the rule is the driver's.
pub const fn fits(needed: usize, available: usize) -> bool {
    needed <= available
}
