//! bit-struct

#![no_std]

use core::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    ops::{
        Add, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
        Mul, Rem, Shl, ShlAssign, Shr, ShrAssign, Sub,
    },
};
use num_traits::{Bounded, Num, One, Zero};

/// [UnsafeStorage] is used to mark that there are some arbitrary invariants which must
/// be maintained in storing its inner value. Therefore, creation and modifying of the
/// inner value is an "unsafe" behavior. Although it might not be unsafe in traditional
/// Rust terms (no memory unsafety), behavior might be "undefined"—or at least undocumented,
/// because invariants are expected to be upheld.
///
/// This is useful in macros which do not encapsulate their storage in modules. This makes
/// the macros for the end-user more ergonomic, as they can use the macro multiple times in
/// a single module.
#[repr(transparent)]
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Default)]
pub struct UnsafeStorage<T>(T);

impl<T> UnsafeStorage<T> {
    /// # Safety
    /// - See the broader scope that this is called in and which invariants are mentioned
    pub unsafe fn new_unsafe(inner: T) -> Self {
        Self(inner)
    }

    /// # Safety
    /// This should be a safe operation assuming that when modifying T to T',
    /// UnsafeStorage::new_unsafe(T') is safe
    pub unsafe fn as_ref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Copy> UnsafeStorage<T> {
    pub fn inner(&self) -> T {
        self.0
    }
}

/// A struct which allows for getting/setting a given property
pub struct GetSet<'a, P, T, const START: usize, const STOP: usize> {
    /// The referenced bitfield type.
    parent: &'a mut P,
    /// The type in the get/set operations
    _phantom: PhantomData<T>,
}

impl<'a, P, T, const START: usize, const STOP: usize> GetSet<'a, P, T, START, STOP> {
    pub fn start(&self) -> usize {
        START
    }

    pub fn stop(&self) -> usize {
        STOP
    }
}

/// A trait which defines how many bits are needed to store a struct.
///
/// # Safety
/// Define `Num` as `{i,u}{8,16,32,64,128}`.
/// - when calling `core::mem::transmute` on `Self`, only bits [0, COUNT) can be
///   non-zero
/// - TryFrom<Num> produces Some(x) <=> core::mem::transmute(num) produces a
///   valid Self(x)
/// - TryFrom<Num> produces None <=> core::mem::transmute(num) produces an
///   invalid state for Self
pub unsafe trait BitCount {
    const COUNT: usize;
}

macro_rules! bit_counts {
    ($($num: ty = $count: literal),*) => {
        $(
        unsafe impl BitCount for $num {
            const COUNT: usize = $count;
        }
        )*
    };
}

bit_counts!(u8 = 8, u16 = 16, u32 = 32, u64 = 64, u128 = 128, bool = 1);

macro_rules! always_valid {
    ($($elem: ty),*) => {
        $(
        unsafe impl <P> ValidCheck<P> for $elem {
            const ALWAYS_VALID: bool = true;
        }
        )*
    };
}

always_valid!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool);

impl u1 {
    pub const TRUE: u1 = u1(1);
    pub const FALSE: u1 = u1(0);

    pub fn toggle(self) -> Self {
        match self {
            Self::FALSE => u1::TRUE,
            _ => Self::FALSE,
        }
    }
}

