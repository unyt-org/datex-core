use num_traits::{Float, FromBytes, Signed, ToBytes, ToPrimitive, Zero};
use ordered_float::OrderedFloat;
use std::hash::Hash;
use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};
use std::io::{Read, Seek};
use std::ops::{Neg, Sub};
use std::str::FromStr;
use bigdecimal::BigDecimal;
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use num_bigint::BigInt;
use num_enum::TryFromPrimitive;
use crate::datex_values::{
    core_value_trait::CoreValueTrait, traits::soft_eq::SoftEq,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtendedBigDecimal {
    /// all non-zero finite big decimals
    Finite(BigDecimal),
    /// +0.0
    Zero,
    /// -0.0
    MinusZero,
    /// +infinity
    Infinity,
    /// -infinity
    NegativeInfinity,
    // nan
    NaN,
}

impl ExtendedBigDecimal {
    pub fn from_string(s: &str) -> Option<ExtendedBigDecimal> {
        match s {
            "infinity" | "Infinity" => Some(ExtendedBigDecimal::Infinity),
            "-infinity" | "-Infinity" => Some(ExtendedBigDecimal::NegativeInfinity),
            "nan" | "NaN" => Some(ExtendedBigDecimal::NaN),
            _ => {
                let big_decimal = BigDecimal::from_str(s).ok()?;
                if big_decimal.is_zero() {
                    if s.starts_with('-') {
                        Some(ExtendedBigDecimal::MinusZero)
                    } else {
                        Some(ExtendedBigDecimal::Zero)
                    }
                } else {
                    Some(ExtendedBigDecimal::Finite(big_decimal))
                }
            }
        }
    }

    pub fn neg(&self) -> Self {
        match self {
            ExtendedBigDecimal::Finite(value) => ExtendedBigDecimal::Finite(-value),
            ExtendedBigDecimal::Zero => ExtendedBigDecimal::MinusZero,
            ExtendedBigDecimal::MinusZero => ExtendedBigDecimal::Zero,
            ExtendedBigDecimal::Infinity => ExtendedBigDecimal::NegativeInfinity,
            ExtendedBigDecimal::NegativeInfinity => ExtendedBigDecimal::Infinity,
            ExtendedBigDecimal::NaN => ExtendedBigDecimal::NaN,
        }
    }
}

impl Display for ExtendedBigDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtendedBigDecimal::Finite(value) => write!(f, "{}", value),
            ExtendedBigDecimal::Zero => write!(f, "0.0"),
            ExtendedBigDecimal::MinusZero => write!(f, "-0.0"),
            ExtendedBigDecimal::Infinity => write!(f, "infinity"),
            ExtendedBigDecimal::NegativeInfinity => write!(f, "-infinity"),
            ExtendedBigDecimal::NaN => write!(f, "nan"),
        }
    }
}

