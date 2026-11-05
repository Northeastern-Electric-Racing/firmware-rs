//! What can be kept on a part.
//!
//! [`Storable`] is all the driver asks of a value: how many bytes it takes, and how
//! to move it in and out of them. A scheme decides where those bytes live.

/// A value that can be kept on a part.
///
/// All the driver asks of a payload. How you get there - by hand, by `#[derive]`,
/// with `bitfield-struct` - is yours.
pub trait Storable: Sized + Clone + PartialEq {
    /// Encoded length in bytes.
    const LEN: usize;

    /// Write `Self::LEN` bytes into the front of `buf`.
    fn encode(&self, buf: &mut [u8]);

    /// Read back what [`encode`](Self::encode) wrote, or `None` if it is not valid.
    fn decode(buf: &[u8]) -> Option<Self>;
}

/// Bytes needed to hold `bits` flags.
pub const fn bytes_for(bits: usize) -> usize {
    bits.div_ceil(8usize)
}

/// A flag per channel, one bit each.
///
/// Sized in bytes because an array length cannot be computed from a const generic on
/// stable: `BitMask<{ bytes_for(184) }>`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BitMask<const BYTES: usize>([u8; BYTES]);

impl<const BYTES: usize> BitMask<BYTES> {
    /// Every flag clear.
    pub const fn new() -> Self {
        Self([0u8; BYTES])
    }

    /// Whether flag `i` is set. Out of range reads as clear.
    pub const fn get(&self, i: usize) -> bool {
        if i >= BYTES * 8usize {
            return false;
        }
        self.0[i / 8usize] & (1u8 << (i % 8usize)) != 0u8
    }

    /// Set or clear flag `i`. Out of range is ignored.
    pub const fn set(&mut self, i: usize, value: bool) {
        if i >= BYTES * 8usize {
            return;
        }
        let bit = 1u8 << (i % 8usize);
        if value {
            self.0[i / 8usize] |= bit;
        } else {
            self.0[i / 8usize] &= !bit;
        }
    }

    /// Indices of the flags that are set.
    pub fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        (0usize..BYTES * 8usize).filter(|&i| self.get(i))
    }

    /// How many flags are set.
    pub fn count(&self) -> usize {
        self.iter_set().count()
    }
}

impl<const BYTES: usize> Storable for BitMask<BYTES> {
    const LEN: usize = BYTES;

    fn encode(&self, buf: &mut [u8]) {
        buf[..BYTES].copy_from_slice(&self.0);
    }

    fn decode(buf: &[u8]) -> Option<Self> {
        Some(Self(buf[..BYTES].try_into().ok()?))
    }
}

impl<const BYTES: usize> Default for BitMask<BYTES> {
    fn default() -> Self {
        Self::new()
    }
}

/// Leaf impls for a derived value to delegate to. They live here because only the
/// crate defining a trait may implement it for a foreign type.
macro_rules! impl_storable_int {
    ($($t:ty),* $(,)?) => {
        $(
            impl Storable for $t {
                const LEN: usize = core::mem::size_of::<$t>();

                fn encode(&self, buf: &mut [u8]) {
                    buf[..Self::LEN].copy_from_slice(&self.to_le_bytes());
                }

                fn decode(buf: &[u8]) -> Option<Self> {
                    Some(Self::from_le_bytes(buf[..Self::LEN].try_into().ok()?))
                }
            }
        )*
    };
}

impl_storable_int!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl Storable for bool {
    const LEN: usize = 1usize;

    fn encode(&self, buf: &mut [u8]) {
        buf[0usize] = *self as u8;
    }

    fn decode(buf: &[u8]) -> Option<Self> {
        Some(buf[0usize] != 0u8)
    }
}

impl<const N: usize> Storable for [u8; N] {
    const LEN: usize = N;

    fn encode(&self, buf: &mut [u8]) {
        buf[..N].copy_from_slice(self);
    }

    fn decode(buf: &[u8]) -> Option<Self> {
        buf[..N].try_into().ok()
    }
}