macro_rules! new_signed_types {
    (
        $($name: ident($count: literal, $inner: ty, $signed: ty)),*
    ) => {
        $(

        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $name($inner);

        always_valid!($name);

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.value().partial_cmp(&other.value())
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.value().cmp(&other.value())
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{}", self.value()))
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{}", self.value()))
            }
        }

        #[macro_export]
        macro_rules! $name {
            ($value: expr) => {
                {
                    const VALUE: $signed = $value;
                    const _: () = assert!(VALUE <= bit_struct::$name::MAX, "The provided value is too large");
                    const _: () = assert!(VALUE >= bit_struct::$name::MIN, "The provided value is too small");
                    let res: $name = unsafe {bit_struct::$name::new_unchecked(VALUE)};
                    res
                }
            };
        }


        unsafe impl BitCount for $name {
            const COUNT: usize = $count;
        }

        num_traits!($name, $signed);

        impl $name {
            /// Create a new $name from value
            /// # Safety
            /// - value must fit within the number of bits defined in the type
            pub unsafe fn new_unchecked(value: $signed) -> Self {
                let unsigned_value = value as $inner;
                if value >= 0 {
                    Self(unsigned_value)
                } else {
                    // we can do this
                    let value = unsigned_value & Self::MAX_UNSIGNED;
                    Self(value | Self::POLARITY_FLAG)
                }
            }


            /// Create a new $name from value
            /// # Safety
            /// - value must fit within the number of bits defined in the type
            pub fn new(value: $signed) -> Option<Self> {
                if value < Self::MIN || value > Self::MAX {
                    None
                } else {
                    Some(unsafe {Self::new_unchecked(value)})
                }
            }

            pub fn inner_raw(self) -> $inner {
                self.0
            }

            const POLARITY_FLAG: $inner = (1 << ($count - 1));
            const MAX_UNSIGNED: $inner = (1 << ($count-1)) - 1;
            pub const MAX: $signed = Self::MAX_UNSIGNED as $signed;
            pub const MIN: $signed = -(Self::MAX_UNSIGNED as $signed) - 1;

            pub fn value(self) -> $signed {
                match self.0 >> ($count - 1) {
                    0 => self.0 as $signed,
                    _ => {
                        // 0's out negative
                        let rem = self.0 ^ Self::POLARITY_FLAG;
                        let amount = Self::MAX_UNSIGNED - rem;
                        -(amount as $signed) - 1
                    }
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(0)
            }
        }
        )*
    };
}

macro_rules! num_traits {
    ($num:ident, $super_kind:ty) => {
        impl Zero for $num {
            fn zero() -> Self {
                $num::new(0).unwrap()
            }

            fn is_zero(&self) -> bool {
                self.0 == 0
            }
        }

        impl Add for $num {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                $num::new(self.value() + rhs.value()).unwrap()
            }
        }

        impl One for $num {
            fn one() -> Self {
                $num::new(1).unwrap()
            }
        }

        impl Mul for $num {
            type Output = Self;

            fn mul(self, rhs: Self) -> Self::Output {
                $num::new(self.value() * rhs.value()).unwrap()
            }
        }

        impl Sub for $num {
            type Output = $num;

            fn sub(self, rhs: Self) -> Self::Output {
                $num::new(self.value() - rhs.value()).unwrap()
            }
        }

        impl Div for $num {
            type Output = Self;

            fn div(self, rhs: Self) -> Self::Output {
                $num::new(self.value() / rhs.value()).unwrap()
            }
        }

        impl Rem for $num {
            type Output = Self;

            fn rem(self, rhs: Self) -> Self::Output {
                $num::new(self.value() % rhs.value()).unwrap()
            }
        }

        impl Num for $num {
            type FromStrRadixErr = ();

            fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
                let parse = <$super_kind>::from_str_radix(str, radix).map_err(|_| ())?;
                $num::new(parse).ok_or(())
            }
        }

        impl Shr<usize> for $num {
            type Output = $num;

            fn shr(self, rhs: usize) -> Self::Output {
                $num::new(self.value() >> rhs).unwrap()
            }
        }

        impl Shl<usize> for $num {
            type Output = $num;

            fn shl(self, rhs: usize) -> Self::Output {
                $num::new(self.value() << rhs).unwrap()
            }
        }

        impl ShrAssign<usize> for $num {
            fn shr_assign(&mut self, rhs: usize) {
                let got = *self >> rhs;
                *self = got;
            }
        }

        impl ShlAssign<usize> for $num {
            fn shl_assign(&mut self, rhs: usize) {
                let got = *self << rhs;
                *self = got;
            }
        }

        impl Bounded for $num {
            fn min_value() -> Self {
                $num::new(Self::MIN).unwrap()
            }

            fn max_value() -> Self {
                $num::new(Self::MAX).unwrap()
            }
        }

        impl BitAnd for $num {
            type Output = $num;

            fn bitand(self, rhs: Self) -> Self::Output {
                $num(self.0 & rhs.0)
            }
        }

        impl BitXor for $num {
            type Output = $num;

            fn bitxor(self, rhs: Self) -> Self::Output {
                $num(self.0 ^ rhs.0)
            }
        }

        impl BitXorAssign for $num {
            fn bitxor_assign(&mut self, rhs: Self) {
                self.0 ^= rhs.0
            }
        }

        impl BitAndAssign for $num {
            fn bitand_assign(&mut self, rhs: Self) {
                self.0 &= rhs.0
            }
        }

        impl BitOr for $num {
            type Output = Self;

            fn bitor(self, rhs: Self) -> Self::Output {
                $num(self.0 | rhs.0)
            }
        }

        impl BitOrAssign for $num {
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0;
            }
        }
    };
}