impl TryFrom<BigDecimalType> for ExtendedBigDecimal {
    type Error = ();
    fn try_from(value: BigDecimalType) -> Result<Self, Self::Error> {
        match value {
            BigDecimalType::Zero => Ok(ExtendedBigDecimal::Zero),
            BigDecimalType::MinusZero => Ok(ExtendedBigDecimal::MinusZero),
            BigDecimalType::Infinity => Ok(ExtendedBigDecimal::Infinity),
            BigDecimalType::NegativeInfinity => Ok(ExtendedBigDecimal::NegativeInfinity),
            BigDecimalType::NaN => Ok(ExtendedBigDecimal::NaN),
            BigDecimalType::Finite => Err(()), // Finite is not a valid type for conversion
        }
    }
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum BigDecimalType {
    Finite = 0x00,
    Zero = 0x01,
    MinusZero = 0x02,
    Infinity = 0x03,
    NegativeInfinity = 0x04,
    NaN = 0x05,
}

impl From<&ExtendedBigDecimal> for BigDecimalType {
    fn from(value: &ExtendedBigDecimal) -> Self {
        match value {
            ExtendedBigDecimal::Finite(_) => BigDecimalType::Finite,
            ExtendedBigDecimal::Zero => BigDecimalType::Zero,
            ExtendedBigDecimal::MinusZero => BigDecimalType::MinusZero,
            ExtendedBigDecimal::Infinity => BigDecimalType::Infinity,
            ExtendedBigDecimal::NegativeInfinity => BigDecimalType::NegativeInfinity,
            ExtendedBigDecimal::NaN => BigDecimalType::NaN,
        }
    }
}

impl BinRead for ExtendedBigDecimal {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(reader: &mut R, endian: Endian, _: Self::Args<'_>) -> BinResult<Self> {
        // only handle le for now
        if endian != Endian::Little {
            return Err(binrw::Error::AssertFail {
                pos: reader.stream_position().unwrap_or(0),
                message: "Only little-endian is supported for ExtendedBigDecimal".to_string(),
            });
        }
        let big_decimal_type = BigDecimalType::try_from(reader.read_le::<u8>()?);

        match big_decimal_type {
            Ok(BigDecimalType::Finite) => {
                let bigint_length = reader.read_le::<u32>()? as usize;
                let mut bigint_bytes = vec![0; bigint_length];
                reader.read_exact(&mut bigint_bytes)?;
                let exponent = reader.read_le::<i64>()?;
                if bigint_bytes.len() != bigint_length {
                    return Err(binrw::Error::AssertFail {
                        pos: reader.stream_position().unwrap_or(0),
                        message: format!(
                            "Expected {} bytes for BigInt, but got {}",
                            bigint_length,
                            bigint_bytes.len()
                        ),
                    });
                }
                let bigint = BigInt::from_signed_bytes_le(&bigint_bytes);
                let big_decimal = BigDecimal::new(bigint, exponent);
                if big_decimal.is_negative() && big_decimal.is_zero() {
                    Ok(ExtendedBigDecimal::MinusZero)
                } else if big_decimal.is_zero() {
                    Ok(ExtendedBigDecimal::Zero)
                } else {
                    Ok(ExtendedBigDecimal::Finite(big_decimal))
                }
            },
            Ok(big_decimal_type) => Ok(big_decimal_type.try_into().unwrap()),
            Err(_) => Err(binrw::Error::AssertFail {
                pos: reader.stream_position().unwrap_or(0),
                message: "Invalid BigDecimalType".to_string(),
            }),
        }

    }
}

impl BinWrite for ExtendedBigDecimal {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + Seek>(&self, writer: &mut W, endian: Endian, _: Self::Args<'_>) -> BinResult<()> {
        // only handle le for now
        if endian != Endian::Little {
            return Err(binrw::Error::AssertFail {
                pos: writer.stream_position().unwrap_or(0),
                message: "Only little-endian is supported for ExtendedBigDecimal".to_string(),
            });
        }
        // write type
        writer.write_all(&[BigDecimalType::from(self) as u8])?;

        // if finite, add value
        if let ExtendedBigDecimal::Finite(value) = self {
            let (bigint, exponent) = value.as_bigint_and_exponent();
            let bigint_bytes = bigint.to_signed_bytes_le();
            // assert that bigint_bytes.len() is in u32 range
            if bigint_bytes.len() > u32::MAX as usize {
                return Err(binrw::Error::AssertFail {
                    pos: writer.stream_position().unwrap_or(0),
                    message: "BigInt too large to fit in u32".to_string(),
                });
            }
            writer.write_all(&(bigint_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(&bigint_bytes)?;
            writer.write_all(&exponent.to_le_bytes())?;
        }

        Ok(())
    }
}

impl ExtendedBigDecimal {
    fn try_into_f32(&self) -> Option<f32> {
        match self {
            ExtendedBigDecimal::Finite(value) => value.to_f32(),
            ExtendedBigDecimal::Zero => Some(0.0),
            ExtendedBigDecimal::MinusZero => Some(-0.0),
            ExtendedBigDecimal::Infinity => Some(f32::INFINITY),
            ExtendedBigDecimal::NegativeInfinity => Some(f32::NEG_INFINITY),
            ExtendedBigDecimal::NaN => None,
        }
    }
    fn try_into_f64(&self) -> Option<f64> {
        match self {
            ExtendedBigDecimal::Finite(value) => value.to_f64(),
            ExtendedBigDecimal::Zero => Some(0.0),
            ExtendedBigDecimal::MinusZero => Some(-0.0),
            ExtendedBigDecimal::Infinity => Some(f64::INFINITY),
            ExtendedBigDecimal::NegativeInfinity => Some(f64::NEG_INFINITY),
            ExtendedBigDecimal::NaN => None,
        }
    }
}

impl SoftEq for ExtendedBigDecimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ExtendedBigDecimal::Finite(a), ExtendedBigDecimal::Finite(b)) => a == b,
            (ExtendedBigDecimal::Zero, ExtendedBigDecimal::Zero) => true,
            (ExtendedBigDecimal::MinusZero, ExtendedBigDecimal::MinusZero) => true,
            (ExtendedBigDecimal::Infinity, ExtendedBigDecimal::Infinity) => true,
            (ExtendedBigDecimal::NegativeInfinity, ExtendedBigDecimal::NegativeInfinity) => true,
            (ExtendedBigDecimal::NaN, ExtendedBigDecimal::NaN) => false,
            _ => false,
        }
    }
}


