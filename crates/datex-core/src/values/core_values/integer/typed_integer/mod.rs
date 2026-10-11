//! This module contains the implementation of the [TypedInteger] enum, which represents a typed integer value in the DATEX type system.
//! It supports various integer types, including signed and unsigned integers of different sizes, as well as an arbitrary precision integer type (IBig).
mod to_instructions;
use crate::values::core_values::{
    error::NumberParseError,
    integer::{
        Integer,
        utils::{smallest_fitting_signed, smallest_fitting_unsigned},
    },
};
pub mod equality;
pub mod update_handler;
use crate::{
    libs::core::type_id::CoreLibTypeId, prelude::*,
    traits::get_core_lib_type_id::GetCoreLibTypeId,
};
use ::binrw::{BinRead, BinWrite};
use core::{fmt::Display, hash::Hash, result::Result, unreachable};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use strum::Display;
use strum_macros::{AsRefStr, EnumIter, EnumString};

mod classification;
mod convert_parts;
mod datex_hash;
mod datex_native;
mod datex_native_structural;
mod get_core_lib_type_id;
mod get_datex_type;
pub mod primitive;
pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod try_clone;
mod value_access;

/// The integer type variants to be used as a inline
/// definition in DATEX (such as 42u32 or -42i64).
/// Note that changing the enum variants will change
/// the way integers are parsed in DATEX scripts.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    EnumIter,
    AsRefStr,
    IntoPrimitive,
    TryFromPrimitive,
    Serialize,
    Deserialize,
    Display,
    Default,
)]
#[repr(u8)]
#[strum(serialize_all = "lowercase")]
pub enum IntegerTypeVariant {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    #[default]
    I32,
    I64,
    I128,
    IBig,
    // TODO #610: ubig?
}

#[derive(Debug, Clone, Eq, BinRead, BinWrite)]
pub enum TypedInteger {
    #[brw(magic = 0u8)]
    IBig(Integer),
    #[brw(magic = 1u8)]
    I8(i8),
    #[brw(magic = 2u8)]
    I16(i16),
    #[brw(magic = 3u8)]
    I32(i32),
    #[brw(magic = 4u8)]
    I64(i64),
    #[brw(magic = 5u8)]
    I128(i128),
    #[brw(magic = 6u8)]
    U8(u8),
    #[brw(magic = 7u8)]
    U16(u16),
    #[brw(magic = 8u8)]
    U32(u32),
    #[brw(magic = 9u8)]
    U64(u64),
    #[brw(magic = 10u8)]
    U128(u128),
}

impl From<&TypedInteger> for CoreLibTypeId {
    fn from(value: &TypedInteger) -> Self {
        value.core_lib_type_id()
    }
}

impl Hash for TypedInteger {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypedInteger::IBig(v) => v.hash(state),
            TypedInteger::I8(v) => v.hash(state),
            TypedInteger::I16(v) => v.hash(state),
            TypedInteger::I32(v) => v.hash(state),
            TypedInteger::I64(v) => v.hash(state),
            TypedInteger::I128(v) => v.hash(state),
            TypedInteger::U8(v) => v.hash(state),
            TypedInteger::U16(v) => v.hash(state),
            TypedInteger::U32(v) => v.hash(state),
            TypedInteger::U64(v) => v.hash(state),
            TypedInteger::U128(v) => v.hash(state),
        }
    }
}

impl TypedInteger {
    // TODO #342: add from_integer_with_variant

    /// Parses a string into a TypedInteger with the given variant.
    /// If the string is not a valid integer, returns an error.
    pub fn try_from_string_and_variant(
        s: &str,
        variant: IntegerTypeVariant,
    ) -> Result<TypedInteger, NumberParseError> {
        Self::try_from_string_radix_with_variant(s, 10, variant)
    }