macro_rules! new_unsigned_types {
    (
        $($name: ident($count: literal, $inner: ty)),*
    ) => {
        $(

        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name($inner);

        always_valid!($name);

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{}", self.0))
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{}", self.0))
            }
        }

        #[macro_export]
        macro_rules! $name {
            ($value: literal) => {
                {
                    const VALUE: $inner = $value;

                    // this is always valid because we have one more bit than we need in $inner
                    // type
                    const _: () = assert!(bit_struct::$name::MAX >= VALUE, "The provided value is too large");
                    unsafe {bit_struct::$name::new_unchecked(VALUE)}
                }
            };
        }


        unsafe impl BitCount for $name {
            const COUNT: usize = $count;
        }

        impl $name {
            pub const MAX: $inner = (1 << ($count)) - 1;
            pub const MIN: $inner = 0;

            /// Create a new $name from value
            /// # Safety
            /// - value must fit within the number of bits defined in the type
            pub unsafe fn new_unchecked(value: $inner) -> Self {
                Self(value)
            }

            pub fn new(value: $inner) -> Option<Self> {
                if value >= Self::MIN && value <= Self::MAX {
                    Some(unsafe {Self::new_unchecked(value)})
                } else {
                    None
                }
            }

            pub fn value(self) -> $inner {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(0)
            }
        }

        num_traits!($name, $inner);
        )*
    };
}

new_signed_types!(
    i2  (2, u8, i8),
    i3  (3, u8, i8),
    i4  (4, u8, i8),
    i5  (5, u8, i8),
    i6  (6, u8, i8),
    i7  (7, u8, i8),
    i9  (9, u16, i16),
    i10 (10, u16, i16),
    i11 (11, u16, i16),
    i12 (12, u16, i16),
    i13 (13, u16, i16),
    i14 (14, u16, i16),
    i15 (15, u16, i16),
    i17 (17, u32, i32),
    i18 (18, u32, i32),
    i19 (19, u32, i32),
    i20 (20, u32, i32),
    i21 (21, u32, i32),
    i22 (22, u32, i32),
    i23 (23, u32, i32),
    i24 (24, u32, i32),
    i25 (25, u32, i32),
    i26 (26, u32, i32),
    i27 (27, u32, i32),
    i28 (28, u32, i32),
    i29 (29, u32, i32),
    i30 (30, u32, i32),
    i31 (31, u32, i32),
    i33 (33, u64, i64),
    i34 (34, u64, i64),
    i35 (35, u64, i64),
    i36 (36, u64, i64),
    i37 (37, u64, i64),
    i38 (38, u64, i64),
    i39 (39, u64, i64),
    i40 (40, u64, i64),
    i41 (41, u64, i64),
    i42 (42, u64, i64),
    i43 (43, u64, i64),
    i44 (44, u64, i64),
    i45 (45, u64, i64),
    i46 (46, u64, i64),
    i47 (47, u64, i64),
    i48 (48, u64, i64),
    i49 (49, u64, i64),
    i50 (50, u64, i64),
    i51 (51, u64, i64),
    i52 (52, u64, i64),
    i53 (53, u64, i64),
    i54 (54, u64, i64),
    i55 (55, u64, i64),
    i56 (56, u64, i64),
    i57 (57, u64, i64),
    i58 (58, u64, i64),
    i59 (59, u64, i64),
    i60 (60, u64, i64),
    i61 (61, u64, i64),
    i62 (62, u64, i64),
    i63 (63, u64, i64)
);

