#![no_std]

use bitfield_struct::bitfield;
use cangen_macro::generate_all_messages;
use embedded_can::{ExtendedId, Frame, Id, StandardId};

mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

pub trait CanRepr: sealed::Sealed + Copy {
    type Bytes: AsRef<[u8]>;
    fn to_be_bytes(self) -> Self::Bytes;
}

impl CanRepr for u8 {
    type Bytes = [u8; 1];
    fn to_be_bytes(self) -> Self::Bytes {
        u8::to_be_bytes(self)
    }
}
impl CanRepr for u16 {
    type Bytes = [u8; 2];
    fn to_be_bytes(self) -> Self::Bytes {
        u16::to_be_bytes(self)
    }
}
impl CanRepr for u32 {
    type Bytes = [u8; 4];
    fn to_be_bytes(self) -> Self::Bytes {
        u32::to_be_bytes(self)
    }
}
impl CanRepr for u64 {
    type Bytes = [u8; 8];
    fn to_be_bytes(self) -> Self::Bytes {
        u64::to_be_bytes(self)
    }
}

/// Creates a CAN frame
/// All the users have to do is specify `Repr`, `ID`, and `LEN`
pub trait ToCanFrame: Sized + Into<Self::Repr> {
    type Repr: CanRepr;

    const ID: Id;
    const LEN: usize;

    #[doc(hidden)]
    /// a hack to ensure that the `Repr` is specified correctly
    const CHECK_BITS_FIT: () = assert!(
        Self::LEN <= core::mem::size_of::<Self::Repr>(),
        "BITS exceeds the backing integer's width"
    );

    #[doc(hidden)]
    /// a hack to ensure that the `Repr` and `LEN` are 8 bytes or less (CAN 2.0)
    const CHECK_LEN: () = assert!(
        Self::LEN <= 8 && core::mem::size_of::<Self::Repr>() <= 8,
        "BITS exceeds the backing integer's width"
    );

    fn to_can_frame<F: Frame>(self) -> F {
        // Default assoc consts are only evaluated when referenced, so these
        // force the compile-time bounds check they exist for -- without this,
        // an out-of-range `LEN`/`Repr` silently skips straight to a runtime
        // panic on the slice below instead of failing the build.
        let _: () = Self::CHECK_BITS_FIT;
        let _: () = Self::CHECK_LEN;

        let bytes = self.into().to_be_bytes();
        // this is guarranteed in bounds by the `CHECK_BITS_FIT`
        // SAFETY: this is guarranteed to be within the size of a CAN frame by `CHECK_LEN`
        unsafe { F::new(Self::ID, &bytes.as_ref()[..Self::LEN]).unwrap_unchecked() }
    }
}

/// Error returned by the generated `try_with_*` / `try_set_*` accessors when a
/// physical value can't be represented in a scaled field's bit width.
///
/// The plain `with_*` / `set_*` accessors saturate such values instead of
/// failing; use the `try_*` variants when you need to detect the condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange {
    /// The field (snake_case accessor name) that rejected the value.
    pub field: &'static str,
}

/// Scaled fixed-point conversion helpers used by the generated bitfields.
///
/// Every generated numeric field exposes the *physical* value as an `f32`
/// (the Rust equivalent of the C `float` the definitions use), while the
/// bitfield stores a raw `N`-bit integer. These `const` functions apply the
/// divisor / multiplier from the JSON `formatter` spec and, for signed
/// fields, sign-extend the stored bits.
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

impl ToCanFrame for ExampleDoNotUse {
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
