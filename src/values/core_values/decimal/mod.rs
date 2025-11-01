pub mod rational;
pub mod typed_decimal;
pub mod utils;

use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::error::NumberParseError;
use bigdecimal::BigDecimal;
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use num::BigInt;
use num::BigRational;
use num_enum::TryFromPrimitive;
use num_traits::{FromPrimitive, Zero};
use rational::Rational;
use serde::{Deserialize, Serialize};
use core::cmp::Ordering;
use core::fmt::Display;
use crate::stdlib::hash::Hash;
use crate::stdlib::io::{Read, Seek};
use core::ops::{Add, Neg, Sub};
use core::str::FromStr;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub enum Decimal {
    Finite(Rational),
    NaN,
    Zero,
    NegZero,
    Infinity,
    NegInfinity,
}

impl Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Decimal::Finite(value) => value.hash(state),
            Decimal::NaN => 0.hash(state),
            Decimal::Zero => 1.hash(state),
            Decimal::NegZero => 1.hash(state),
            Decimal::Infinity => 2.hash(state),
            Decimal::NegInfinity => 3.hash(state),
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        if self.is_zero() && other.is_zero() {
            return true; // +0.0 == -0.0
        }
        match (self, other) {
            (Decimal::Finite(a), Decimal::Finite(b)) => a == b,
            (Decimal::Infinity, Decimal::Infinity) => true,
            (Decimal::NegInfinity, Decimal::NegInfinity) => true,
            (Decimal::NaN, Decimal::NaN) => true,
            _ => false,
        }
    }
}

impl Decimal {
    /// Attempts to convert the Decimal to an f32.
    /// If an overflow occurs, returns infinity or -infinity.
    pub fn into_f32(&self) -> f32 {
        match self {
            Decimal::Finite(value) => value.to_f32(),
            Decimal::Zero => 0.0,
            Decimal::NegZero => -0.0,
            Decimal::Infinity => f32::INFINITY,
            Decimal::NegInfinity => f32::NEG_INFINITY,
            Decimal::NaN => f32::NAN,
        }
    }

    /// Attempts to convert the Decimal to an f64.
    /// If an overflow occurs, returns infinity or -infinity.
    pub fn into_f64(&self) -> f64 {
        match self {
            Decimal::Finite(value) => value.to_f64(),
            Decimal::Zero => 0.0,
            Decimal::NegZero => -0.0,
            Decimal::Infinity => f64::INFINITY,
            Decimal::NegInfinity => f64::NEG_INFINITY,
            Decimal::NaN => f64::NAN,
        }
    }

    /// Returns true if the value is finite (not NaN or Infinity).
    pub fn is_finite(&self) -> bool {
        matches!(self, Decimal::Finite(_) | Decimal::Zero | Decimal::NegZero)
    }

    /// Returns true if the value is infinite (positive or negative).
    pub fn is_infinite(&self) -> bool {
        matches!(self, Decimal::Infinity | Decimal::NegInfinity)
    }

    /// Returns true if the value is zero (positive or negative).
    pub fn is_nan(&self) -> bool {
        matches!(self, Decimal::NaN)
    }

    /// Returns true if the value is zero (positive or negative).
    pub fn is_zero(&self) -> bool {
        matches!(self, Decimal::Zero | Decimal::NegZero)
    }

    /// Returns true if the value has a positive sign.
    /// Positive values are greater than or equal to zero. So -0.0 is not positive.
    pub fn is_sign_positive(&self) -> bool {
        match self {
            Decimal::Finite(value) => value.is_positive(),
            Decimal::Infinity | Decimal::Zero => true,
            Decimal::NegZero | Decimal::NaN | Decimal::NegInfinity => false,
        }
    }

    /// Returns true if the value has a negative sign.
    /// Negative values are less than zero. So +0.0 is not negative.
    pub fn is_sign_negative(&self) -> bool {
        match self {
            Decimal::Finite(value) => value.is_negative(),
            Decimal::NegZero | Decimal::NegInfinity => true,
            Decimal::Zero | Decimal::Infinity | Decimal::NaN => false,
        }
    }