new_unsigned_types!(
    u1  (1, u8),
    u2  (2, u8),
    u3  (3, u8),
    u4  (4, u8),
    u5  (5, u8),
    u6  (6, u8),
    u7  (7, u8),
    u9  (9, u16),
    u10 (10, u16),
    u11 (11, u16),
    u12 (12, u16),
    u13 (13, u16),
    u14 (14, u16),
    u15 (15, u16),
    u17 (17, u32),
    u18 (18, u32),
    u19 (19, u32),
    u20 (20, u32),
    u21 (21, u32),
    u22 (22, u32),
    u23 (23, u32),
    u24 (24, u32),
    u25 (25, u32),
    u26 (26, u32),
    u27 (27, u32),
    u28 (28, u32),
    u29 (29, u32),
    u30 (30, u32),
    u31 (31, u32),
    u33 (33, u64),
    u34 (34, u64),
    u35 (35, u64),
    u36 (36, u64),
    u37 (37, u64),
    u38 (38, u64),
    u39 (39, u64),
    u40 (40, u64),
    u41 (41, u64),
    u42 (42, u64),
    u43 (43, u64),
    u44 (44, u64),
    u45 (45, u64),
    u46 (46, u64),
    u47 (47, u64),
    u48 (48, u64),
    u49 (49, u64),
    u50 (50, u64),
    u51 (51, u64),
    u52 (52, u64),
    u53 (53, u64),
    u54 (54, u64),
    u55 (55, u64),
    u56 (56, u64),
    u57 (57, u64),
    u58 (58, u64),
    u59 (59, u64),
    u60 (60, u64),
    u61 (61, u64),
    u62 (62, u64),
    u63 (63, u64)
);

macro_rules! byte_from_impls {
    ($($kind: ident: $super_kind: ty)*) => {
        $(
        impl $kind {
            const ARR_SIZE: usize = <$kind>::COUNT / 8;
            const SUPER_BYTES: usize = core::mem::size_of::<$super_kind>();
            pub fn from_be_bytes(bytes: [u8; Self::ARR_SIZE]) -> Self {
                let mut res_bytes = [0_u8; Self::SUPER_BYTES];
                for (set, &get) in res_bytes.iter_mut().rev().zip(bytes.iter().rev()) {
                    *set = get;
                }
                Self(<$super_kind>::from_be_bytes(res_bytes))
            }

            pub fn to_be_bytes(self) -> [u8; Self::ARR_SIZE] {
                let mut res = [0; Self::ARR_SIZE];
                let inner_bytes = self.0.to_be_bytes();
                for (&get, set) in inner_bytes.iter().rev().zip(res.iter_mut().rev()) {
                    *set = get;
                }
                res
            }

            pub fn from_le_bytes(bytes: [u8; Self::ARR_SIZE]) -> Self {
                let mut res_bytes = [0_u8; Self::SUPER_BYTES];
                for (set, &get) in res_bytes.iter_mut().zip(bytes.iter()) {
                    *set = get;
                }
                Self(<$super_kind>::from_le_bytes(res_bytes))
            }

            pub fn to_le_bytes(self) -> [u8; Self::ARR_SIZE] {
                let mut res = [0; Self::ARR_SIZE];
                let inner_bytes = self.0.to_le_bytes();
                for (&get, set) in inner_bytes.iter().zip(res.iter_mut()) {
                    *set = get;
                }
                res
            }
        }

        impl From<u8> for $kind {
            fn from(byte: u8) -> Self {
                let inner = <$super_kind>::from(byte);
                $kind(inner)
            }
        }
        )*
    };
}

byte_from_impls! {
    u24: u32
    u40: u64
    u48: u64
    u56: u64
    i24: u32
    i40: u64
    i48: u64
    i56: u64
}

impl<
        'a,
        P: Num + Bounded + ShlAssign<usize> + ShrAssign<usize> + BitCount,
        T,
        const START: usize,
        const STOP: usize,
    > GetSet<'a, P, T, START, STOP>
{
    /// Create a new [GetSet]. This should be called from methods generated by
    /// the [bit_struct] macro
    pub fn new(parent: &'a mut P) -> Self {
        Self {
            parent,
            _phantom: PhantomData::default(),
        }
    }

    /// Get a mask of STOP-START + 1 length. This doesn't use the shift left and
    /// subtract one trick because of the special case where (0b1 << (STOP -
    /// START + 1)) - 1 will cause an overflow
    fn mask(&self) -> P {
        let num_bits = P::COUNT;
        let mut max_value = P::max_value();
        let keep_bits = STOP - START + 1;

        max_value >>= num_bits - keep_bits;
        max_value
    }
}