impl From<BigDecimal> for ExtendedBigDecimal {
    fn from(value: BigDecimal) -> Self {
        if value.is_negative() && value.is_zero() {
            ExtendedBigDecimal::MinusZero
        }
        else if value.is_zero() {
            ExtendedBigDecimal::Zero
        }
        else {
            ExtendedBigDecimal::Finite(value)
        }
    }
}

impl From<f32> for ExtendedBigDecimal {
    fn from(value: f32) -> Self {
        if value.is_nan() {
            ExtendedBigDecimal::NaN
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                ExtendedBigDecimal::Infinity
            } else {
                ExtendedBigDecimal::NegativeInfinity
            }
        } else if value.is_zero() && value.is_sign_negative() {
            ExtendedBigDecimal::MinusZero
        } else {
            ExtendedBigDecimal::Finite(BigDecimal::try_from(value).unwrap())
        }
    }
}

impl From<f64> for ExtendedBigDecimal {
    fn from(value: f64) -> Self {
        if value.is_nan() {
            ExtendedBigDecimal::NaN
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                ExtendedBigDecimal::Infinity
            } else {
                ExtendedBigDecimal::NegativeInfinity
            }
        } else if value.is_zero() && value.is_sign_negative() {
            ExtendedBigDecimal::MinusZero
        } else {
            ExtendedBigDecimal::Finite(BigDecimal::try_from(value).unwrap())
        }
    }
}

impl Add for ExtendedBigDecimal {
    type Output = ExtendedBigDecimal;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (ExtendedBigDecimal::Finite(a), ExtendedBigDecimal::Finite(b)) => {
                ExtendedBigDecimal::Finite(a + b)
            }
            (ExtendedBigDecimal::Zero, b) | (b, ExtendedBigDecimal::Zero) => b,
            (ExtendedBigDecimal::MinusZero, b) | (b, ExtendedBigDecimal::MinusZero) => b,
            (ExtendedBigDecimal::Infinity, ExtendedBigDecimal::NegativeInfinity) | (ExtendedBigDecimal::NegativeInfinity, ExtendedBigDecimal::Infinity) => {
                ExtendedBigDecimal::NaN
            }
            (ExtendedBigDecimal::Infinity, _) | (_, ExtendedBigDecimal::Infinity) => {
                ExtendedBigDecimal::Infinity
            }
            (ExtendedBigDecimal::NegativeInfinity, _) | (_, ExtendedBigDecimal::NegativeInfinity) => {
                ExtendedBigDecimal::NegativeInfinity
            }
            (ExtendedBigDecimal::NaN, _) | (_, ExtendedBigDecimal::NaN) => ExtendedBigDecimal::NaN,
        }
    }
}

impl Add<TypedDecimal> for ExtendedBigDecimal {
    type Output = ExtendedBigDecimal;