    /// Parses a decimal string into a BigRational.
    fn parse_decimal_to_rational(s: &str) -> Option<BigRational> {
        let decimal = BigDecimal::from_str(s).ok()?;
        let (bigint, scale) = decimal.as_bigint_and_exponent();
        let ten = BigInt::from(10);

        match scale.cmp(&0) {
            Ordering::Equal => Some(BigRational::from(bigint)),
            Ordering::Greater => {
                let denominator = ten.pow(scale as u32);
                Some(BigRational::new(bigint, denominator))
            }
            Ordering::Less => {
                let numerator = bigint * ten.pow((-scale) as u32);
                Some(BigRational::from(numerator))
            }
        }
    }

    /// Creates a Decimal from a fraction represented by numerator and denominator strings.
    pub fn from_fraction(numerator: &str, denominator: &str) -> Self {
        let rational = BigRational::new(
            BigInt::from_str(numerator).unwrap(),
            BigInt::from_str(denominator).unwrap(),
        );
        Decimal::from(Rational::from_big_rational(rational))
    }

    /// Creates a Decimal from a string representation.
    /// TODO #333: Add error handling
    pub fn from_string(s: &str) -> Result<Self, NumberParseError> {
        // TODO #133 represent as Infinity/-Infinity if out of bounds for representable DATEX values
        match s {
            "Infinity" | "infinity" => Ok(Decimal::Infinity),
            "-Infinity" | "-infinity" => Ok(Decimal::NegInfinity),
            "nan" | "NaN" | "-nan" | "-NaN" => Ok(Decimal::NaN),
            _ => {
                let s = &s.trim().replace('_', "");
                if s.contains("/") {
                    // If the string contains a fraction, parse it as a fraction
                    let parts: Vec<&str> = s.split('/').collect();
                    if parts.len() == 2 {
                        Ok(Decimal::from_fraction(parts[0], parts[1]))
                    } else {
                        Err(NumberParseError::InvalidFormat)
                    }
                } else {
                    let big_rational = Decimal::parse_decimal_to_rational(s);
                    match big_rational {
                        Some(big_rational) => {
                            if big_rational.is_zero() {
                                if s.starts_with('-') {
                                    Ok(Decimal::NegZero)
                                } else {
                                    Ok(Decimal::Zero)
                                }
                            } else {
                                Ok(Decimal::Finite(
                                    Rational::from_big_rational(big_rational),
                                ))
                            }
                        }
                        None => Err(NumberParseError::InvalidFormat),
                    }
                }
            }
        }
    }
}

impl StructuralEq for Decimal {
    fn structural_eq(&self, other: &Self) -> bool {
        if self.is_zero() && other.is_zero() {
            return true; // +0.0 == -0.0
        }
        match (self, other) {
            (Decimal::Finite(a), Decimal::Finite(b)) => a == b,
            (Decimal::Infinity, Decimal::Infinity) => true,
            (Decimal::NegInfinity, Decimal::NegInfinity) => true,
            (Decimal::NaN, Decimal::NaN) => false,
            _ => false,
        }
    }
}

impl ValueEq for Decimal {
    fn value_eq(&self, other: &Self) -> bool {
        self.structural_eq(other)
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Decimal::Finite(value) => Decimal::Finite(-value),
            Decimal::Zero => Decimal::NegZero,
            Decimal::NegZero => Decimal::Zero,
            Decimal::Infinity => Decimal::NegInfinity,
            Decimal::NegInfinity => Decimal::Infinity,
            Decimal::NaN => Decimal::NaN,
        }
    }
}

impl Add for Decimal {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Decimal::Finite(a), Decimal::Finite(b)) => Decimal::from(a + b),
            (Decimal::NegZero, Decimal::Zero)
            | (Decimal::Zero, Decimal::NegZero) => Decimal::Zero,
            (Decimal::Zero, b) | (b, Decimal::Zero) => b,
            (Decimal::NegZero, b) | (b, Decimal::NegZero) => b,
            (Decimal::Infinity, Decimal::NegInfinity)
            | (Decimal::NegInfinity, Decimal::Infinity) => Decimal::NaN,
            (Decimal::Infinity, _) | (_, Decimal::Infinity) => {
                Decimal::Infinity
            }
            (Decimal::NegInfinity, _) | (_, Decimal::NegInfinity) => {
                Decimal::NegInfinity
            }
            (Decimal::NaN, _) | (_, Decimal::NaN) => Decimal::NaN,
        }
    }
}