    /// Parses a string into a TypedInteger with the given variant and radix.
    /// If the string is not a valid integer, returns an error.
    pub fn try_from_string_radix_with_variant(
        s: &str,
        radix: u32,
        variant: IntegerTypeVariant,
    ) -> Result<TypedInteger, NumberParseError> {
        if core::matches!(variant, IntegerTypeVariant::IBig) {
            return Ok(TypedInteger::IBig(Integer::from_string_radix(
                s, radix,
            )?));
        }

        match variant {
            IntegerTypeVariant::U8 => {
                u8::from_str_radix(s, radix).map(TypedInteger::U8)
            }
            IntegerTypeVariant::U16 => {
                u16::from_str_radix(s, radix).map(TypedInteger::U16)
            }
            IntegerTypeVariant::U32 => {
                u32::from_str_radix(s, radix).map(TypedInteger::U32)
            }
            IntegerTypeVariant::U64 => {
                u64::from_str_radix(s, radix).map(TypedInteger::U64)
            }
            IntegerTypeVariant::U128 => {
                u128::from_str_radix(s, radix).map(TypedInteger::U128)
            }
            IntegerTypeVariant::I8 => {
                i8::from_str_radix(s, radix).map(TypedInteger::I8)
            }
            IntegerTypeVariant::I16 => {
                i16::from_str_radix(s, radix).map(TypedInteger::I16)
            }
            IntegerTypeVariant::I32 => {
                i32::from_str_radix(s, radix).map(TypedInteger::I32)
            }
            IntegerTypeVariant::I64 => {
                i64::from_str_radix(s, radix).map(TypedInteger::I64)
            }
            IntegerTypeVariant::I128 => {
                i128::from_str_radix(s, radix).map(TypedInteger::I128)
            }
            _ => unreachable!(""),
        }
        .map_err(|e| match e.kind() {
            core::num::IntErrorKind::Zero
            | core::num::IntErrorKind::Empty
            | core::num::IntErrorKind::InvalidDigit => {
                NumberParseError::InvalidFormat
            }
            core::num::IntErrorKind::PosOverflow
            | core::num::IntErrorKind::NegOverflow => {
                NumberParseError::OutOfRange
            }
            _ => core::panic!("Unhandled integer parse error: {:?}", e.kind()),
        })
    }

    /// Converts the integer to the smallest fitting TypedInteger variant.
    pub fn to_smallest_fitting(&self) -> TypedInteger {
        if self.is_unsigned_variant()
            && let Some(u128) = self.as_u128()
        {
            smallest_fitting_unsigned(u128)
        } else if let Some(i128) = self.as_i128() {
            smallest_fitting_signed(i128)
        } else {
            self.clone()
        }
    }

    /// Converts the TypedInteger to an Integer (big).
    pub fn as_integer(&self) -> Integer {
        match self {
            TypedInteger::IBig(v) => v.clone(),
            TypedInteger::I8(v) => Integer::from(*v),
            TypedInteger::I16(v) => Integer::from(*v),
            TypedInteger::I32(v) => Integer::from(*v),
            TypedInteger::I64(v) => Integer::from(*v),
            TypedInteger::I128(v) => Integer::from(*v),
            TypedInteger::U8(v) => Integer::from(*v),
            TypedInteger::U16(v) => Integer::from(*v),
            TypedInteger::U32(v) => Integer::from(*v),
            TypedInteger::U64(v) => Integer::from(*v),
            TypedInteger::U128(v) => Integer::from(*v),
        }
    }

    /// Converts the integer to an i8 if it fits, otherwise returns None.
    pub fn as_i8(&self) -> Option<i8> {
        match self {
            TypedInteger::I8(v) => Some(*v),
            TypedInteger::I16(v) => i8::try_from(*v).ok(),
            TypedInteger::I32(v) => i8::try_from(*v).ok(),
            TypedInteger::I64(v) => i8::try_from(*v).ok(),
            TypedInteger::I128(v) => i8::try_from(*v).ok(),

            TypedInteger::U8(v) => i8::try_from(*v).ok(),
            TypedInteger::U16(v) => i8::try_from(*v).ok(),
            TypedInteger::U32(v) => i8::try_from(*v).ok(),
            TypedInteger::U64(v) => i8::try_from(*v).ok(),
            TypedInteger::U128(v) => i8::try_from(*v).ok(),

            TypedInteger::IBig(v) => v.as_i8(),
        }
    }