    fn add(self, rhs: TypedDecimal) -> Self::Output {
        match rhs {
            TypedDecimal::F32(value) => {
                match (self, value.into_inner()) {
                    // both are minus zero
                    (ExtendedBigDecimal::MinusZero, b) if b.is_zero() && b.is_sign_negative() => ExtendedBigDecimal::MinusZero,
                    (ExtendedBigDecimal::Zero, 0.0) | (ExtendedBigDecimal::MinusZero, 0.0) => ExtendedBigDecimal::Zero,
                    // left zero/minus zero and right other
                    (ExtendedBigDecimal::Zero, b) | (ExtendedBigDecimal::MinusZero, b) => {
                        ExtendedBigDecimal::Finite(BigDecimal::try_from(b).unwrap())
                    }

                    // either left or right is NaN
                    (ExtendedBigDecimal::NaN, _) => ExtendedBigDecimal::NaN,
                    (_, b) if b.is_nan() => ExtendedBigDecimal::NaN,

                    // either left or right is +infinity
                    (ExtendedBigDecimal::Infinity, _) | (_, f32::INFINITY) => ExtendedBigDecimal::Infinity,
                    // either left or right is -infinity
                    (ExtendedBigDecimal::NegativeInfinity, _) | (_, f32::NEG_INFINITY) => ExtendedBigDecimal::NegativeInfinity,

                    // both are finite
                    (ExtendedBigDecimal::Finite(a), b) => {
                        ExtendedBigDecimal::Finite(a + BigDecimal::try_from(b).unwrap())
                    }
                }
            }
            TypedDecimal::F64(value) => {
                match (self, value.into_inner()) {
                    // both are minus zero
                    (ExtendedBigDecimal::MinusZero, b) if b.is_zero() && b.is_sign_negative() => ExtendedBigDecimal::MinusZero,
                    (ExtendedBigDecimal::Zero, 0.0) | (ExtendedBigDecimal::MinusZero, 0.0) => ExtendedBigDecimal::Zero,
                    // left zero/minus zero and right other
                    (ExtendedBigDecimal::Zero, b) | (ExtendedBigDecimal::MinusZero, b) => {
                        ExtendedBigDecimal::Finite(BigDecimal::try_from(b).unwrap())
                    }

                    // either left or right is NaN
                    (ExtendedBigDecimal::NaN, _) => ExtendedBigDecimal::NaN,
                    (_, b) if b.is_nan() => ExtendedBigDecimal::NaN,

                    // either left or right is +infinity
                    (ExtendedBigDecimal::Infinity, _) | (_, f64::INFINITY) => ExtendedBigDecimal::Infinity,
                    // either left or right is -infinity
                    (ExtendedBigDecimal::NegativeInfinity, _) | (_, f64::NEG_INFINITY) => ExtendedBigDecimal::NegativeInfinity,

                    // both are finite
                    (ExtendedBigDecimal::Finite(a), b) => {
                        ExtendedBigDecimal::Finite(a + BigDecimal::try_from(b).unwrap())
                    }
                }
            }
            TypedDecimal::Big(value) => self.add(value),
        }
    }
}



// TODO: currently not required
pub fn smallest_fitting_float(value: f64) -> TypedDecimal {
    if value.is_nan()
        || value.is_infinite()
        || (value >= f32::MIN as f64 && value <= f32::MAX as f64)
    {
        TypedDecimal::F32(OrderedFloat(value as f32))
    }
    // otherwise use f64
    else {
        TypedDecimal::F64(OrderedFloat(value))
    }
}

// TODO: normal decimal must always use f64 under the hood, otherwise soft_eq and eq will not work correctly for all cases!
#[derive(Debug, Clone, Eq)]
pub struct Decimal(pub TypedDecimal);
impl SoftEq for Decimal {
    fn soft_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: Into<TypedDecimal>> From<T> for Decimal {
    fn from(value: T) -> Self {
        let typed = value.into();
        Decimal(TypedDecimal::Big(ExtendedBigDecimal::from(typed.as_f64())))
    }
}

impl From<&str> for Decimal {
    fn from(value: &str) -> Self {
        Decimal(TypedDecimal::Big(ExtendedBigDecimal::from_string(&value).unwrap_or(ExtendedBigDecimal::NaN)))
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        Decimal(self.0 + rhs.0)
    }
}

impl Add for &Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        Decimal::add(self.clone(), rhs.clone())
    }
}

impl Sub for Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self.0, rhs.0) {
            (TypedDecimal::Big(a), TypedDecimal::Big(b)) => {
                Decimal(TypedDecimal::Big(a.add(b.neg())))
            }
            _ => unreachable!("Subtraction of Decimal should only be used with TypedDecimal::Big"),
        }
    }
}

impl Sub for &Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Self) -> Self::Output {
        Decimal::sub(self.clone(), rhs.clone())
    }
}


impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypedDecimal {
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
    Big(ExtendedBigDecimal)
}
impl CoreValueTrait for TypedDecimal {}

impl SoftEq for TypedDecimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypedDecimal::F32(a), TypedDecimal::F32(b)) => a == b,
            (TypedDecimal::F64(a), TypedDecimal::F64(b)) => a == b,
            (TypedDecimal::F32(a), TypedDecimal::F64(b)) | (TypedDecimal::F64(b), TypedDecimal::F32(a)) => {
                a.into_inner() as f64 == b.into_inner()
            }
            (TypedDecimal::Big(a), TypedDecimal::Big(b)) => a.soft_eq(b),
            (a, TypedDecimal::Big(b)) | (TypedDecimal::Big(b), a) => {
                match a {
                    TypedDecimal::F32(value) => {
                        b.try_into_f32().map_or(false, |v| v == value.into_inner())
                    }
                    TypedDecimal::F64(value) => {
                        b.try_into_f64().map_or(false, |v| v == value.into_inner())
                    }
                    _ => false,
                }
            }
        }
    }
}