pub unsafe trait ValidCheck<P> {
    const ALWAYS_VALID: bool = false;
    fn is_valid(_input: P) -> bool {
        true
    }
}

impl<
        'a,
        P: Num
            + Shl<usize, Output = P>
            + Shr<usize, Output = P>
            + ShlAssign<usize>
            + ShrAssign<usize>
            + Bounded
            + BitAnd<Output = P>
            + Copy
            + BitCount,
        T: ValidCheck<P>,
        const START: usize,
        const STOP: usize,
    > GetSet<'a, P, T, START, STOP>
{
    /// Get the property. Returns an error it does not exist.
    pub fn get(&self) -> T {
        let section = self.get_raw();
        unsafe { core::mem::transmute_copy(&section) }
    }

    pub fn is_valid(&self) -> bool {
        let section = self.get_raw();
        T::is_valid(section)
    }

    pub fn get_raw(&self) -> P {
        let parent = *self.parent;
        let mask = self.mask();
        (parent >> START) & mask
    }
}

impl<
        'a,
        P: Num
            + Shl<usize, Output = P>
            + Copy
            + BitOrAssign
            + BitXorAssign
            + BitAnd<Output = P>
            + ShlAssign<usize>
            + ShrAssign<usize>
            + PartialOrd
            + Bounded
            + Sized
            + BitCount,
        T,
        const START: usize,
        const STOP: usize,
    > GetSet<'a, P, T, START, STOP>
{
    /// Set the property with a core::mem::transmute_copy
    pub fn set(&mut self, value: T) {
        let value = unsafe { core::mem::transmute_copy(&value) };
        unsafe { self.set_raw(value) }
    }

    /// Set the field to a raw value.
    /// # Safety
    /// value must be a valid representation of the field. i.e.,
    /// `core::mem::transmute` between P and T must be defined.
    pub unsafe fn set_raw(&mut self, value: P) {
        let mask = self.mask();
        let mask_shifted = mask << START;

        // zero out parent
        *self.parent |= mask_shifted;
        *self.parent ^= mask_shifted;

        let to_set = value & mask;
        *self.parent |= to_set << START;
    }
}

pub trait BitStruct<const ALWAYS_VALID: bool> {
    type Kind;
    unsafe fn from_unchecked(value: Self::Kind) -> Self;
}

pub trait BitStructExt: BitStruct<true> {
    fn exact_from(value: Self::Kind) -> Self;
}

impl<T: BitStruct<true>> BitStructExt for T {
    fn exact_from(value: Self::Kind) -> Self {
        unsafe { Self::from_unchecked(value) }
    }
}

#[macro_export]
macro_rules! impl_fields {

    ($on: expr, $kind: ty =>
    [$($first_field_doc: expr),*], $head_field: ident, $head_actual: ty $(, [$($field_doc: expr),*], $field: ident, $actual: ty)*) => {
        $(#[doc=$first_field_doc])*
        pub fn $head_field(&mut self) -> bit_struct::GetSet<'_, $kind, $head_actual, {$on - <$head_actual as bit_struct::BitCount>::COUNT}, {$on - 1}> {
            bit_struct::GetSet::new(unsafe {self.0.as_ref_mut()})
        }

        bit_struct::impl_fields!($on - <$head_actual as bit_struct::BitCount>::COUNT, $kind => $([$($field_doc),*], $field, $actual),*);
    };
    ($on: expr, $kind: ty =>) => {};
}

