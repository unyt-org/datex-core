//! This module contains the implementation of the [Integer] struct, which represents an arbitrary precision integer value in type system.

use core::result::Result;
pub mod typed_integer;
pub mod utils;
use crate::prelude::*;

use crate::values::core_values::{
    error::NumberParseError, integer::typed_integer::TypedInteger,
};
pub mod equality;
pub mod ops;
pub mod serde_dif;
use num::{BigInt, Num, ToPrimitive};
mod to_instructions;
use core::{fmt::Display, hash::Hash, str::FromStr};
use num_integer::Integer as NumInteger;
use serde::Deserialize;
pub mod binrw;
pub mod classification;
mod convert_parts;
mod datex_hash;
mod datex_native;
pub mod datex_native_structural;
mod get_core_lib_type_id;
mod get_datex_type;
pub mod primitive;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
pub mod update_handler;
mod value_access;

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Eq)]
pub struct Integer(pub BigInt);

impl From<&i8> for Integer {
    fn from(value: &i8) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&i16> for Integer {
    fn from(value: &i16) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&i32> for Integer {
    fn from(value: &i32) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&i64> for Integer {
    fn from(value: &i64) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&i128> for Integer {
    fn from(value: &i128) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&u8> for Integer {
    fn from(value: &u8) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&u16> for Integer {
    fn from(value: &u16) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&u32> for Integer {
    fn from(value: &u32) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&u64> for Integer {
    fn from(value: &u64) -> Self {
        Integer(BigInt::from(*value))
    }
}
impl From<&u128> for Integer {
    fn from(value: &u128) -> Self {
        Integer(BigInt::from(*value))
    }
}

impl<'de> Deserialize<'de> for Integer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Integer::try_from_string(&s).map_err(serde::de::Error::custom)
    }
}

impl Integer {
    pub fn new<T: Into<BigInt>>(value: T) -> Self {
        Integer(value.into())
    }
    /// Parse an integer from a string in base 10.
    /// Returns an error if the string is not a valid integer.
    pub fn try_from_string(s: &str) -> Result<Self, NumberParseError> {
        BigInt::from_str(s)
            .map(Integer)
            .map_err(|_| NumberParseError::InvalidFormat)
    }

    /// Parse an integer from a string in the given radix (base).
    /// Returns an error if the string is not a valid integer in the given radix.
    pub fn from_string_radix(
        s: &str,
        radix: u32,
    ) -> Result<Self, NumberParseError> {
        BigInt::from_str_radix(s, radix)
            .map(Integer)
            .map_err(|_| NumberParseError::InvalidFormat)
    }

    /// Returns true if the integer is zero.
    pub fn is_zero(&self) -> bool {
        self.0 == BigInt::ZERO
    }

    /// Returns true if the integer is negative.
    /// Note that zero is neither positive nor negative.
    pub fn is_negative(&self) -> bool {
        self.0.sign() == num::bigint::Sign::Minus
    }
    /// Returns true if the integer is positive.
    /// Note that zero is neither positive nor negative.
    pub fn is_positive(&self) -> bool {
        self.0.sign() == num::bigint::Sign::Plus
    }

    /// Converts the integer to an i8 if it fits, otherwise returns None.
    pub fn as_i8(&self) -> Option<i8> {
        self.0.to_i8()
    }

    /// Converts the integer to a u8 if it fits, otherwise returns None.
    pub fn as_u8(&self) -> Option<u8> {
        self.0.to_u8()
    }

    /// Converts the integer to an i16 if it fits, otherwise returns None.
    pub fn as_i16(&self) -> Option<i16> {
        self.0.to_i16()
    }

    /// Converts the integer to a u16 if it fits, otherwise returns None.
    pub fn as_u16(&self) -> Option<u16> {
        self.0.to_u16()
    }

    /// Converts the integer to an i32 if it fits, otherwise returns None.
    pub fn as_i32(&self) -> Option<i32> {
        self.0.to_i32()
    }

    /// Converts the integer to a u32 if it fits, otherwise returns None.
    pub fn as_u32(&self) -> Option<u32> {
        self.0.to_u32()
    }

    /// Converts the integer to an i64 if it fits, otherwise returns None.
    pub fn as_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }

    /// Converts the integer to a usize if it fits, otherwise returns None.
    pub fn as_usize(&self) -> Option<usize> {
        self.0.to_usize()
    }

    /// Converts the integer to a u64 if it fits, otherwise returns None.
    pub fn as_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }

    /// Converts the integer to an i128 if it fits, otherwise returns None.
    pub fn as_i128(&self) -> Option<i128> {
        self.0.to_i128()
    }

    /// Converts the integer to a u128 if it fits, otherwise returns None.
    pub fn as_u128(&self) -> Option<u128> {
        self.0.to_u128()
    }

    pub fn as_f32(&self) -> f32 {
        unsafe { self.0.to_f32().unwrap_unchecked() } // Note: this is always Some for BigInt
    }

    pub fn as_f64(&self) -> f64 {
        unsafe { self.0.to_f64().unwrap_unchecked() } // Note: this is always Some for BigInt
    }

    // TODO #722: this can be optimized and redundant code can be reduced
    /// Converts the integer to an i8, wrapping on overflow.
    pub fn as_wrapped_i8(&self) -> i8 {
        const MAX: i16 = u8::MAX as i16 + 1;
        let i16 = self.0.mod_floor(&BigInt::from(MAX)).to_i16().unwrap();
        if i16 > MAX {
            (i16 - MAX) as i8
        } else {
            i16 as i8
        }
    }

    /// Converts the integer to a u8, wrapping on overflow.
    pub fn as_wrapped_u8(&self) -> u8 {
        self.0
            .mod_floor(&(BigInt::from(u8::MAX as u16 + 1)))
            .to_u8()
            .unwrap()
    }

    pub fn as_wrapped_i16(&self) -> i16 {
        const MAX: i32 = u16::MAX as i32 + 1;
        let i32 = self.0.mod_floor(&BigInt::from(MAX)).to_i32().unwrap();
        if i32 > MAX {
            (i32 - MAX) as i16
        } else {
            i32 as i16
        }
    }

    pub fn as_wrapped_u16(&self) -> u16 {
        self.0
            .mod_floor(&BigInt::from(u16::MAX as u32 + 1))
            .to_u16()
            .unwrap()
    }

    pub fn as_wrapped_i32(&self) -> i32 {
        const MAX: i64 = u32::MAX as i64 + 1;
        let i64 = self.0.mod_floor(&BigInt::from(MAX)).to_i64().unwrap();
        if i64 > MAX {
            (i64 - MAX) as i32
        } else {
            i64 as i32
        }
    }

    pub fn as_wrapped_u32(&self) -> u32 {
        self.0
            .mod_floor(&BigInt::from(u32::MAX as u64 + 1))
            .to_u32()
            .unwrap()
    }

    pub fn as_wrapped_i64(&self) -> i64 {
        const MAX: i128 = u64::MAX as i128 + 1;
        let i128 = self.0.mod_floor(&BigInt::from(MAX)).to_i128().unwrap();
        if i128 > MAX {
            (i128 - MAX) as i64
        } else {
            i128 as i64
        }
    }

    pub fn as_wrapped_u64(&self) -> u64 {
        self.0
            .mod_floor(&BigInt::from(u64::MAX as u128 + 1))
            .to_u64()
            .unwrap()
    }

    pub fn as_wrapped_i128(&self) -> i128 {
        let max: BigInt = BigInt::from(u128::MAX) + BigInt::from(1);
        let i256 = self.0.mod_floor(&max);
        if i256 > max {
            (i256 - max).to_i128().unwrap()
        } else {
            i256.to_i128().unwrap()
        }
    }

    pub fn as_wrapped_u128(&self) -> u128 {
        self.0
            .mod_floor(&(BigInt::from(u128::MAX) + BigInt::from(1)))
            .to_u128()
            .unwrap()
    }

    /// Converts the integer to the smallest fitting TypedInteger variant.
    /// If it doesn't fit in any smaller type, returns TypedInteger::Big.
    pub fn to_smallest_fitting(&self) -> TypedInteger {
        if let Some(i) = self.as_i8() {
            return TypedInteger::I8(i);
        }
        if let Some(u) = self.as_u8() {
            return TypedInteger::U8(u);
        }
        if let Some(i) = self.as_i16() {
            return TypedInteger::I16(i);
        }
        if let Some(u) = self.as_u16() {
            return TypedInteger::U16(u);
        }
        if let Some(i) = self.as_i32() {
            return TypedInteger::I32(i);
        }
        if let Some(u) = self.as_u32() {
            return TypedInteger::U32(u);
        }
        if let Some(i) = self.as_i64() {
            return TypedInteger::I64(i);
        }
        if let Some(u) = self.as_u64() {
            return TypedInteger::U64(u);
        }
        if let Some(i) = self.as_i128() {
            return TypedInteger::I128(i);
        }
        if let Some(u) = self.as_u128() {
            return TypedInteger::U128(u);
        }

        // If no smaller fitting type is found, return BigInt
        TypedInteger::IBig(self.clone())
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "{}", self.0)
    }
}

impl From<i8> for Integer {
    fn from(value: i8) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<u8> for Integer {
    fn from(value: u8) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<i16> for Integer {
    fn from(value: i16) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<u16> for Integer {
    fn from(value: u16) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<i32> for Integer {
    fn from(value: i32) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<u32> for Integer {
    fn from(value: u32) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<i128> for Integer {
    fn from(value: i128) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<u128> for Integer {
    fn from(value: u128) -> Self {
        Integer(BigInt::from(value))
    }
}
impl From<BigInt> for Integer {
    fn from(value: BigInt) -> Self {
        Integer(value)
    }
}

impl From<TypedInteger> for Integer {
    fn from(value: TypedInteger) -> Self {
        match value {
            TypedInteger::I8(v) => Integer::from(v),
            TypedInteger::U8(v) => Integer::from(v),
            TypedInteger::I16(v) => Integer::from(v),
            TypedInteger::U16(v) => Integer::from(v),
            TypedInteger::I32(v) => Integer::from(v),
            TypedInteger::U32(v) => Integer::from(v),
            TypedInteger::I64(v) => Integer::from(v),
            TypedInteger::U64(v) => Integer::from(v),
            TypedInteger::I128(v) => Integer::from(v),
            TypedInteger::U128(v) => Integer::from(v),
            TypedInteger::IBig(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_addition() {
        let dec1 = Integer::try_from_string("12").unwrap();
        let dec2 = Integer::try_from_string("56").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "68");

        let dec1 = Integer::try_from_string("-12345").unwrap();
        let dec2 = Integer::try_from_string("3").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "-12342");
    }

    #[test]
    fn formatting() {
        let int1 = Integer::try_from_string("12").unwrap();
        assert_eq!(int1.to_string(), "12");

        let int2 = Integer::try_from_string("-12345").unwrap();
        assert_eq!(int2.to_string(), "-12345");
        let int3 = Integer::try_from_string("0").unwrap();
        assert_eq!(int3.to_string(), "0");

        let int4 =
            Integer::try_from_string("123456789012345678901234567890").unwrap();
        assert_eq!(int4.to_string(), "123456789012345678901234567890");

        let int5 = Integer::try_from_string("-123456789012345678901234567890")
            .unwrap();
        assert_eq!(int5.to_string(), "-123456789012345678901234567890");
    }
}