    /// Converts the integer to an i16 if it fits, otherwise returns None.
    pub fn as_i16(&self) -> Option<i16> {
        match self {
            TypedInteger::I8(v) => Some(i16::from(*v)),
            TypedInteger::I16(v) => Some(*v),
            TypedInteger::I32(v) => i16::try_from(*v).ok(),
            TypedInteger::I64(v) => i16::try_from(*v).ok(),
            TypedInteger::I128(v) => i16::try_from(*v).ok(),

            TypedInteger::U8(v) => Some(i16::from(*v)),
            TypedInteger::U16(v) => i16::try_from(*v).ok(),
            TypedInteger::U32(v) => i16::try_from(*v).ok(),
            TypedInteger::U64(v) => i16::try_from(*v).ok(),
            TypedInteger::U128(v) => i16::try_from(*v).ok(),
            TypedInteger::IBig(v) => v.as_i16(),
        }
    }

    /// Converts the integer to an i32 if it fits, otherwise returns None.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            TypedInteger::I8(v) => Some(i32::from(*v)),
            TypedInteger::I16(v) => Some(i32::from(*v)),
            TypedInteger::I32(v) => Some(*v),
            TypedInteger::I64(v) => i32::try_from(*v).ok(),
            TypedInteger::I128(v) => i32::try_from(*v).ok(),

