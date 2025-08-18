use crate::values::{
    core_values::integer::{
        typed_integer::TypedInteger,
        utils::{smallest_fitting_signed, smallest_fitting_unsigned},
    },
    traits::structural_eq::StructuralEq,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvalidIntegerError {
    ParseError(String),
    OutOfBounds(String),
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Integer(pub BigInt);
impl Integer {
    // pub fn to_smallest_fitting(&self) -> TypedInteger {
    //     self.0.to_smallest_fitting()
    // }

    pub fn from_string(s: &str) -> Result<Self, InvalidIntegerError> {
        BigInt::from_str(s)
            .map(Integer)
            .map_err(|_| InvalidIntegerError::ParseError(s.into()))
    }
    pub fn from_string_radix(
        s: &str,
        radix: u32,
    ) -> Result<Self, InvalidIntegerError> {
        // remove all underscores
        let s = &s.replace('_', "");
        BigInt::from_str_radix(s, radix)
            .map(Integer)
            .map_err(|_| InvalidIntegerError::ParseError(s.into()))
    }

    pub fn is_negative(&self) -> bool {
        self.0.sign() == num::bigint::Sign::Minus
    }
    pub fn is_positive(&self) -> bool {
        self.0.sign() == num::bigint::Sign::Plus
    }

    pub fn as_i8(&self) -> Option<i8> {
        self.0.to_i8()
    }
    pub fn as_u8(&self) -> Option<u8> {
        self.0.to_u8()
    }
    pub fn as_i16(&self) -> Option<i16> {
        self.0.to_i16()
    }
    pub fn as_u16(&self) -> Option<u16> {
        self.0.to_u16()
    }
    pub fn as_i32(&self) -> Option<i32> {
        self.0.to_i32()
    }
    pub fn as_u32(&self) -> Option<u32> {
        self.0.to_u32()
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }
    pub fn as_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }
    pub fn as_i128(&self) -> Option<i128> {
        self.0.to_i128()
    }
    pub fn as_u128(&self) -> Option<u128> {
        self.0.to_u128()
    }

    // pub fn from_string(s: &str) -> Result<Self, String> {
    //     Integer::from_string_radix(s, 10)
    // }

    // pub fn from_string_radix(s: &str, radix: u32) -> Result<Self, String> {
    //     // remove all underscores
    //     let s = &s.replace('_', "");
    //     match i128::from_str_radix(s, radix) {
    //         Ok(value) => Ok(Integer(TypedInteger::I128(value))),
    //         Err(_) => match s.parse::<u128>() {
    //             Ok(value) => Ok(Integer(TypedInteger::U128(value))),
    //             Err(_) => Err(format!(
    //                 "Failed to parse integer from string with radix {radix}: {s}"
    //             )),
    //         },
    //     }
    // }
}

impl StructuralEq for Integer {
    fn structural_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Neg for Integer {
    type Output = Self;

    fn neg(self) -> Self::Output {
        return Integer(-self.0);
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
        endian: Endian,
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
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        let sign = reader.read_le::<u8>()?;
        let len = reader.read_le::<u32>()? as usize;
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes)?;

        let big_int = BigInt::from_bytes_be(
            if sign == 0 {
                num::bigint::Sign::Plus
            } else {
                num::bigint::Sign::Minus
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
            TypedInteger::Integer(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_addition() {
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
    fn test_formatting() {
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

    /* TODO Move these test cases to typed_integer module once ready (17/08/2025)
    #[test]
    fn test_typed_integer_addition() {
        let a = TypedInteger::I8(10);
        let b = TypedInteger::I8(20);

        let result = a + b;
        assert_eq!(result, Some(TypedInteger::I8(30)));

        let c = TypedInteger::U8(10);
        let result = a + c;
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
    fn test_typed_integer_subtraction() {
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
    fn test_integer_addition() {
        let a = Integer::from(10_i8);
        let b = Integer::from(20_i8);
        let result = a + b;
        assert_eq!(result, Some(Integer(TypedInteger::I8(30))));

        // let c = Integer::from(10 as u8);
        // let result = a + c;
        // assert_eq!(result, Some(Integer(TypedInteger::I8(20))));

        // let result = c + a;
        // assert_eq!(result, Some(Integer(TypedInteger::U8(20))));

        // // out of bounds
        // let d = Integer::from(100 as i8);
        // let e = Integer::from(50 as i8);
        // let result = d + e;
        // assert_eq!(result, None);
    }

    #[test]
    fn test_integer() {
        let a = Integer::from(1_i8);
        assert_eq!(a.0, TypedInteger::I8(1));

        let b = Integer::from(1_u8);
        assert_eq!(b.0, TypedInteger::U8(1));

        let c = Integer::from(1_i16);
        assert_eq!(c.0, TypedInteger::I16(1));

        let d = Integer::from(1_u16);
        assert_eq!(d.0, TypedInteger::U16(1));

        let e = Integer::from(1_i32);
        assert_eq!(e.0, TypedInteger::I32(1));

        let f = Integer::from(1_u32);
        assert_eq!(f.0, TypedInteger::U32(1));

        let g = Integer::from(1_i64);
        assert_eq!(g.0, TypedInteger::I64(1));

        let h = Integer::from(1_u64);
        assert_eq!(h.0, TypedInteger::U64(1));

        let i = Integer::from(1_i128);
        assert_eq!(i.0, TypedInteger::I128(1));

        let j = Integer::from(1_u128);
        assert_eq!(j.0, TypedInteger::U128(1));

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
     */
}