impl Add for &Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        // FIXME #334: Avoid cloning, as add should be applicable for refs only
        Decimal::add(self.clone(), rhs.clone())
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Sub for &Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Self) -> Self::Output {
        // FIXME #335: Avoid cloning, as sub should be applicable for refs only
        Decimal::sub(self.clone(), rhs.clone())
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Decimal::Finite(value) => core::write!(f, "{value}"),
            Decimal::NaN => core::write!(f, "nan"),
            Decimal::Zero => core::write!(f, "0.0"),
            Decimal::NegZero => core::write!(f, "-0.0"),
            Decimal::Infinity => core::write!(f, "infinity"),
            Decimal::NegInfinity => core::write!(f, "-infinity"),
        }
    }
}

impl TryFrom<BigDecimalType> for Decimal {
    type Error = ();
    fn try_from(value: BigDecimalType) -> Result<Self, Self::Error> {
        match value {
            BigDecimalType::Zero => Ok(Decimal::Zero),
            BigDecimalType::NegZero => Ok(Decimal::NegZero),
            BigDecimalType::Infinity => Ok(Decimal::Infinity),
            BigDecimalType::NegInfinity => Ok(Decimal::NegInfinity),
            BigDecimalType::NaN => Ok(Decimal::NaN),
            BigDecimalType::Finite => Err(()), // Finite is not a valid type for conversion
        }
    }
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum BigDecimalType {
    Finite = 0x00,
    Zero = 0x01,
    NegZero = 0x02,
    Infinity = 0x03,
    NegInfinity = 0x04,
    NaN = 0x05,
}

impl From<&Decimal> for BigDecimalType {
    fn from(value: &Decimal) -> Self {
        match value {
            Decimal::Finite(_) => BigDecimalType::Finite,
            Decimal::Zero => BigDecimalType::Zero,
            Decimal::NegZero => BigDecimalType::NegZero,
            Decimal::Infinity => BigDecimalType::Infinity,
            Decimal::NegInfinity => BigDecimalType::NegInfinity,
            Decimal::NaN => BigDecimalType::NaN,
        }
    }
}

impl BinRead for Decimal {
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
                message: "Only little-endian is supported for Decimal"
                    .to_string(),
            });
        }
        let big_decimal_type =
            BigDecimalType::try_from(reader.read_le::<u8>()?);

        match big_decimal_type {
            Ok(BigDecimalType::Finite) => {
                let numerator_len = reader.read_le::<u32>()? as usize;
                let denominator_len = reader.read_le::<u32>()? as usize;

                let mut numerator_bytes = vec![0; numerator_len];
                let mut denominator_bytes = vec![0; denominator_len];

                reader.read_exact(&mut numerator_bytes)?;
                reader.read_exact(&mut denominator_bytes)?;

                let numerator = BigInt::from_signed_bytes_le(&numerator_bytes);
                let denominator =
                    BigInt::from_signed_bytes_le(&denominator_bytes);

                Ok(Decimal::Finite(Rational::new(numerator, denominator)))
            }
            Ok(big_decimal_type) => Ok(big_decimal_type.try_into().unwrap()),
            Err(_) => Err(binrw::Error::AssertFail {
                pos: reader.stream_position().unwrap_or(0),
                message: "Invalid BigDecimalType".to_string(),
            }),
        }
    }
}

impl BinWrite for Decimal {
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
                message: "Only little-endian is supported for Decimal"
                    .to_string(),
            });
        }
        // write type
        writer.write_all(&[BigDecimalType::from(self) as u8])?;

        // if finite, add value
        if let Decimal::Finite(value) = self {
            let numerator = value.numer();
            let denominator = value.denom();
            let numerator_bytes = numerator.to_signed_bytes_le();
            let denominator_bytes = denominator.to_signed_bytes_le();
            let numerator_len = numerator_bytes.len() as u32;
            let denominator_len = denominator_bytes.len() as u32;
            // write lengths
            writer.write_all(&numerator_len.to_le_bytes())?;
            writer.write_all(&denominator_len.to_le_bytes())?;
            // write numerator and denominator
            writer.write_all(&numerator_bytes)?;
            writer.write_all(&denominator_bytes)?;
        }

        Ok(())
    }
}

impl From<Rational> for Decimal {
    fn from(value: Rational) -> Self {
        if value.is_zero() {
            Decimal::Zero
        } else {
            Decimal::Finite(value)
        }
    }
}

