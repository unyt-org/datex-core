use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::datex_values::{
    core_value_trait::CoreValueTrait, traits::soft_eq::SoftEq,
};
use bigdecimal::BigDecimal;
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use num_bigint::BigInt;
use num_enum::TryFromPrimitive;
use num_traits::{Float, FromBytes, Signed, ToBytes, ToPrimitive, Zero};
use ordered_float::OrderedFloat;
use std::hash::Hash;
use std::io::{Read, Seek};
use std::ops::{Neg, Sub};
use std::str::FromStr;
use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

#[derive(Debug, Clone, Eq, Hash)]
pub enum ExtendedBigDecimal {
    /// all non-zero finite big decimals
    /// We should never use Finite(BigDecimal) directly, but rather use the ExtendedBigDecimal::from_string
    /// to avoid creating invalid finite values that contain 0.
    Finite(BigDecimal),
    /// +0.0
    Zero,
    /// -0.0
    MinusZero,
    /// +infinity
    Inf,
    /// -infinity
    NegInf,
    // nan
    NaN,
}

impl PartialEq for ExtendedBigDecimal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ExtendedBigDecimal::Finite(a), ExtendedBigDecimal::Finite(b)) => {
                a == b
            }
            (ExtendedBigDecimal::Zero, ExtendedBigDecimal::Zero)
            | (ExtendedBigDecimal::MinusZero, ExtendedBigDecimal::MinusZero)
            | (ExtendedBigDecimal::Inf, ExtendedBigDecimal::Inf)
            | (ExtendedBigDecimal::NegInf, ExtendedBigDecimal::NegInf) => true,
            (ExtendedBigDecimal::NaN, _) | (_, ExtendedBigDecimal::NaN) => {
                false
            }
            _ => false,
        }
    }
}

impl ExtendedBigDecimal {
    pub fn from_string(s: &str) -> Option<ExtendedBigDecimal> {
        match s {
            "infinity" | "Infinity" => Some(ExtendedBigDecimal::Inf),
            "-infinity" | "-Infinity" => Some(ExtendedBigDecimal::NegInf),
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
            ExtendedBigDecimal::Finite(value) => {
                ExtendedBigDecimal::Finite(-value)
            }
            ExtendedBigDecimal::Zero => ExtendedBigDecimal::MinusZero,
            ExtendedBigDecimal::MinusZero => ExtendedBigDecimal::Zero,
            ExtendedBigDecimal::Inf => ExtendedBigDecimal::NegInf,
            ExtendedBigDecimal::NegInf => ExtendedBigDecimal::Inf,
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
            ExtendedBigDecimal::Inf => write!(f, "infinity"),
            ExtendedBigDecimal::NegInf => write!(f, "-infinity"),
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
            BigDecimalType::Infinity => Ok(ExtendedBigDecimal::Inf),
            BigDecimalType::NegativeInfinity => Ok(ExtendedBigDecimal::NegInf),
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
            ExtendedBigDecimal::Inf => BigDecimalType::Infinity,
            ExtendedBigDecimal::NegInf => BigDecimalType::NegativeInfinity,
            ExtendedBigDecimal::NaN => BigDecimalType::NaN,
        }
    }
}