/// Helper macro
#[macro_export]
macro_rules! bit_struct_impl {
    (
        $(#[doc = $struct_doc:expr])*
        #[derive(Default)]
        $struct_vis: vis struct $name: ident ($kind: ty) {
        $(
            $(#[doc = $field_doc:expr])*
            $field: ident: $actual: ty
        ),* $(,)?
        }
    ) => {

        bit_struct::bit_struct_impl!(
        $(#[doc = $struct_doc])*
        $struct_vis struct $name ($kind) {
        $(
            $(#[doc = $field_doc])*
            $field: $actual
        ),*
        }

        );

        impl Default for $name {
            fn default() -> Self {
                Self::of_defaults()
            }
        }

    };

    (
        $(#[doc = $struct_doc:expr])*
        $struct_vis: vis struct $name: ident ($kind: ty) {
        $(
            $(#[doc = $field_doc:expr])*
            $field: ident: $actual: ty
        ),* $(,)?
        }
    ) => {

        impl $name {

            /// Returns a valid representation for the bit_struct, where all values are
            /// the defaults. This is different than Self::default(), because the actual
            /// default implementation might not be composed of only the defaults of the
            /// given fields
            pub fn of_defaults() -> Self {
                let mut res = unsafe { Self::from_unchecked(<$kind as bit_struct::BitStructZero>::bs_zero()) };
                $(
                    res.$field().set(Default::default());
                )*
                res
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut copied = *self;
                f.debug_struct(stringify!($name))
                    $(
                        .field(stringify!($field), &copied.$field().get())
                    )*
                    .finish()
            }
        }
    };
}

pub trait BitStructZero: Zero {
    fn bs_zero() -> Self {
        Self::zero()
    }
}

impl<T: Zero> BitStructZero for T {}

// the main is actually needed

#[allow(clippy::needless_doctest_main)]
/// Create a bit struct.
///
///
/// This macro can only be used once for each module.
/// This is because the macro creates sub-module to limit access to certain
/// unsafe access. In the macro, bit-structs can be defined just like a struct
/// outside of the the macro. The catch is a **base type** must be specified.
/// Valid base types are u{8,16,32,64,128}. The elements stored in the struct
/// are statically guaranteed to not exceed the number of bits in the base type.
/// This means we cannot store a `u16` in a `u8`, but it also means we cannot
/// store 9 `u1`s in a u8.
///
/// Elements start at the top of the number (for a u16 this would be the 15th
/// bit) and progress down.
///
/// # Example
/// ```
/// bit_struct::enums! {
///     /// The default value for each enum is always the first
///     pub ThreeVariants { Zero, One, Two }
///
///     /// This is syntax to set the default value to Cat
///     pub Animal(Cat) { Cow, Bird, Cat, Dog }
///
///     pub Color { Orange, Red, Blue, Yellow, Green }
/// }
///
/// bit_struct::bit_struct! {
///     /// We can write documentation for the struct here. Here BitStruct1
///     /// derives default values from the above enums macro
///     #[derive(Default)]
///     struct BitStruct1 (u16){
///         /// a 1 bit element. This is stored in u16[15]
///         a: bit_struct::u1,
///
///         /// This is calculated to take up 2 bits. This is stored in u16[13..=14]
///         variant: ThreeVariants,
///
///         /// This also takes 2 bits. This is stored in u16[11..=12]
///         animal: Animal,
///
///         /// This takes up 3 bits. This is stored u16[8..=10]
///         color: Color,
///     }
///
///     struct BitStruct2(u32) {
///         /// We could implement for this too. Note, this does not have a default
///         a_color: Color,
///         b: bit_struct::u3,
///     }
/// }
///
/// fn main() {
///     use std::convert::TryFrom;
///     let mut bit_struct: BitStruct1 = BitStruct1::default();
///
///     assert_eq!(bit_struct.a().start(), 15);
///     assert_eq!(bit_struct.a().stop(), 15);
///
///     assert_eq!(bit_struct.color().start(), 8);
///     assert_eq!(bit_struct.color().stop(), 10);
///
///     assert_eq!(
///         format!("{:?}", bit_struct),
///         "BitStruct1 { a: 0, variant: Zero, animal: Cat, color: Orange }"
///     );
///     assert_eq!(bit_struct.raw(), 4096);
///
///     let reverse_bit_struct = BitStruct1::try_from(4096);
///     assert_eq!(
///         format!("{:?}", reverse_bit_struct),
///         "Ok(BitStruct1 { a: 0, variant: Zero, animal: Cat, color: Orange })"
///     );
///
///     // u3! macro provides a static assert that the number is not too large
///     let mut other_struct = BitStruct2::new(Color::Green, bit_struct::u3!(0b101));
///     assert_eq!(
///         format!("{:?}", other_struct),
///         "BitStruct2 { a_color: Green, b: 5 }"
///     );
///
///     assert_eq!(other_struct.a_color().get(), Color::Green);
///
///     other_struct.a_color().set(Color::Red);
///
///     assert_eq!(other_struct.a_color().get(), Color::Red);
/// }
/// ```
#[macro_export]
macro_rules! bit_struct {
    (
        $(
        $(#[doc = $struct_doc:expr])*
        $(#[derive($($struct_der: ident),+)])?
        $struct_vis: vis struct $name: ident ($kind: ty) {
        $(
            $(#[doc = $field_doc:expr])*
            $field: ident: $actual: ty
        ),* $(,)?
        }
        )*
    ) => {
        $(
        $(#[doc = $struct_doc])*
        #[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
        pub struct $name(bit_struct::UnsafeStorage<$kind>);

        impl TryFrom<$kind> for $name {
            type Error = ();
            fn try_from(elem: $kind) -> Result<$name, ()> {
                let mut res = unsafe{Self::from_unchecked(elem)};
                $(
                    if !res.$field().is_valid() {
                        return Err(());
                    }
                )*
                Ok(res)
            }
        }

        impl bit_struct::BitStruct<{$(<$actual as bit_struct::ValidCheck<$kind>>::ALWAYS_VALID &&)* true}> for $name {
            type Kind = $kind;

            unsafe fn from_unchecked(inner: $kind) -> Self {
               Self(unsafe {bit_struct::UnsafeStorage::new_unsafe(inner)})
            }
        }
        
        impl $name {

            unsafe fn from_unchecked(inner: $kind) -> Self {
               Self(unsafe {bit_struct::UnsafeStorage::new_unsafe(inner)})
            }

            #[allow(clippy::too_many_arguments)]
            pub fn new($($field: $actual),*) -> Self {
                let mut res = unsafe { Self::from_unchecked(<$kind as bit_struct::BitStructZero>::bs_zero()) };
                $(
                    res.$field().set($field);
                )*
                res
            }

            pub fn raw(self) -> $kind {
                self.0.inner()
            }

            bit_struct::impl_fields!(<$kind as bit_struct::BitCount>::COUNT, $kind => $([$($field_doc),*], $field, $actual),*);
        }

        )*

        $(
        bit_struct::bit_struct_impl!(
        $(#[doc = $struct_doc])*
        $(#[derive($($struct_der),+)])?
        $struct_vis struct $name ($kind) {
        $(
            $(#[doc = $field_doc])*
            $field: $actual
        ),*
        }

        );
        )*
    };
}

#[macro_export]
macro_rules! count_idents {
    ($on: expr, [$head: ident $(,$xs: ident)*]) => {
        bit_struct::count_idents!($on + 1, [$($xs),*])
    };
    ($on: expr, []) => {
        $on
    }
}

pub const fn bits(num: usize) -> usize {
    const fn helper(count: usize, on: usize) -> usize {
        // 0b11 = 3  log2_ceil(0b11) = 2 .. 2^2
        // 0b10 = 2 log2_ceil = 2 .. 2^1
        if on > 0 {
            helper(count + 1, on >> 1)
        } else {
            count
        }
    }

    helper(0, num)
}

/// Not meant to be directly used
#[macro_export]
macro_rules! enum_impl {
    (FROMS $name: ident: [$($kind: ty),*]) => {
        $(
        impl From<$name> for $kind {
            fn from(value: $name) -> Self {
                <$kind>::from(value as u8)
            }
        }
        )*
    };
    (VALID_CORE $name: ident: [$($kind: ty),*]) => {
        $(
        unsafe impl bit_struct::ValidCheck<$kind> for $name {

            fn is_valid(value: $kind) -> bool {
                $name::is_valid(value as u8)
            }
        }
        )*
    };
    (VALID_BIT_STRUCT $name: ident: [$($kind: ty),*]) => {
        $(
        unsafe impl bit_struct::ValidCheck<$kind> for $name {

            fn is_valid(value: $kind) -> bool {
                let inner = value.value();
                $name::is_valid(inner as u8)
            }
        }
        )*
    };
    (FROM_IMPLS $name: ident) => {
        bit_struct::enum_impl!(VALID_CORE $name: [u16, u32, u64, u128]);
        bit_struct::enum_impl!(VALID_BIT_STRUCT $name: [bit_struct::u24, bit_struct::u40, bit_struct::u48, bit_struct::u56]);
        bit_struct::enum_impl!(FROMS $name: [u8, u16, u32, u64, u128, bit_struct::u24, bit_struct::u40, bit_struct::u48, bit_struct::u56]);
    };
    (
        $(#[doc = $struct_doc:expr])*
        $(#[derive($($struct_der:ident),+)])?
        $enum_vis: vis $name: ident($default: ident){
            $(#[doc = $fst_field_doc:expr])*
            $fst_field: ident
            $(,
                $(#[doc = $field_doc:expr])*
                $field: ident
            )* $(,)?
        }
    ) => {

        #[repr(u8)]
        $(#[doc = $struct_doc])*
        $(#[derive($($struct_der),+)])?
        #[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
        $enum_vis enum $name {
            $(#[doc = $fst_field_doc])*
            $fst_field,
            $(
                $(#[doc = $field_doc])*
                $field
            ),*
        }

        unsafe impl bit_struct::BitCount for $name {
            const COUNT: usize = bit_struct::bits(bit_struct::count_idents!(0, [$($field),*]));
        }

        unsafe impl bit_struct::ValidCheck<u8> for $name {
            fn is_valid(value: u8) -> bool {
                let variants = [$name::$fst_field, $($name::$field),*];
                if (value as usize) < variants.len() {
                    true
                } else {
                    false
                }
            }
        }

        bit_struct::enum_impl!(FROM_IMPLS $name);

        impl Default for $name {
            fn default() -> Self {
                $name::$default
            }
        }

    };


    (
        $(#[doc = $struct_doc:expr])*
        $(#[derive($($struct_der:ident),+)])?
        $enum_vis: vis $name: ident {
            $(#[doc = $fst_field_doc:expr])*
            $fst_field: ident
            $(,
                $(#[doc = $field_doc:expr])*
                $field: ident
            )* $(,)?
        }
    ) => {
        #[repr(u8)]
        $(#[doc = $struct_doc])*
        $(#[derive($($struct_der),+)])?
        #[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
        $enum_vis enum $name {
            $(#[doc = $fst_field_doc])*
            $fst_field,
            $(
                $(#[doc = $field_doc])*
                $field
            ),*
        }

        impl Default for $name {
            fn default() -> Self {
                $name::$fst_field
            }
        }

        unsafe impl bit_struct::BitCount for $name {
            const COUNT: usize = bit_struct::bits(bit_struct::count_idents!(0, [$($field),*]));
        }


        unsafe impl bit_struct::ValidCheck<u8> for $name {
            fn is_valid(value: u8) -> bool {
                let variants = [$name::$fst_field, $($name::$field),*];
                if (value as usize) < variants.len() {
                    true
                } else {
                    false
                }
            }
        }

        bit_struct::enum_impl!(FROM_IMPLS $name);
    };
}

/// Create enums which have convenient TryFrom/From implementations. This makes
/// using them with the [bit_struct] macro much easier.
#[macro_export]
macro_rules! enums {
    (
        $(
        $(#[doc = $struct_doc:expr])*
        $(#[derive($($struct_der:ident),+)])?
        $enum_vis: vis $name: ident $(($enum_default: ident))? {
            $(#[doc = $fst_field_doc:expr])*
            $fst_field: ident
            $(,
                $(#[doc = $field_doc:expr])*
                $field: ident
            )* $(,)?
        }
        )+
    ) => {
        $(
        bit_struct::enum_impl!(
        $(#[doc = $struct_doc])*
        $(#[derive($($struct_der),+)])?
        $enum_vis $name $(($enum_default))?{
            $(#[doc = $fst_field_doc])*
            $fst_field
            $(,
                $(#[doc = $field_doc])*
                $field
            )*
        }
        );
        )+
    }
}