impl From<f32> for Decimal {
    fn from(value: f32) -> Self {
        if value.is_nan() {
            Decimal::NaN
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                Decimal::Infinity
            } else {
                Decimal::NegInfinity
            }
        } else if value.is_zero() && value.is_sign_negative() {
            Decimal::NegZero
        } else if value.is_zero() {
            Decimal::Zero
        } else {
            Decimal::Finite(Rational::from_big_rational(
                // FIXME #336: We should be able to use unwrap_unchecked here
                // as we know that the f32 is finite
                BigRational::from_f32(value).unwrap(),
            ))
        }
    }
}

impl From<TypedDecimal> for Decimal {
    fn from(value: TypedDecimal) -> Self {
        match value {
            TypedDecimal::F32(ordered_float) => {
                Decimal::from(ordered_float.into_inner())
            }
            TypedDecimal::F64(ordered_float) => {
                Decimal::from(ordered_float.into_inner())
            }
            TypedDecimal::Decimal(big_decimal) => big_decimal,
        }
    }
}

impl From<f64> for Decimal {
    fn from(value: f64) -> Self {
        if value.is_nan() {
            Decimal::NaN
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                Decimal::Infinity
            } else {
                Decimal::NegInfinity
            }
        } else if value.is_zero() && value.is_sign_negative() {
            Decimal::NegZero
        } else if value.is_zero() {
            Decimal::Zero
        } else {
            Decimal::Finite(Rational::from_big_rational(
                BigRational::from_f64(value).unwrap(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::assert_matches::assert_matches;

    #[test]
    fn decimal_addition() {
        let dec1 = Decimal::from_string("12.34").unwrap();
        let dec2 = Decimal::from_string("56.78").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "69.12");

        let dec1 = Decimal::from_string("-12345.678901234536784").unwrap();
        let dec2 = Decimal::from_string("3").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "-12342.678901234536784");

        let dec1 = Decimal::from_string("1/3").unwrap();
        let dec2 = Decimal::from_string("1/3").unwrap();
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "2/3");
    }

    #[test]
    fn formatting() {
        let dec1 = Decimal::from_string("12.34").unwrap();
        assert_eq!(dec1.to_string(), "12.34");

        let dec2 = Decimal::from_string("0.001").unwrap();
        assert_eq!(dec2.to_string(), "0.001");

        let dec3 = Decimal::from_string("-0.001").unwrap();
        assert_eq!(dec3.to_string(), "-0.001");

        let dec4 = Decimal::from_string("Infinity").unwrap();
        assert_eq!(dec4.to_string(), "infinity");

        let dec5 = Decimal::from_string("-Infinity").unwrap();
        assert_eq!(dec5.to_string(), "-infinity");

        let dec6 = Decimal::from_string("NaN").unwrap();
        assert_eq!(dec6.to_string(), "nan");

        let dec7 = Decimal::from_string("1234567").unwrap();
        assert_eq!(dec7.to_string(), "1234567.0");

        let dec8 = Decimal::from_string("-1234567").unwrap();
        assert_eq!(dec8.to_string(), "-1234567.0");

        let dec9 = Decimal::from_string("-0").unwrap();
        assert_eq!(dec9.to_string(), "-0.0");

        let dec10 = Decimal::from_string("0").unwrap();
        assert_eq!(dec10.to_string(), "0.0");

        let dec11 = Decimal::from_string("1/3").unwrap();
        assert_eq!(dec11.to_string(), "1/3");

        let dec12 = Decimal::from_string("-1/3").unwrap();
        assert_eq!(dec12.to_string(), "-1/3");

        let dec13 = Decimal::from_string("1/2").unwrap();
        assert_eq!(dec13.to_string(), "0.5");

        let dec14 = Decimal::from_string("824/16").unwrap();
        assert_eq!(dec14.to_string(), "51.5");
    }

    #[test]
    fn zero() {
        let a = Decimal::from(0.0f32);
        assert!(matches!(a, Decimal::Zero));
        assert!(!matches!(a, Decimal::NegZero));

        let b = Decimal::from(0.0f64);
        assert!(matches!(b, Decimal::Zero));
        assert!(!matches!(b, Decimal::NegZero));

        let c = Decimal::from_string("0.0").unwrap();
        assert!(matches!(c, Decimal::Zero));
        assert!(!matches!(c, Decimal::NegZero));
    }

    #[test]
    fn neg_zero() {
        let a = Decimal::from(-0.0f32);
        assert!(matches!(a, Decimal::NegZero));
        assert!(!matches!(a, Decimal::Zero));

        let b = Decimal::from(-0.0f64);
        assert!(matches!(b, Decimal::NegZero));
        assert!(!matches!(b, Decimal::Zero));

        let c = Decimal::from_string("-0.0").unwrap();
        assert!(matches!(c, Decimal::NegZero));
        assert!(!matches!(c, Decimal::Zero));
    }

    #[test]
    fn inf() {
        let a = Decimal::from(f32::INFINITY);
        assert!(matches!(a, Decimal::Infinity));

        let b = Decimal::from(f64::INFINITY);
        assert!(matches!(b, Decimal::Infinity));

        let c = Decimal::from_string("infinity").unwrap();
        assert!(matches!(c, Decimal::Infinity));
    }

    #[test]
    fn neg_inf() {
        let a = Decimal::from(f32::NEG_INFINITY);
        assert!(matches!(a, Decimal::NegInfinity));

        let b = Decimal::from(f64::NEG_INFINITY);
        assert!(matches!(b, Decimal::NegInfinity));

        let c = Decimal::from_string("-infinity").unwrap();
        assert!(matches!(c, Decimal::NegInfinity));
    }

    #[test]
    fn nan() {
        let a = Decimal::from(f32::NAN);
        assert!(matches!(a, Decimal::NaN));

        let b = Decimal::from(f64::NAN);
        assert!(matches!(b, Decimal::NaN));

        let c = Decimal::from_string("nan").unwrap();
        assert!(matches!(c, Decimal::NaN));

        let a = Decimal::from(-f32::NAN);
        assert!(matches!(a, Decimal::NaN));

        let b = Decimal::from(-f64::NAN);
        assert!(matches!(b, Decimal::NaN));

        let c = Decimal::from_string("-nan").unwrap();
        assert!(matches!(c, Decimal::NaN));
    }

    #[test]
    fn finite() {
        let a = Decimal::from(1.23f32);
        assert!(matches!(a, Decimal::Finite(_)));

        let b = Decimal::from(4.56f64);
        assert!(matches!(b, Decimal::Finite(_)));

        let c = Decimal::from_string("7.89").unwrap();
        assert!(matches!(c, Decimal::Finite(_)));

        let d = Decimal::from_string("-1.23").unwrap();
        assert!(matches!(d, Decimal::Finite(_)));
    }

    #[test]
    fn zero_neg_zero() {
        let a = Decimal::from_string("0.0").unwrap();
        let b = Decimal::from_string("-0.0").unwrap();
        assert_matches!(a, Decimal::Zero);
        assert_matches!(b, Decimal::NegZero);
        assert_eq!(a, b);
    }

    #[test]
    fn nan_eq() {
        // implicit big decimal NaN
        let a = Decimal::from_string("nan").unwrap();
        let b = Decimal::from_string("nan").unwrap();
        // partial equality for nan values
        assert_eq!(a, b);
        // no structural equality for nan values
        assert!(!a.structural_eq(&b));
        // no value equality for nan values
        assert!(!a.value_eq(&b));

        // explicit big decimal NaN
        let c = Decimal::NaN;
        let d = Decimal::NaN;
        // partial equality for nan values
        assert_eq!(c, d);
        // no structural equality for nan values
        assert!(!c.structural_eq(&d));
        // no value equality for nan values
        assert!(!c.value_eq(&d));

        // f32 NaN
        let e = Decimal::from(f32::NAN);
        let f = Decimal::from(f32::NAN);
        assert_eq!(e, f);
        assert!(!e.structural_eq(&f));
        assert!(!e.value_eq(&f));

        // f64 NaN
        let g = Decimal::from(f64::NAN);
        let h = Decimal::from(f64::NAN);
        assert_eq!(g, h);
        assert!(!g.structural_eq(&h));
        assert!(!g.value_eq(&h));

        // structural equality
        assert!(!a.structural_eq(&c));
        assert!(!a.structural_eq(&e));
        assert!(!a.structural_eq(&g));
        assert!(!a.structural_eq(&h));
        assert!(!b.structural_eq(&c));
        assert!(!b.structural_eq(&e));
        assert!(!b.structural_eq(&g));
        assert!(!b.structural_eq(&h));
        assert!(!c.structural_eq(&e));
        assert!(!c.structural_eq(&g));
        assert!(!c.structural_eq(&h));
        assert!(!d.structural_eq(&e));
        assert!(!d.structural_eq(&g));
        assert!(!d.structural_eq(&h));
        assert!(!e.structural_eq(&g));
        assert!(!e.structural_eq(&h));
        assert!(!f.structural_eq(&g));
        assert!(!f.structural_eq(&h));
        assert!(!g.structural_eq(&h));
    }

    #[test]
    fn zero_eq() {
        let a = Decimal::from_string("0.0").unwrap();
        let b = Decimal::from_string("0.0").unwrap();
        let c = Decimal::from_string("-0.0").unwrap();

        assert_eq!(a, b);
        assert_eq!(a, c);

        assert!(a.structural_eq(&b));
        assert!(a.structural_eq(&c));
        assert!(b.structural_eq(&c));
        assert!(a.value_eq(&b));
        assert!(a.value_eq(&c));
        assert!(b.value_eq(&c));
    }

    #[test]
    fn equality() {
        let a = Decimal::from_string("1.0").unwrap();
        let b = Decimal::from_string("1.0").unwrap();
        let c = Decimal::from_string("2.0").unwrap();
        assert!(a.structural_eq(&b));
        assert!(!a.structural_eq(&c));
        assert!(!b.structural_eq(&c));

        let d = Decimal::from_string("infinity").unwrap();
        let e = Decimal::from_string("-infinity").unwrap();
        assert!(d.structural_eq(&Decimal::from_string("infinity").unwrap()));
        assert!(e.structural_eq(&Decimal::from_string("-infinity").unwrap()));
    }

    #[test]
    fn decimal_addition_2() {
        let a = Decimal::from_string("1.0").unwrap();
        let b = Decimal::from_string("2.0").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("3.0").unwrap());

        let c = Decimal::from_string("1.5").unwrap();
        let d = Decimal::from_string("2.5").unwrap();
        let result2 = c + d;
        assert_eq!(result2, Decimal::from(4.0));

        let e = Decimal::from_string("0.1").unwrap();
        let f = Decimal::from_string("0.2").unwrap();
        let result3 = &e + &f;
        assert_eq!(result3, Decimal::from_string("0.3").unwrap());
    }

