use super::rational::Rational;
use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::datex_values::traits::soft_eq::SoftEq;
use bigdecimal::BigDecimal;
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, Endian};
use num::BigInt;
use num::BigRational;
use num_enum::TryFromPrimitive;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::io::{Read, Seek};
use std::ops::{Add, Neg, Sub};
use std::str::FromStr;

#[derive(Debug, Clone, Eq)]
pub enum Decimal {
    Finite(Rational),
    NaN,
    Zero,
    NegZero,
    Infinity,
    NegInfinity,
}

// TODO: this is only a temporary solution to make clippy happy
impl Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Decimal::Finite(value) => value.hash(state),
            Decimal::NaN => 0.hash(state),
            Decimal::Zero => 1.hash(state),
            Decimal::NegZero => 2.hash(state),
            Decimal::Infinity => 3.hash(state),
            Decimal::NegInfinity => 4.hash(state),
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Decimal::Finite(a), Decimal::Finite(b)) => a == b,
            (Decimal::Zero, Decimal::Zero)
            | (Decimal::NegZero, Decimal::NegZero)
            | (Decimal::Zero, Decimal::NegZero)
            | (Decimal::NegZero, Decimal::Zero) => true,
            (Decimal::Infinity, Decimal::Infinity) => true,
            (Decimal::NegInfinity, Decimal::NegInfinity) => true,
            (Decimal::NaN, Decimal::NaN) => false,
            _ => false,
        }
    }
}

impl Decimal {
    pub fn try_into_f32(&self) -> Option<f32> {
        match self {
            Decimal::Finite(value) => value.to_f32(),
            Decimal::Zero => Some(0.0),
            Decimal::NegZero => Some(-0.0),
            Decimal::Infinity => Some(f32::INFINITY),
            Decimal::NegInfinity => Some(f32::NEG_INFINITY),
            Decimal::NaN => None,
        }
    }
    pub fn try_into_f64(&self) -> Option<f64> {
        match self {
            Decimal::Finite(value) => value.to_f64(),
            Decimal::Zero => Some(0.0),
            Decimal::NegZero => Some(-0.0),
            Decimal::Infinity => Some(f64::INFINITY),
            Decimal::NegInfinity => Some(f64::NEG_INFINITY),
            Decimal::NaN => None,
        }
    }

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

    pub fn from_fraction(numerator: &str, denominator: &str) -> Self {
        let rational = BigRational::new(
            BigInt::from_str(numerator).unwrap(),
            BigInt::from_str(denominator).unwrap(),
        );
        Decimal::from(Rational::from_big_rational(rational))
    }

    pub fn from_string(s: &str) -> Self {
        // TODO represent as Infinity/-Infinity if out of bounds for representable DATEX values
        match s {
            "Infinity" | "infinity" => Decimal::Infinity,
            "-Infinity" | "-infinity" => Decimal::NegInfinity,
            "nan" | "NaN" | "-nan" | "-NaN" => Decimal::NaN,
            _ => {
                if s.contains("/") {
                    // If the string contains a fraction, parse it as a fraction
                    let parts: Vec<&str> = s.split('/').collect();
                    if parts.len() == 2 {
                        Decimal::from_fraction(parts[0], parts[1])
                    } else {
                        Decimal::NaN
                    }
                } else {
                    let big_rational = Decimal::parse_decimal_to_rational(s);
                    match big_rational {
                        Some(big_rational) => {
                            if big_rational.is_zero() {
                                if s.starts_with('-') {
                                    Decimal::NegZero
                                } else {
                                    Decimal::Zero
                                }
                            } else {
                                Decimal::Finite(Rational::from_big_rational(
                                    big_rational,
                                ))
                            }
                        }
                        None => Decimal::NaN,
                    }
                }
            }
        }
    }

    pub fn is_nan(&self) -> bool {
        matches!(self, Decimal::NaN)
    }
}