impl TypedDecimal {
    pub fn as_f32(&self) -> f32 {
        match self {
            TypedDecimal::F32(value) => value.into_inner(),
            TypedDecimal::F64(value) => value.into_inner() as f32,
            TypedDecimal::Big(value) => value.try_into_f32().unwrap_or(f32::NAN),
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            TypedDecimal::F32(value) => value.into_inner() as f64,
            TypedDecimal::F64(value) => value.into_inner(),
            TypedDecimal::Big(value) => value.try_into_f64().unwrap_or(f64::NAN),
        }
    }

    /// Returns true if the value can be represented as an exact integer in the range of i64.
    pub fn is_integer(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => {
                value.into_inner() as f64 >= i64::MIN as f64
                    && value.into_inner() as f64 <= i64::MAX as f64
                    && !(value.into_inner().is_zero()
                        && value.into_inner().is_sign_negative())
                    && value.into_inner().fract() == 0.0
            }
            TypedDecimal::F64(value) => {
                value.into_inner() >= i64::MIN as f64
                    && value.into_inner() <= i64::MAX as f64
                    && !(value.into_inner().is_zero()
                        && value.into_inner().is_sign_negative())
                    && value.into_inner().fract() == 0.0
            }
            TypedDecimal::Big(value) => match value {
                ExtendedBigDecimal::Finite(big_value) => {
                    big_value.is_integer() && big_value.to_f64().unwrap_or(f64::NAN).is_finite()
                }
                ExtendedBigDecimal::Zero => true,
                ExtendedBigDecimal::MinusZero => true,
                ExtendedBigDecimal::Infinity | ExtendedBigDecimal::NegativeInfinity | ExtendedBigDecimal::NaN => false,
            },
        }
    }

    /// Returns the value as an integer if it is an exact integer, otherwise returns None.
    pub fn as_integer(&self) -> Option<i64> {
        if self.is_integer() {
            match self {
                TypedDecimal::F32(value) => Some(value.into_inner() as i64),
                TypedDecimal::F64(value) => Some(value.into_inner() as i64),
                TypedDecimal::Big(value) => match value {
                    ExtendedBigDecimal::Finite(big_value) => big_value.to_i64(),
                    ExtendedBigDecimal::Zero => Some(0),
                    ExtendedBigDecimal::MinusZero => Some(0),
                    ExtendedBigDecimal::Infinity | ExtendedBigDecimal::NegativeInfinity | ExtendedBigDecimal::NaN => None,
                },
            }
        } else {
            None
        }
    }

    pub fn is_f32(&self) -> bool {
        matches!(self, TypedDecimal::F32(_))
    }
    pub fn is_f64(&self) -> bool {
        matches!(self, TypedDecimal::F64(_))
    }
    pub fn is_positive(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_sign_positive(),
            TypedDecimal::F64(value) => value.is_sign_positive(),
            TypedDecimal::Big(value) => match value {
                ExtendedBigDecimal::Finite(big_value) => big_value.is_positive(),
                ExtendedBigDecimal::Zero => true,
                ExtendedBigDecimal::MinusZero => false,
                ExtendedBigDecimal::Infinity => true,
                ExtendedBigDecimal::NegativeInfinity | ExtendedBigDecimal::NaN => false,
            }
        }
    }
    pub fn is_negative(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_sign_negative(),
            TypedDecimal::F64(value) => value.is_sign_negative(),
            TypedDecimal::Big(value) => match value {
                ExtendedBigDecimal::Finite(big_value) => big_value.is_negative(),
                ExtendedBigDecimal::Zero => false,
                ExtendedBigDecimal::MinusZero => true,
                ExtendedBigDecimal::Infinity | ExtendedBigDecimal::NaN => false,
                ExtendedBigDecimal::NegativeInfinity => true,
            },
        }
    }
    pub fn is_nan(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_nan(),
            TypedDecimal::F64(value) => value.is_nan(),
            TypedDecimal::Big(value) => match value {
                ExtendedBigDecimal::NaN => true,
                _ => false,
            },
        }
    }
}

impl Display for TypedDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedDecimal::F32(value) => {
                decimal_to_string(value.into_inner(), false).fmt(f)
            }
            TypedDecimal::F64(value) => {
                decimal_to_string(value.into_inner(), false).fmt(f)
            }
            TypedDecimal::Big(value) => value.to_string().fmt(f),
        }
    }
}