impl BinRead for ExtendedBigDecimal {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        // only handle le for now
        if endian != Endian::Little {
            return Err(binrw::Error::AssertFail {
                pos: reader.stream_position().unwrap_or(0),
                message:
                    "Only little-endian is supported for ExtendedBigDecimal"
                        .to_string(),
            });
        }
        let big_decimal_type =
            BigDecimalType::try_from(reader.read_le::<u8>()?);

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
            }
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

    fn write_options<W: std::io::Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        // only handle le for now
        if endian != Endian::Little {
            return Err(binrw::Error::AssertFail {
                pos: writer.stream_position().unwrap_or(0),
                message:
                    "Only little-endian is supported for ExtendedBigDecimal"
                        .to_string(),
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
    pub fn try_into_f32(&self) -> Option<f32> {
        match self {
            ExtendedBigDecimal::Finite(value) => value.to_f32(),
            ExtendedBigDecimal::Zero => Some(0.0),
            ExtendedBigDecimal::MinusZero => Some(-0.0),
            ExtendedBigDecimal::Inf => Some(f32::INFINITY),
            ExtendedBigDecimal::NegInf => Some(f32::NEG_INFINITY),
            ExtendedBigDecimal::NaN => None,
        }
    }
    pub fn try_into_f64(&self) -> Option<f64> {
        match self {
            ExtendedBigDecimal::Finite(value) => value.to_f64(),
            ExtendedBigDecimal::Zero => Some(0.0),
            ExtendedBigDecimal::MinusZero => Some(-0.0),
            ExtendedBigDecimal::Inf => Some(f64::INFINITY),
            ExtendedBigDecimal::NegInf => Some(f64::NEG_INFINITY),
            ExtendedBigDecimal::NaN => None,
        }
    }
}

impl SoftEq for ExtendedBigDecimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ExtendedBigDecimal::Finite(a), ExtendedBigDecimal::Finite(b)) => {
                a == b
            }
            (ExtendedBigDecimal::Zero, ExtendedBigDecimal::Zero) => true,
            (ExtendedBigDecimal::MinusZero, ExtendedBigDecimal::MinusZero) => {
                true
            }
            (ExtendedBigDecimal::Inf, ExtendedBigDecimal::Inf) => true,
            (ExtendedBigDecimal::NegInf, ExtendedBigDecimal::NegInf) => true,
            (ExtendedBigDecimal::NaN, ExtendedBigDecimal::NaN) => false,
            _ => false,
        }
    }
}

