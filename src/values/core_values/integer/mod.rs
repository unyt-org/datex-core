pub mod typed_integer;
pub mod utils;

use crate::traits::structural_eq::StructuralEq;
use crate::values::core_values::{
    error::NumberParseError, integer::typed_integer::TypedInteger,
};
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use num::{BigInt, Num};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    hash::Hash,
    io::{Read, Seek},
    ops::{Add, Neg, Sub},
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Integer(pub BigInt);

impl Serialize for Integer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_str_radix(10))
    }
}

impl<'de> Deserialize<'de> for Integer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Integer::from_string(&s).map_err(serde::de::Error::custom)
    }
}

impl Integer {
    /// Parse an integer from a string in base 10.
    /// Returns an error if the string is not a valid integer.
    pub fn from_string(s: &str) -> Result<Self, NumberParseError> {
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
        TypedInteger::Big(self.clone())
    }
}

impl StructuralEq for Integer {
    fn structural_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Neg for Integer {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Integer(-self.0)
    }
}

impl Add for Integer {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Integer(self.0 + rhs.0)
    }
}
impl Add for &Integer {
    type Output = Integer;

    fn add(self, rhs: Self) -> Self::Output {
        Integer::add(self.clone(), rhs.clone())
    }
}

impl Sub for Integer {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Sub for &Integer {
    type Output = Integer;

    fn sub(self, rhs: Self) -> Self::Output {
        Integer::sub(self.clone(), rhs.clone())
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl BinWrite for Integer {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + Seek>(
        &self,
        writer: &mut W,
        _endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        let (sign, bytes) = self.0.to_bytes_be();
        let len = bytes.len() as u32;
        writer.write_all(&[sign as u8])?;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&bytes)?;

        Ok(())
    }
}
impl BinRead for Integer {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        let sign = reader.read_le::<u8>()?;
        let len = reader.read_le::<u32>()? as usize;
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes)?;

        let big_int = BigInt::from_bytes_be(
            match sign {
                0 => num::bigint::Sign::Minus,
                1 => num::bigint::Sign::NoSign,
                2 => num::bigint::Sign::Plus,
                _ => {
                    return Err(binrw::Error::AssertFail {
                        pos: reader.stream_position()?,
                        message: "Invalid sign byte".into(),
                    });
                }
            },
            &bytes,
        );
        Ok(Integer(big_int))
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
            TypedInteger::Big(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_addition() {
        let dec1 = Integer::from_string("12").unwrap();
        let dec2 = Integer::from_string("56").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "68");

        let dec1 = Integer::from_string("-12345").unwrap();
        let dec2 = Integer::from_string("3").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "-12342");
    }

    #[test]
    fn formatting() {
        let int1 = Integer::from_string("12").unwrap();
        assert_eq!(int1.to_string(), "12");

        let int2 = Integer::from_string("-12345").unwrap();
        assert_eq!(int2.to_string(), "-12345");
        let int3 = Integer::from_string("0").unwrap();
        assert_eq!(int3.to_string(), "0");

        let int4 =
            Integer::from_string("123456789012345678901234567890").unwrap();
        assert_eq!(int4.to_string(), "123456789012345678901234567890");

        let int5 =
            Integer::from_string("-123456789012345678901234567890").unwrap();
        assert_eq!(int5.to_string(), "-123456789012345678901234567890");
    }
}
