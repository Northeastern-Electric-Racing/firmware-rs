#![no_std]

use bitfield_struct::bitfield;
use cangen_macro::generate_all_messages;
use embedded_can::{ExtendedId, StandardId};
pub use embedded_can::{Frame, Id};

mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

pub trait CanRepr: sealed::Sealed + Copy {
    type Bytes: AsRef<[u8]> + AsMut<[u8]> + Default;
    fn to_be_bytes(self) -> Self::Bytes;
    fn from_be_bytes(bytes: Self::Bytes) -> Self;
}

macro_rules! impl_can_repr {
    ($($t:ty),*) => {$(
        impl CanRepr for $t {
            type Bytes = [u8; core::mem::size_of::<$t>()];
            fn to_be_bytes(self) -> Self::Bytes { <$t>::to_be_bytes(self) }
            fn from_be_bytes(b: Self::Bytes) -> Self { <$t>::from_be_bytes(b) }
        }
    )*};
}
impl_can_repr!(u8, u16, u32, u64);

/// Creates a CAN frame
/// All the users have to do is specify `Repr`, `ID`, and `LEN`
/// Describes a CAN message's identity and layout. Implemented once per message.
pub trait CanMessage: Sized {
    type Repr: CanRepr;

    const ID: Id;
    const LEN: usize;

    #[doc(hidden)]
    const CHECK: () = {
        assert!(
            Self::LEN <= core::mem::size_of::<Self::Repr>(),
            "LEN exceeds Repr"
        );
        assert!(Self::LEN <= 8, "LEN exceeds the 8-byte CAN 2.0 limit");
    };
}

pub trait ToCanFrame: CanMessage + Into<<Self as CanMessage>::Repr> {
    fn to_can_frame<F: Frame>(self) -> F {
        let _: () = Self::CHECK;

        let repr: <Self as CanMessage>::Repr = self.into();
        let bytes = repr.to_be_bytes();
        F::new(Self::ID, &bytes.as_ref()[..Self::LEN]).expect("frame construction failed")
    }
}

pub trait FromCanFrame: CanMessage + From<<Self as CanMessage>::Repr> {
    fn from_can_frame<F: Frame>(frame: &F) -> Option<Self> {
        let _: () = Self::CHECK;

        if frame.is_remote_frame() || frame.id() != Self::ID || frame.data().len() != Self::LEN {
            return None;
        }
        let mut bytes = <<Self as CanMessage>::Repr as CanRepr>::Bytes::default();
        bytes.as_mut()[..Self::LEN].copy_from_slice(frame.data());
        Some(Self::from(
            <<Self as CanMessage>::Repr as CanRepr>::from_be_bytes(bytes),
        ))
    }
}

/// Error returned by the generated `try_with_*` / `try_set_*` accessors when a
/// physical value can't be represented in a scaled field's bit width.
///
/// The plain `with_*` / `set_*` accessors saturate such values instead of
/// failing; use the `try_*` variants when you need to detect the condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange {
    /// The field that rejected the value.
    pub field: &'static str,
}