impl From<BigDecimal> for ExtendedBigDecimal {
    fn from(value: BigDecimal) -> Self {
        if value.is_negative() && value.is_zero() {
            ExtendedBigDecimal::MinusZero
        } else if value.is_zero() {
            ExtendedBigDecimal::Zero
        } else {
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
                ExtendedBigDecimal::Inf
            } else {
                ExtendedBigDecimal::NegInf
            }
        } else if value.is_zero() && value.is_sign_negative() {
            ExtendedBigDecimal::MinusZero
        } else if value.is_zero() {
            ExtendedBigDecimal::Zero
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
                ExtendedBigDecimal::Inf
            } else {
                ExtendedBigDecimal::NegInf
            }
        } else if value.is_zero() && value.is_sign_negative() {
            ExtendedBigDecimal::MinusZero
        } else if value.is_zero() {
            ExtendedBigDecimal::Zero
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
            (ExtendedBigDecimal::MinusZero, ExtendedBigDecimal::Zero)
            | (ExtendedBigDecimal::Zero, ExtendedBigDecimal::MinusZero) => {
                ExtendedBigDecimal::Zero
            }
            (ExtendedBigDecimal::Zero, b) | (b, ExtendedBigDecimal::Zero) => b,
            (ExtendedBigDecimal::MinusZero, b)
            | (b, ExtendedBigDecimal::MinusZero) => b,
            (ExtendedBigDecimal::Inf, ExtendedBigDecimal::NegInf)
            | (ExtendedBigDecimal::NegInf, ExtendedBigDecimal::Inf) => {
                ExtendedBigDecimal::NaN
            }
            (ExtendedBigDecimal::Inf, _) | (_, ExtendedBigDecimal::Inf) => {
                ExtendedBigDecimal::Inf
            }
            (ExtendedBigDecimal::NegInf, _)
            | (_, ExtendedBigDecimal::NegInf) => ExtendedBigDecimal::NegInf,
            (ExtendedBigDecimal::NaN, _) | (_, ExtendedBigDecimal::NaN) => {
                ExtendedBigDecimal::NaN
            }
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
                    (ExtendedBigDecimal::MinusZero, b)
                        if b.is_zero() && b.is_sign_negative() =>
                    {
                        ExtendedBigDecimal::MinusZero
                    }
                    (ExtendedBigDecimal::Zero, 0.0)
                    | (ExtendedBigDecimal::MinusZero, 0.0) => {
                        ExtendedBigDecimal::Zero
                    }
                    // left zero/minus zero and right other
                    (ExtendedBigDecimal::Zero, b)
                    | (ExtendedBigDecimal::MinusZero, b) => {
                        ExtendedBigDecimal::Finite(
                            BigDecimal::try_from(b).unwrap(),
                        )
                    }

                    // either left or right is NaN
                    (ExtendedBigDecimal::NaN, _) => ExtendedBigDecimal::NaN,
                    (_, b) if b.is_nan() => ExtendedBigDecimal::NaN,

                    // either left or right is +infinity
                    (ExtendedBigDecimal::Inf, _) | (_, f32::INFINITY) => {
                        ExtendedBigDecimal::Inf
                    }
                    // either left or right is -infinity
                    (ExtendedBigDecimal::NegInf, _)
                    | (_, f32::NEG_INFINITY) => ExtendedBigDecimal::NegInf,

                    // both are finite
                    (ExtendedBigDecimal::Finite(a), b) => {
                        ExtendedBigDecimal::from(
                            a + BigDecimal::try_from(b).unwrap(),
                        )
                    }
                }
            }
            TypedDecimal::F64(value) => {
                match (self, value.into_inner()) {
                    // both are minus zero
                    (ExtendedBigDecimal::MinusZero, b)
                        if b.is_zero() && b.is_sign_negative() =>
                    {
                        ExtendedBigDecimal::MinusZero
                    }
                    (ExtendedBigDecimal::Zero, 0.0)
                    | (ExtendedBigDecimal::MinusZero, 0.0) => {
                        ExtendedBigDecimal::Zero
                    }
                    // left zero/minus zero and right other
                    (ExtendedBigDecimal::Zero, b)
                    | (ExtendedBigDecimal::MinusZero, b) => {
                        ExtendedBigDecimal::Finite(
                            BigDecimal::try_from(b).unwrap(),
                        )
                    }

                    // either left or right is NaN
                    (ExtendedBigDecimal::NaN, _) => ExtendedBigDecimal::NaN,
                    (_, b) if b.is_nan() => ExtendedBigDecimal::NaN,

                    // either left or right is +infinity
                    (ExtendedBigDecimal::Inf, _) | (_, f64::INFINITY) => {
                        ExtendedBigDecimal::Inf
                    }
                    // either left or right is -infinity
                    (ExtendedBigDecimal::NegInf, _)
                    | (_, f64::NEG_INFINITY) => ExtendedBigDecimal::NegInf,

                    // both are finite
                    (ExtendedBigDecimal::Finite(a), b) => {
                        ExtendedBigDecimal::from(
                            a + BigDecimal::try_from(b).unwrap(),
                        )
                    }
                }
            }
            TypedDecimal::Big(value) => self.add(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::datex_values::core_values::decimal::big_decimal::ExtendedBigDecimal;

    #[test]
    fn test_zero() {
        let a = ExtendedBigDecimal::from(0.0f32);
        assert!(matches!(a, ExtendedBigDecimal::Zero));
        assert!(!matches!(a, ExtendedBigDecimal::MinusZero));

        let b = ExtendedBigDecimal::from(0.0f64);
        assert!(matches!(b, ExtendedBigDecimal::Zero));
        assert!(!matches!(b, ExtendedBigDecimal::MinusZero));

        let c = ExtendedBigDecimal::from_string("0.0").unwrap();
        assert!(matches!(c, ExtendedBigDecimal::Zero));
        assert!(!matches!(c, ExtendedBigDecimal::MinusZero));

        ExtendedBigDecimal::Finite(0.into());
    }

    #[test]
    fn test_minus_zero() {
        let a = ExtendedBigDecimal::from(-0.0f32);
        assert!(matches!(a, ExtendedBigDecimal::MinusZero));
        assert!(!matches!(a, ExtendedBigDecimal::Zero));

        let b = ExtendedBigDecimal::from(-0.0f64);
        assert!(matches!(b, ExtendedBigDecimal::MinusZero));
        assert!(!matches!(b, ExtendedBigDecimal::Zero));

        let c = ExtendedBigDecimal::from_string("-0.0").unwrap();
        assert!(matches!(c, ExtendedBigDecimal::MinusZero));
        assert!(!matches!(c, ExtendedBigDecimal::Zero));
    }
}