pub fn decimal_to_string<T: Float + Display>(
    value: T,
    json_compatible: bool,
) -> String {
    if value.is_nan() {
        if json_compatible {
            "NaN".to_string()
        } else {
            "nan".to_string()
        }
    } else if value.is_infinite() {
        format!(
            "{}{}",
            if value.is_sign_positive() { "" } else { "-" },
            if json_compatible {
                "Infinity".to_string()
            } else {
                "infinity".to_string()
            }
        )
    } else if value.fract() == T::zero() {
        format!("{value:.1}")
    }
    // TODO: add e-notation for large numbers
    else {
        format!("{value}")
    }
}

impl Add for TypedDecimal {
    type Output = TypedDecimal;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            TypedDecimal::F32(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F32(a + b),
                TypedDecimal::F64(b) => TypedDecimal::F32(OrderedFloat(
                    a.into_inner() + b.into_inner() as f32,
                )),
                TypedDecimal::Big(b) => {
                    let result = b + TypedDecimal::F32(a);
                    if let Some(result_f32) = result.try_into_f32() {
                        TypedDecimal::F32(result_f32.into())
                    }
                    else {
                        TypedDecimal::F32(f32::NAN.into())
                    }
                }
            },
            TypedDecimal::F64(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F64(OrderedFloat(
                    a.into_inner() + b.into_inner() as f64,
                )),
                TypedDecimal::F64(b) => TypedDecimal::F64(a + b),
                TypedDecimal::Big(b) => {
                    let result = b + TypedDecimal::F64(a);
                    if let Some(result_f64) = result.try_into_f64() {
                        TypedDecimal::F64(result_f64.into())
                    }
                    else {
                        TypedDecimal::F64(f64::NAN.into())
                    }
                }
            },
            TypedDecimal::Big(a) => TypedDecimal::Big(a + rhs)
        }
    }
}

impl Add for &TypedDecimal {
    type Output = TypedDecimal;

    fn add(self, rhs: Self) -> Self::Output {
        TypedDecimal::add(self.clone(), rhs.clone())
    }
}

impl Sub for TypedDecimal {
    type Output = TypedDecimal;

    fn sub(self, rhs: Self) -> Self::Output {
        let neg_rhs = match rhs {
            TypedDecimal::F32(v) => TypedDecimal::F32(OrderedFloat(v.into_inner().neg())),
            TypedDecimal::F64(v) => TypedDecimal::F64(OrderedFloat(v.into_inner().neg())),
            TypedDecimal::Big(v) => TypedDecimal::Big(v.neg()),
        };
        TypedDecimal::add(self, neg_rhs)
    }
}


impl Sub for &TypedDecimal {
    type Output = TypedDecimal;

    fn sub(self, rhs: Self) -> Self::Output {
        TypedDecimal::sub(self.clone(), rhs.clone())
    }
}


impl AddAssign for TypedDecimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = TypedDecimal::add(self.clone(), rhs);
    }
}

impl From<f32> for TypedDecimal {
    fn from(value: f32) -> Self {
        TypedDecimal::F32(value.into())
    }
}
impl From<f64> for TypedDecimal {
    fn from(value: f64) -> Self {
        TypedDecimal::F64(value.into())
    }
}


impl From<ExtendedBigDecimal> for TypedDecimal {
    fn from(value: ExtendedBigDecimal) -> Self {
        TypedDecimal::Big(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smallest_fitting_float() {
        assert_eq!(
            smallest_fitting_float(1.0),
            TypedDecimal::F32(OrderedFloat(1.0))
        );
        assert_eq!(
            smallest_fitting_float(1.5),
            TypedDecimal::F32(OrderedFloat(1.5))
        );
        assert_eq!(
            smallest_fitting_float(1e200),
            TypedDecimal::F64(OrderedFloat(1e200))
        );
        assert_eq!(
            smallest_fitting_float(f64::NAN),
            TypedDecimal::F32(OrderedFloat(f32::NAN))
        );
    }

    #[test]
    fn test_decimal_addition() {
        let a = Decimal::from("1.0");
        let b = Decimal::from("2.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("3.0"));

        let c = Decimal::from("1.5");
        let d = Decimal::from("2.5");
        let result2 = c + d;
        assert_eq!(
            result2,
            Decimal::from(4.0)
        );

        let e = Decimal::from("0.1");
        let f = Decimal::from("0.2");
        let result3 = &e + &f;
        assert_eq!(
            result3,
            Decimal::from("0.3")
        );
    }
}