/// Scaled fixed-point conversion helpers used by the generated bitfields.
///
///
/// They are referenced by the `#[bits(N, from = .., into = ..)]` attributes
/// that `generate_all_messages!` emits, e.g.
/// `#[bits(12, from = conv::div_from_u16::<12, 10>, into = conv::div_into_u16::<12, 10>)]`.
///
/// - `B` is the field width in bits (used for masking / sign-extension only).
/// - `ARG` is the formatter argument (the divisor or multiplier).
pub mod conv {
    macro_rules! scaled {
        ($u:ty, $i:ty, $w:literal,
         $div_from:ident, $div_into:ident, $mul_from:ident, $mul_into:ident,
         $sdiv_from:ident, $sdiv_into:ident, $smul_from:ident, $smul_into:ident) => {
            /// unsigned, divide: physical = raw / ARG
            pub const fn $div_from<const B: u32, const ARG: u32>(bits: $u) -> f32 {
                bits as f32 / ARG as f32
            }
            /// The clamped result is always within the `#[bits(B)]`
            /// range, so `bitfield_struct`'s bounds check never fires.
            /// This is why a custom try is needed
            pub const fn $div_into<const B: u32, const ARG: u32>(v: f32) -> $u {
                let hi = (<$u>::MAX >> ($w - B)) as f32;
                let raw = v * ARG as f32;
                (if raw < 0.0 {
                    0.0
                } else if raw > hi {
                    hi
                } else {
                    raw
                }) as $u
            }
            /// unsigned, multiply: physical = raw * ARG
            pub const fn $mul_from<const B: u32, const ARG: u32>(bits: $u) -> f32 {
                bits as f32 * ARG as f32
            }
            /// Saturates the raw count to `[0, 2^B - 1]` (see `$div_into`).
            pub const fn $mul_into<const B: u32, const ARG: u32>(v: f32) -> $u {
                let hi = (<$u>::MAX >> ($w - B)) as f32;
                let raw = v / ARG as f32;
                (if raw < 0.0 {
                    0.0
                } else if raw > hi {
                    hi
                } else {
                    raw
                }) as $u
            }
            /// signed, divide: sign-extended over B bits on read.
            pub const fn $sdiv_from<const B: u32, const ARG: u32>(bits: $u) -> f32 {
                let sh = $w - B;
                let s = ((bits << sh) as $i) >> sh;
                s as f32 / ARG as f32
            }
            /// Saturates the raw count to `[-2^(B-1), 2^(B-1) - 1]`, then masks
            /// to B bits so the two's-complement pattern fits the field.
            pub const fn $sdiv_into<const B: u32, const ARG: u32>(v: f32) -> $u {
                let half = 1i128 << (B - 1);
                let hi = (half - 1) as f32;
                let lo = -(half as f32);
                let raw = v * ARG as f32;
                let raw = if raw < lo {
                    lo
                } else if raw > hi {
                    hi
                } else {
                    raw
                };
                let mask = <$u>::MAX >> ($w - B);
                ((raw as $i) as $u) & mask
            }
            /// signed, multiply: sign-extended over B bits on read.
            pub const fn $smul_from<const B: u32, const ARG: u32>(bits: $u) -> f32 {
                let sh = $w - B;
                let s = ((bits << sh) as $i) >> sh;
                s as f32 * ARG as f32
            }
            /// Saturates the raw count to `[-2^(B-1), 2^(B-1) - 1]` (see `$sdiv_into`).
            pub const fn $smul_into<const B: u32, const ARG: u32>(v: f32) -> $u {
                let half = 1i128 << (B - 1);
                let hi = (half - 1) as f32;
                let lo = -(half as f32);
                let raw = v / ARG as f32;
                let raw = if raw < lo {
                    lo
                } else if raw > hi {
                    hi
                } else {
                    raw
                };
                let mask = <$u>::MAX >> ($w - B);
                ((raw as $i) as $u) & mask
            }
        };
    }

    scaled!(
        u8,
        i8,
        8u32,
        div_from_u8,
        div_into_u8,
        mul_from_u8,
        mul_into_u8,
        sdiv_from_u8,
        sdiv_into_u8,
        smul_from_u8,
        smul_into_u8
    );
    scaled!(
        u16,
        i16,
        16u32,
        div_from_u16,
        div_into_u16,
        mul_from_u16,
        mul_into_u16,
        sdiv_from_u16,
        sdiv_into_u16,
        smul_from_u16,
        smul_into_u16
    );
    scaled!(
        u32,
        i32,
        32u32,
        div_from_u32,
        div_into_u32,
        mul_from_u32,
        mul_into_u32,
        sdiv_from_u32,
        sdiv_into_u32,
        smul_from_u32,
        smul_into_u32
    );
    scaled!(
        u64,
        i64,
        64u32,
        div_from_u64,
        div_into_u64,
        mul_from_u64,
        mul_into_u64,
        sdiv_from_u64,
        sdiv_into_u64,
        smul_from_u64,
        smul_into_u64
    );
}

impl CanMessage for ExampleDoNotUse {
    type Repr = u64;
    const ID: Id = Id::Standard(StandardId::new(0x00).unwrap());
    const LEN: usize = 6;
}

/// Hand-written demo of what `generate_all_messages!` produces: an `f32`
/// accessor backed by a scaled integer field.
#[bitfield(u64)]
struct ExampleDoNotUse {
    /// `a` is stored in 4 bits and divided by 1000 to obtain the physical value.
    #[bits(4, from = conv::div_from_u8::<4, 1000>, into = conv::div_into_u8::<4, 1000>)]
    pub a: f32,
    pub b: u16,
    pub c: u16,
    #[bits(12)]
    _1: u16,
    _2: u16,
}

// entrypoint to macro.  All expanded code must be no_std
generate_all_messages!();