            TypedInteger::U8(v) => Some(i32::from(*v)),
            TypedInteger::U16(v) => Some(i32::from(*v)),
            TypedInteger::U32(v) => i32::try_from(*v).ok(),
            TypedInteger::U64(v) => i32::try_from(*v).ok(),
            TypedInteger::U128(v) => i32::try_from(*v).ok(),
            TypedInteger::IBig(v) => v.as_i32(),
        }
    }

    /// Converts the integer to an i64 if it fits, otherwise returns None.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TypedInteger::I8(v) => Some(i64::from(*v)),
            TypedInteger::I16(v) => Some(i64::from(*v)),
            TypedInteger::I32(v) => Some(i64::from(*v)),
            TypedInteger::I64(v) => Some(*v),
            TypedInteger::I128(v) => i64::try_from(*v).ok(),

            TypedInteger::U8(v) => Some(i64::from(*v)),
            TypedInteger::U16(v) => Some(i64::from(*v)),
            TypedInteger::U32(v) => Some(i64::from(*v)),
            TypedInteger::U64(v) => i64::try_from(*v).ok(),
            TypedInteger::U128(v) => i64::try_from(*v).ok(),
            TypedInteger::IBig(v) => v.as_i64(),
        }
    }

    // FIXME #125 we should probably allow casting to i128 from u128
    pub fn as_u128(&self) -> Option<u128> {
        match self {
            TypedInteger::U8(v) => Some(*v as u128),
            TypedInteger::U16(v) => Some(*v as u128),
            TypedInteger::U32(v) => Some(*v as u128),
            TypedInteger::U64(v) => Some(*v as u128),
            TypedInteger::U128(v) => Some(*v),
            TypedInteger::IBig(v) => v.as_u128(),
            _ => unreachable!("as_u128 called on a signed integer"),
        }
    }

    /// Converts the integer to an i128 if it fits, otherwise returns None.
    pub fn as_i128(&self) -> Option<i128> {
        match self {
            TypedInteger::I8(v) => Some(*v as i128),
            TypedInteger::I16(v) => Some(*v as i128),
            TypedInteger::I32(v) => Some(*v as i128),
            TypedInteger::I64(v) => Some(*v as i128),
            TypedInteger::I128(v) => Some(*v),
            TypedInteger::U8(v) => Some(*v as i128),
            TypedInteger::U16(v) => Some(*v as i128),
            TypedInteger::U32(v) => Some(*v as i128),
            TypedInteger::U64(v) => Some(*v as i128),
            TypedInteger::U128(v) => Some(i128::try_from(*v).ok()?),
            TypedInteger::IBig(v) => v.as_i128(),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self {
            TypedInteger::IBig(v) => v.as_f32(),
            TypedInteger::I8(v) => *v as f32,
            TypedInteger::I16(v) => *v as f32,
            TypedInteger::I32(v) => *v as f32,
            TypedInteger::I64(v) => *v as f32,
            TypedInteger::I128(v) => *v as f32,
            TypedInteger::U8(v) => *v as f32,
            TypedInteger::U16(v) => *v as f32,
            TypedInteger::U32(v) => *v as f32,
            TypedInteger::U64(v) => *v as f32,
            TypedInteger::U128(v) => *v as f32,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            TypedInteger::IBig(v) => v.as_f64(),
            TypedInteger::I8(v) => *v as f64,
            TypedInteger::I16(v) => *v as f64,
            TypedInteger::I32(v) => *v as f64,
            TypedInteger::I64(v) => *v as f64,
            TypedInteger::I128(v) => *v as f64,
            TypedInteger::U8(v) => *v as f64,
            TypedInteger::U16(v) => *v as f64,
            TypedInteger::U32(v) => *v as f64,
            TypedInteger::U64(v) => *v as f64,
            TypedInteger::U128(v) => *v as f64,
        }
    }

    /// Converts the integer to a usize if it fits, otherwise returns None.
    pub fn as_usize(&self) -> Option<usize> {
        self.as_u128().and_then(|v| usize::try_from(v).ok())
    }

    pub fn borrow_as_u8(&self) -> Option<&u8> {
        match self {
            TypedInteger::U8(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_u8(&mut self) -> Option<&mut u8> {
        match self {
            TypedInteger::U8(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_u16(&self) -> Option<&u16> {
        match self {
            TypedInteger::U16(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_u16(&mut self) -> Option<&mut u16> {
        match self {
            TypedInteger::U16(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_u32(&self) -> Option<&u32> {
        match self {
            TypedInteger::U32(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_u32(&mut self) -> Option<&mut u32> {
        match self {
            TypedInteger::U32(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_u64(&self) -> Option<&u64> {
        match self {
            TypedInteger::U64(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_u64(&mut self) -> Option<&mut u64> {
        match self {
            TypedInteger::U64(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_u128(&self) -> Option<&u128> {
        match self {
            TypedInteger::U128(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_mut_as_u128(&mut self) -> Option<&mut u128> {
        match self {
            TypedInteger::U128(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_i8(&self) -> Option<&i8> {
        match self {
            TypedInteger::I8(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_i8(&mut self) -> Option<&mut i8> {
        match self {
            TypedInteger::I8(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_i16(&self) -> Option<&i16> {
        match self {
            TypedInteger::I16(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_i16(&mut self) -> Option<&mut i16> {
        match self {
            TypedInteger::I16(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_i32(&self) -> Option<&i32> {
        match self {
            TypedInteger::I32(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_i32(&mut self) -> Option<&mut i32> {
        match self {
            TypedInteger::I32(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_i64(&self) -> Option<&i64> {
        match self {
            TypedInteger::I64(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_i64(&mut self) -> Option<&mut i64> {
        match self {
            TypedInteger::I64(v) => Some(v),
            _ => None,
        }
    }

    pub fn borrow_as_i128(&self) -> Option<&i128> {
        match self {
            TypedInteger::I128(v) => Some(v),
            _ => None,
        }
    }
    pub fn borrow_mut_as_i128(&mut self) -> Option<&mut i128> {
        match self {
            TypedInteger::I128(v) => Some(v),
            _ => None,
        }
    }

    /// Returns true if the integer is of a signed type.
    pub fn is_signed_variant(&self) -> bool {
        if let TypedInteger::IBig(_) = self {
            return true;
        }
        core::matches!(
            self,
            TypedInteger::I8(_)
                | TypedInteger::I16(_)
                | TypedInteger::I32(_)
                | TypedInteger::I64(_)
                | TypedInteger::I128(_)
        )
    }

    pub fn is_zero(&self) -> bool {
        match self {
            TypedInteger::IBig(v) => v.is_zero(),
            TypedInteger::I8(v) => *v == 0,
            TypedInteger::I16(v) => *v == 0,
            TypedInteger::I32(v) => *v == 0,
            TypedInteger::I64(v) => *v == 0,
            TypedInteger::I128(v) => *v == 0,
            TypedInteger::U8(v) => *v == 0,
            TypedInteger::U16(v) => *v == 0,
            TypedInteger::U32(v) => *v == 0,
            TypedInteger::U64(v) => *v == 0,
            TypedInteger::U128(v) => *v == 0,
        }
    }

    /// Returns true if the integer is of an unsigned type.
    pub fn is_unsigned_variant(&self) -> bool {
        !self.is_signed_variant()
    }

    /// Returns true if the integer is positive.
    /// Zero is not considered positive.
    pub fn is_positive(&self) -> bool {
        if let TypedInteger::IBig(v) = self {
            return v.is_positive();
        }
        let v = self.as_i128().unwrap();
        v > 0
    }

    /// Returns true if the integer is negative.
    /// Zero is not considered negative.
    pub fn is_negative(&self) -> bool {
        if let TypedInteger::IBig(v) = self {
            return v.is_negative();
        }
        let v = self.as_i128().unwrap();
        v < 0
    }

    pub fn variant(&self) -> IntegerTypeVariant {
        match self {
            TypedInteger::IBig(_) => IntegerTypeVariant::IBig,
            TypedInteger::I8(_) => IntegerTypeVariant::I8,
            TypedInteger::I16(_) => IntegerTypeVariant::I16,
            TypedInteger::I32(_) => IntegerTypeVariant::I32,
            TypedInteger::I64(_) => IntegerTypeVariant::I64,
            TypedInteger::I128(_) => IntegerTypeVariant::I128,
            TypedInteger::U8(_) => IntegerTypeVariant::U8,
            TypedInteger::U16(_) => IntegerTypeVariant::U16,
            TypedInteger::U32(_) => IntegerTypeVariant::U32,
            TypedInteger::U64(_) => IntegerTypeVariant::U64,
            TypedInteger::U128(_) => IntegerTypeVariant::U128,
        }
    }

    pub fn to_string_with_suffix(&self) -> String {
        match self {
            TypedInteger::I8(v) => format!("{v}i8"),
            TypedInteger::I16(v) => format!("{v}i16"),
            TypedInteger::I32(v) => format!("{v}i32"),
            TypedInteger::I64(v) => format!("{v}i64"),
            TypedInteger::I128(v) => format!("{v}i128"),
            TypedInteger::U8(v) => format!("{v}u8"),
            TypedInteger::U16(v) => format!("{v}u16"),
            TypedInteger::U32(v) => format!("{v}u32"),
            TypedInteger::U64(v) => format!("{v}u64"),
            TypedInteger::U128(v) => format!("{v}u128"),
            TypedInteger::IBig(v) => format!("{v}ibig"),
        }
    }
}

impl Display for TypedInteger {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            TypedInteger::I8(v) => core::write!(f, "{v}"),
            TypedInteger::I16(v) => core::write!(f, "{v}"),
            TypedInteger::I32(v) => core::write!(f, "{v}"),
            TypedInteger::I64(v) => core::write!(f, "{v}"),
            TypedInteger::I128(v) => core::write!(f, "{v}"),
            TypedInteger::U8(v) => core::write!(f, "{v}"),
            TypedInteger::U16(v) => core::write!(f, "{v}"),
            TypedInteger::U32(v) => core::write!(f, "{v}"),
            TypedInteger::U64(v) => core::write!(f, "{v}"),
            TypedInteger::U128(v) => core::write!(f, "{v}"),
            TypedInteger::IBig(v) => core::write!(f, "{v}"),
        }
    }
}

impl From<i8> for TypedInteger {
    fn from(v: i8) -> Self {
        TypedInteger::I8(v)
    }
}
impl From<i16> for TypedInteger {
    fn from(v: i16) -> Self {
        TypedInteger::I16(v)
    }
}
impl From<i32> for TypedInteger {
    fn from(v: i32) -> Self {
        TypedInteger::I32(v)
    }
}
impl From<i64> for TypedInteger {
    fn from(v: i64) -> Self {
        TypedInteger::I64(v)
    }
}
impl From<i128> for TypedInteger {
    fn from(v: i128) -> Self {
        TypedInteger::I128(v)
    }
}
impl From<u8> for TypedInteger {
    fn from(v: u8) -> Self {
        TypedInteger::U8(v)
    }
}
impl From<u16> for TypedInteger {
    fn from(v: u16) -> Self {
        TypedInteger::U16(v)
    }
}
impl From<u32> for TypedInteger {
    fn from(v: u32) -> Self {
        TypedInteger::U32(v)
    }
}
impl From<u64> for TypedInteger {
    fn from(v: u64) -> Self {
        TypedInteger::U64(v)
    }
}
impl From<u128> for TypedInteger {
    fn from(v: u128) -> Self {
        TypedInteger::U128(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_integer_addition() {
        let a = TypedInteger::I8(10);
        let b = TypedInteger::I8(20);

        let result = a.clone() + b;
        assert_eq!(result, Some(TypedInteger::I8(30)));

        let c = TypedInteger::U8(10);
        let result = a.clone() + c.clone();
        assert_eq!(result, Some(TypedInteger::I8(20)));

        let result = c + a;
        assert_eq!(result, Some(TypedInteger::U8(20)));

        // out of bounds
        let d = TypedInteger::I8(100);
        let e = TypedInteger::I8(50);
        let result = d + e;
        assert_eq!(result, None);
    }

    #[test]
    fn typed_integer_subtraction() {
        let a = TypedInteger::I8(30);
        let b = TypedInteger::I8(20);
        let result = a - b;
        assert_eq!(result, Some(TypedInteger::I8(10)));

        // negative result
        let c = TypedInteger::I8(20);
        let d = TypedInteger::I8(30);
        let result = c - d;
        assert_eq!(result, Some(TypedInteger::I8(-10)));

        // out of bounds
        let e = TypedInteger::I8(-100);
        let f = TypedInteger::I8(50);
        let result = e - f;
        assert_eq!(result, None);

        let g = TypedInteger::U8(30);
        let h = TypedInteger::I8(30);
        let result = g - h;
        assert_eq!(result, Some(TypedInteger::U8(0)));

        let h = TypedInteger::U8(30);
        let i = TypedInteger::I8(31);

        let result = h - i;
        assert_eq!(result, None);
    }

    #[test]
    fn integer_addition() {
        let a = TypedInteger::from(10_i8);
        let b = TypedInteger::from(20_i8);
        let result = a + b;
        assert_eq!(result, Some(TypedInteger::I8(30_i8)));
    }

    #[test]
    fn integer() {
        let a = TypedInteger::from(1_i8);
        assert_eq!(a, TypedInteger::I8(1));

        let b = TypedInteger::from(1_u8);
        assert_eq!(b, TypedInteger::U8(1));

        let c = TypedInteger::from(1_i16);
        assert_eq!(c, TypedInteger::I16(1));

        let d = TypedInteger::from(1_u16);
        assert_eq!(d, TypedInteger::U16(1));

        let e = TypedInteger::from(1_i32);
        assert_eq!(e, TypedInteger::I32(1));

        let f = TypedInteger::from(1_u32);
        assert_eq!(f, TypedInteger::U32(1));

        let g = TypedInteger::from(1_i64);
        assert_eq!(g, TypedInteger::I64(1));

        let h = TypedInteger::from(1_u64);
        assert_eq!(h, TypedInteger::U64(1));

        let i = TypedInteger::from(1_i128);
        assert_eq!(i, TypedInteger::I128(1));

        let j = TypedInteger::from(1_u128);
        assert_eq!(j, TypedInteger::U128(1));

        assert_eq!(a.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(b.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(c.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(d.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(e.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(f.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(g.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(h.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(i.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(j.to_smallest_fitting(), TypedInteger::U8(1));
    }
}