    #[test]
    fn infinity_calculations() {
        let a = Decimal::from_string("1.0").unwrap();
        let b = Decimal::from_string("infinity").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("infinity").unwrap());

        let a = Decimal::from_string("infinity").unwrap();
        let b = Decimal::from_string("-infinity").unwrap();
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("infinity").unwrap();
        let b = Decimal::from_string("-0.0").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("infinity").unwrap());

        let a = Decimal::from_string("-infinity").unwrap();
        let b = Decimal::from_string("0.0").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("-infinity").unwrap());

        let a = Decimal::from_string("0.0").unwrap();
        let b = Decimal::from_string("-0.0").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("0.0").unwrap());

        let a = Decimal::from_string("-0.0").unwrap();
        let b = Decimal::from_string("0.0").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("0.0").unwrap());

        let a = Decimal::from_string("nan").unwrap();
        let b = Decimal::from_string("1.0").unwrap();
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("1.0").unwrap();
        let b = Decimal::from_string("nan").unwrap();
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("nan").unwrap();
        let b = Decimal::from_string("nan").unwrap();
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("-nan").unwrap();
        let b = Decimal::from_string("1.0").unwrap();
        let result = a + b;
        assert!(result.is_nan());
    }

    #[test]
    fn large_decimal_addition() {
        let a =
            Decimal::from_string("100000000000000000000.00000000000000000001")
                .unwrap();
        let b =
            Decimal::from_string("100000000000000000000.00000000000000000001")
                .unwrap();
        let result = a + b;
        assert_eq!(
            result,
            Decimal::from_string("200000000000000000000.00000000000000000002")
                .unwrap()
        );
    }

    #[test]
    fn e_notation_decimal_addition() {
        let a = Decimal::from_string("1e10").unwrap();
        let b = Decimal::from_string("2e10").unwrap();
        let result = a + b;
        assert_eq!(result, Decimal::from_string("3e10").unwrap());

        let c = Decimal::from_string("1.5e10").unwrap();
        let d = Decimal::from_string("2.5e10").unwrap();
        let result2 = c + d;
        assert_eq!(result2, Decimal::from_string("4e10").unwrap());
    }
}