impl SoftEq for Decimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Decimal::Finite(a), Decimal::Finite(b)) => a == b,
            (Decimal::Zero, Decimal::Zero) => true,
            (Decimal::NegZero, Decimal::NegZero) => true,
            (Decimal::Infinity, Decimal::Infinity) => true,
            (Decimal::NegInfinity, Decimal::NegInfinity) => true,
            (Decimal::NaN, Decimal::NaN) => false,
            _ => false,
        }
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
        Decimal::sub(self.clone(), rhs.clone())
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decimal::Finite(value) => write!(f, "{value}"),
            Decimal::NaN => write!(f, "nan"),
            Decimal::Zero => write!(f, "0.0"),
            Decimal::NegZero => write!(f, "-0.0"),
            Decimal::Infinity => write!(f, "infinity"),
            Decimal::NegInfinity => write!(f, "-infinity"),
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
    use std::assert_matches::assert_matches;

    #[test]
    fn test_decimal_addition() {
        let dec1 = Decimal::from_string("12.34");
        let dec2 = Decimal::from_string("56.78");
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "69.12");

        let dec1 = Decimal::from_string("-12345.678901234536784");
        let dec2 = Decimal::from_string("3");
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "-12342.678901234536784");

        let dec1 = Decimal::from_string("1/3");
        let dec2 = Decimal::from_string("1/3");
        let result = dec1 + dec2;
        assert_eq!(result.to_string(), "2/3");
    }

    #[test]
    fn test_formatting() {
        let dec1 = Decimal::from_string("12.34");
        assert_eq!(dec1.to_string(), "12.34");

        let dec2 = Decimal::from_string("0.001");
        assert_eq!(dec2.to_string(), "0.001");

        let dec3 = Decimal::from_string("-0.001");
        assert_eq!(dec3.to_string(), "-0.001");

        let dec4 = Decimal::from_string("Infinity");
        assert_eq!(dec4.to_string(), "infinity");

        let dec5 = Decimal::from_string("-Infinity");
        assert_eq!(dec5.to_string(), "-infinity");

        let dec6 = Decimal::from_string("NaN");
        assert_eq!(dec6.to_string(), "nan");

        let dec7 = Decimal::from_string("1234567");
        assert_eq!(dec7.to_string(), "1234567.0");

        let dec8 = Decimal::from_string("-1234567");
        assert_eq!(dec8.to_string(), "-1234567.0");

        let dec9 = Decimal::from_string("-0");
        assert_eq!(dec9.to_string(), "-0.0");

        let dec10 = Decimal::from_string("0");
        assert_eq!(dec10.to_string(), "0.0");

        let dec11 = Decimal::from_string("1/3");
        assert_eq!(dec11.to_string(), "1/3");

        let dec12 = Decimal::from_string("-1/3");
        assert_eq!(dec12.to_string(), "-1/3");

        let dec13 = Decimal::from_string("1/2");
        assert_eq!(dec13.to_string(), "0.5");

        let dec14 = Decimal::from_string("824/16");
        assert_eq!(dec14.to_string(), "51.5");
    }

    #[test]
    fn test_zero() {
        let a = Decimal::from(0.0f32);
        assert!(matches!(a, Decimal::Zero));
        assert!(!matches!(a, Decimal::NegZero));

        let b = Decimal::from(0.0f64);
        assert!(matches!(b, Decimal::Zero));
        assert!(!matches!(b, Decimal::NegZero));

        let c = Decimal::from_string("0.0");
        assert!(matches!(c, Decimal::Zero));
        assert!(!matches!(c, Decimal::NegZero));
    }

    #[test]
    fn test_neg_zero() {
        let a = Decimal::from(-0.0f32);
        assert!(matches!(a, Decimal::NegZero));
        assert!(!matches!(a, Decimal::Zero));

        let b = Decimal::from(-0.0f64);
        assert!(matches!(b, Decimal::NegZero));
        assert!(!matches!(b, Decimal::Zero));

        let c = Decimal::from_string("-0.0");
        assert!(matches!(c, Decimal::NegZero));
        assert!(!matches!(c, Decimal::Zero));
    }

    #[test]
    fn test_inf() {
        let a = Decimal::from(f32::INFINITY);
        assert!(matches!(a, Decimal::Infinity));

        let b = Decimal::from(f64::INFINITY);
        assert!(matches!(b, Decimal::Infinity));

        let c = Decimal::from_string("infinity");
        assert!(matches!(c, Decimal::Infinity));
    }

    #[test]
    fn test_neg_inf() {
        let a = Decimal::from(f32::NEG_INFINITY);
        assert!(matches!(a, Decimal::NegInfinity));

        let b = Decimal::from(f64::NEG_INFINITY);
        assert!(matches!(b, Decimal::NegInfinity));

        let c = Decimal::from_string("-infinity");
        assert!(matches!(c, Decimal::NegInfinity));
    }

    #[test]
    fn test_nan() {
        let a = Decimal::from(f32::NAN);
        assert!(matches!(a, Decimal::NaN));

        let b = Decimal::from(f64::NAN);
        assert!(matches!(b, Decimal::NaN));

        let c = Decimal::from_string("nan");
        assert!(matches!(c, Decimal::NaN));

        let a = Decimal::from(-f32::NAN);
        assert!(matches!(a, Decimal::NaN));

        let b = Decimal::from(-f64::NAN);
        assert!(matches!(b, Decimal::NaN));

        let c = Decimal::from_string("-nan");
        assert!(matches!(c, Decimal::NaN));
    }

    #[test]
    fn test_finite() {
        let a = Decimal::from(1.23f32);
        assert!(matches!(a, Decimal::Finite(_)));

        let b = Decimal::from(4.56f64);
        assert!(matches!(b, Decimal::Finite(_)));

        let c = Decimal::from_string("7.89");
        assert!(matches!(c, Decimal::Finite(_)));

        let d = Decimal::from_string("-1.23");
        assert!(matches!(d, Decimal::Finite(_)));
    }

    #[test]
    fn test_zero_neg_zero() {
        let a = Decimal::from_string("0.0");
        let b = Decimal::from_string("-0.0");
        assert_matches!(a, Decimal::Zero);
        assert_matches!(b, Decimal::NegZero);
        assert_eq!(a, b);
    }

    #[test]
    fn test_nan_eq() {
        // implicit big decimal NaN
        let a = Decimal::from_string("nan");
        let b = Decimal::from_string("nan");
        assert_ne!(a, b);
        assert!(!a.soft_eq(&b));

        // explicit big decimal NaN
        let c = Decimal::NaN;
        let d = Decimal::NaN;
        assert_ne!(c, d);
        assert!(!c.soft_eq(&d));

        // f32 NaN
        let e = Decimal::from(f32::NAN);
        let f = Decimal::from(f32::NAN);
        assert_ne!(e, f);
        assert!(!e.soft_eq(&f));

        // f64 NaN
        let g = Decimal::from(f64::NAN);
        let h = Decimal::from(f64::NAN);
        assert_ne!(g, h);
        assert!(!g.soft_eq(&h));

        // eq
        assert_ne!(a, c);
        assert_ne!(a, e);
        assert_ne!(a, g);
        assert_ne!(a, h);
        assert_ne!(b, c);
        assert_ne!(b, e);
        assert_ne!(b, g);
        assert_ne!(b, h);
        assert_ne!(c, e);
        assert_ne!(c, g);
        assert_ne!(c, h);
        assert_ne!(d, e);
        assert_ne!(d, g);
        assert_ne!(d, h);
        assert_ne!(e, g);
        assert_ne!(e, h);
        assert_ne!(f, g);
        assert_ne!(f, h);
        assert_ne!(g, h);

        // soft_eq
        assert!(!a.soft_eq(&c));
        assert!(!a.soft_eq(&e));
        assert!(!a.soft_eq(&g));
        assert!(!a.soft_eq(&h));
        assert!(!b.soft_eq(&c));
        assert!(!b.soft_eq(&e));
        assert!(!b.soft_eq(&g));
        assert!(!b.soft_eq(&h));
        assert!(!c.soft_eq(&e));
        assert!(!c.soft_eq(&g));
        assert!(!c.soft_eq(&h));
        assert!(!d.soft_eq(&e));
        assert!(!d.soft_eq(&g));
        assert!(!d.soft_eq(&h));
        assert!(!e.soft_eq(&g));
        assert!(!e.soft_eq(&h));
        assert!(!f.soft_eq(&g));
        assert!(!f.soft_eq(&h));
        assert!(!g.soft_eq(&h));
    }

    #[test]
    fn test_zero_eq() {
        let a = Decimal::from_string("0.0");
        let b = Decimal::from_string("0.0");
        let c = Decimal::from_string("-0.0");

        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn test_equality() {
        let a = Decimal::from_string("1.0");
        let b = Decimal::from_string("1.0");
        let c = Decimal::from_string("2.0");
        assert!(a.soft_eq(&b));
        assert!(!a.soft_eq(&c));
        assert!(!b.soft_eq(&c));

        let d = Decimal::from_string("infinity");
        let e = Decimal::from_string("-infinity");
        assert!(d.soft_eq(&Decimal::from_string("infinity")));
        assert!(e.soft_eq(&Decimal::from_string("-infinity")));
    }

    #[test]
    fn test_decimal_addition_2() {
        let a = Decimal::from_string("1.0");
        let b = Decimal::from_string("2.0");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("3.0"));

        let c = Decimal::from_string("1.5");
        let d = Decimal::from_string("2.5");
        let result2 = c + d;
        assert_eq!(result2, Decimal::from(4.0));

        let e = Decimal::from_string("0.1");
        let f = Decimal::from_string("0.2");
        let result3 = &e + &f;
        assert_eq!(result3, Decimal::from_string("0.3"));
    }

    #[test]
    fn test_infinity_calculations() {
        let a = Decimal::from_string("1.0");
        let b = Decimal::from_string("infinity");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("infinity"));

        let a = Decimal::from_string("infinity");
        let b = Decimal::from_string("-infinity");
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("infinity");
        let b = Decimal::from_string("-0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("infinity"));

        let a = Decimal::from_string("-infinity");
        let b = Decimal::from_string("0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("-infinity"));

        let a = Decimal::from_string("0.0");
        let b = Decimal::from_string("-0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("0.0"));

        let a = Decimal::from_string("-0.0");
        let b = Decimal::from_string("0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("0.0"));

        let a = Decimal::from_string("nan");
        let b = Decimal::from_string("1.0");
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("1.0");
        let b = Decimal::from_string("nan");
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("nan");
        let b = Decimal::from_string("nan");
        let result = a + b;
        assert!(result.is_nan());

        let a = Decimal::from_string("-nan");
        let b = Decimal::from_string("1.0");
        let result = a + b;
        assert!(result.is_nan());
    }

    #[test]
    fn test_large_decimal_addition() {
        let a =
            Decimal::from_string("100000000000000000000.00000000000000000001");
        let b =
            Decimal::from_string("100000000000000000000.00000000000000000001");
        let result = a + b;
        assert_eq!(
            result,
            Decimal::from_string("200000000000000000000.00000000000000000002")
        );
    }

    #[test]
    fn test_e_notation_decimal_addition() {
        let a = Decimal::from_string("1e10");
        let b = Decimal::from_string("2e10");
        let result = a + b;
        assert_eq!(result, Decimal::from_string("3e10"));

        let c = Decimal::from_string("1.5e10");
        let d = Decimal::from_string("2.5e10");
        let result2 = c + d;
        assert_eq!(result2, Decimal::from_string("4e10"));
    }
}
