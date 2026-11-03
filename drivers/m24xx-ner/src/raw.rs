//! Pages with no scheme over them.

use embedded_storage_async::Storage;

use crate::StoreError;

/// `PAGES` pages with no scheme over them.
///
/// Offsets are relative to the region and bounded by it, so a raw access cannot
/// reach a page another region owns. Nothing here is atomic past one page.
pub struct Raw<'a, S, const PAGE_SIZE: usize, const FIRST_PAGE: usize, const PAGES: usize> {
    storage: &'a mut S,
}

impl<'a, S, const PAGE_SIZE: usize, const FIRST_PAGE: usize, const PAGES: usize>
    Raw<'a, S, PAGE_SIZE, FIRST_PAGE, PAGES>
{
    /// Not public API: the generated accessors build these.
    #[doc(hidden)]
    pub fn __new(storage: &'a mut S) -> Self {
        Self { storage }
    }

    /// Size of the region in bytes.
    pub const fn len() -> u32 {
        (PAGES * PAGE_SIZE) as u32
    }

    const fn bounded<E>(offset: u32, len: usize) -> Result<u32, StoreError<E>> {
        let len = len as u32;
        match offset.checked_add(len) {
            Some(end) if end <= Self::len() => Ok((FIRST_PAGE * PAGE_SIZE) as u32 + offset),
            _ => Err(StoreError::OutOfRegion {
                offset,
                len,
                region: Self::len(),
            }),
        }
    }
}

impl<S: Storage, const PAGE_SIZE: usize, const FIRST_PAGE: usize, const PAGES: usize>
    Raw<'_, S, PAGE_SIZE, FIRST_PAGE, PAGES>
{
    /// Read from `offset` within the region.
    pub async fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), StoreError<S::Error>> {
        let at = Self::bounded(offset, buf.len())?;
        self.storage
            .read(at, buf)
            .await
            .map_err(StoreError::Storage)
    }

    /// Write at `offset` within the region. Only single page writes are atomic.
    pub async fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), StoreError<S::Error>> {
        let at = Self::bounded(offset, data.len())?;
        self.storage
            .write(at, data)
            .await
            .map_err(StoreError::Storage)
    }
}
