use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::error::NumberParseError;
use crate::values::{
    core_value_trait::CoreValueTrait, traits::structural_eq::StructuralEq,
};
use num::Signed;
use num_traits::Zero;
use ordered_float::OrderedFloat;
use std::hash::Hash;
use std::num::ParseFloatError;
use std::ops::Neg;
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub},
};

use strum_macros::{AsRefStr, EnumIter, EnumString};

/// The decimal type variants to be used as a inline
/// definition in DATEX (such as 42.4f32 or -42.4f32).
/// Note that changing the enum variants will change
/// the way decimals are parsed in DATEX scripts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum DecimalTypeVariant {
    F32,
    F64,
    Big,
}

// TODO #130: think about hash keys for NaN
#[derive(Debug, Clone, Eq)]
pub enum TypedDecimal {
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
    Decimal(Decimal),
}

// TODO #131: this is only a temporary solution to make clippy happy
impl Hash for TypedDecimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypedDecimal::F32(value) => {
                value.into_inner().to_bits().hash(state)
            }
            TypedDecimal::F64(value) => {
                value.into_inner().to_bits().hash(state)
            }
            TypedDecimal::Decimal(value) => value.hash(state),
        }
    }
}

impl CoreValueTrait for TypedDecimal {}

impl StructuralEq for TypedDecimal {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypedDecimal::F32(a), TypedDecimal::F32(b)) => {
                a.into_inner() == b.into_inner()
            }
            (TypedDecimal::F64(a), TypedDecimal::F64(b)) => {
                a.into_inner() == b.into_inner()
            }
            (TypedDecimal::F32(a), TypedDecimal::F64(b))
            | (TypedDecimal::F64(b), TypedDecimal::F32(a)) => {
                a.into_inner() as f64 == b.into_inner()
            }
            (TypedDecimal::Decimal(a), TypedDecimal::Decimal(b)) => {
                a.structural_eq(b)
            }
            (a, TypedDecimal::Decimal(b)) | (TypedDecimal::Decimal(b), a) => {
                match a {
                    TypedDecimal::F32(value) => {
                        b.structural_eq(&Decimal::from(value.into_inner()))
                    }
                    TypedDecimal::F64(value) => {
                        b.structural_eq(&Decimal::from(value.into_inner()))
                    }
                    _ => false,
                }
            }
        }
    }
}

impl PartialEq for TypedDecimal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // F32 and F32
            (TypedDecimal::F32(a), TypedDecimal::F32(b)) => {
                a.into_inner() == b.into_inner()
            }
            // F64 and F64
            (TypedDecimal::F64(a), TypedDecimal::F64(b)) => {
                a.into_inner() == b.into_inner()
            }
            // Big and Big
            (TypedDecimal::Decimal(a), TypedDecimal::Decimal(b)) => a == b,
            _ => false,
        }
    }
}

fn parse_checked_f32(s: &str) -> Result<f32, NumberParseError> {
    let v: f64 = s
        .parse()
        .map_err(|e: ParseFloatError| NumberParseError::InvalidFormat)?;
    if v > f32::MAX as f64 || v < f32::MIN as f64 {
        return Err(NumberParseError::OutOfRange);
    }
    Ok(v as f32)
}

fn parse_checked_f64(s: &str) -> Result<f64, NumberParseError> {
    let v = Decimal::from_string(s);
    let res = v.try_into_f64();
    if let Some(v) = res {
        if v.is_finite() {
            Ok(v)
        } else {
            Err(NumberParseError::OutOfRange)
        }
    } else {
        Err(NumberParseError::OutOfRange)
    }
}

impl TypedDecimal {
    /// Creates a TypedDecimal from a string and a variant, ensuring the value is within the valid range.
    /// Returns an error if the value is out of range or cannot be parsed.
    /// Note: This function does not support Decimal syntax, as it can represent any valid decimal
    /// value without range limitations.
    pub fn from_string_and_variant_in_range(
        value: &str,
        variant: DecimalTypeVariant,
    ) -> Result<Self, NumberParseError> {
        match variant {
            DecimalTypeVariant::F32 => parse_checked_f32(value)
                .map(|v| TypedDecimal::F32(OrderedFloat(v))),
            DecimalTypeVariant::F64 => parse_checked_f64(value)
                .map(|v| TypedDecimal::F64(OrderedFloat(v))),
            DecimalTypeVariant::Big => {
                Ok(TypedDecimal::Decimal(Decimal::from_string(value)))
            }
        }
    }

    /// Creates a TypedDecimal from a string and a variant.
    /// Returns an error if the value cannot be parsed.
    /// Note: This function does not check for range limitations, so it may produce
    /// NaN or Infinity for f32 and f64 variants.
    pub fn from_string_and_variant(
        value: &str,
        variant: DecimalTypeVariant,
    ) -> Result<Self, NumberParseError> {
        match variant {
            DecimalTypeVariant::F32 => value
                .parse::<f32>()
                .map(|v| TypedDecimal::F32(OrderedFloat(v)))
                .map_err(|_: ParseFloatError| NumberParseError::InvalidFormat),
            DecimalTypeVariant::F64 => value
                .parse::<f64>()
                .map(|v| TypedDecimal::F64(OrderedFloat(v)))
                .map_err(|_: ParseFloatError| NumberParseError::InvalidFormat),
            DecimalTypeVariant::Big => {
                Ok(TypedDecimal::Decimal(Decimal::from_string(value)))
            }
        }
    }

    /// Converts the TypedDecimal to f32, potentially losing precision.
    /// Returns NaN if the value cannot be represented as f32.
    pub fn as_f32(&self) -> f32 {
        match self {
            TypedDecimal::F32(value) => value.into_inner(),
            TypedDecimal::F64(value) => value.into_inner() as f32,
            TypedDecimal::Decimal(value) => {
                value.try_into_f32().unwrap_or(f32::NAN)
            }
        }
    }

    /// Converts the TypedDecimal to f64, potentially losing precision.
    /// Returns NaN if the value cannot be represented as f64.
    pub fn as_f64(&self) -> f64 {
        match self {
            TypedDecimal::F32(value) => value.into_inner() as f64,
            TypedDecimal::F64(value) => value.into_inner(),
            TypedDecimal::Decimal(value) => {
                value.try_into_f64().unwrap_or(f64::NAN)
            }
        }
    }

    /// Returns true if the value is zero (positive or negative).
    pub fn is_zero(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.into_inner().is_zero(),
            TypedDecimal::F64(value) => value.into_inner().is_zero(),
            TypedDecimal::Decimal(value) => {
                value == &Decimal::Zero || value == &Decimal::NegZero
            }
        }
    }

    /// Returns true if the value can be represented as an exact integer in the range of i64.
    pub fn is_integer(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => {
                value.into_inner() as f64 >= i64::MIN as f64
                    && value.into_inner() as f64 <= i64::MAX as f64
                    && value.into_inner().fract() == 0.0
            }
            TypedDecimal::F64(value) => {
                value.into_inner() >= i64::MIN as f64
                    && value.into_inner() <= i64::MAX as f64
                    && value.into_inner().fract() == 0.0
            }
            TypedDecimal::Decimal(value) => match value {
                Decimal::Finite(big_value) => {
                    big_value.is_integer()
                        && big_value.to_f64().unwrap_or(f64::NAN).is_finite()
                }
                Decimal::Zero => true,
                Decimal::NegZero => true,
                Decimal::Infinity => false,
                Decimal::NegInfinity => false,
                Decimal::NaN => false,
            },
        }
    }

    /// Returns true if the value is finite (not NaN or Infinity).
    pub fn is_finite(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.into_inner().is_finite(),
            TypedDecimal::F64(value) => value.into_inner().is_finite(),
            TypedDecimal::Decimal(value) => value.is_finite(),
        }
    }

    /// Returns true if the value is infinite (positive or negative).
    pub fn is_infinite(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.into_inner().is_infinite(),
            TypedDecimal::F64(value) => value.into_inner().is_infinite(),
            TypedDecimal::Decimal(value) => {
                matches!(value, Decimal::Infinity | Decimal::NegInfinity)
            }
        }
    }

    /// Returns the value as an integer if it is an exact integer, otherwise returns None.
    pub fn as_integer(&self) -> Option<i64> {
        if self.is_integer() {
            match self {
                TypedDecimal::F32(value) => Some(value.into_inner() as i64),
                TypedDecimal::F64(value) => Some(value.into_inner() as i64),
                TypedDecimal::Decimal(value) => match value {
                    Decimal::Finite(big_value) => big_value.to_i64(),
                    Decimal::Zero => Some(0),
                    Decimal::NegZero => Some(-0),
                    _ => unreachable!("Not an integer"), // should not happen due to is_integer check
                },
            }
        } else {
            None
        }
    }

    /// Returns true if the TypedDecimal is of variant F32.
    pub fn is_f32(&self) -> bool {
        matches!(self, TypedDecimal::F32(_))
    }

    /// Returns true if the TypedDecimal is of variant F64.
    pub fn is_f64(&self) -> bool {
        matches!(self, TypedDecimal::F64(_))
    }

    /// Returns true if the TypedDecimal is of variant Decimal.
    pub fn is_positive(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_positive(),
            TypedDecimal::F64(value) => value.is_positive(),
            TypedDecimal::Decimal(value) => match value {
                Decimal::Finite(big_value) => big_value.is_positive(),
                Decimal::Zero => true,
                Decimal::NegZero => false,
                Decimal::Infinity => true,
                Decimal::NegInfinity => false,
                Decimal::NaN => false,
            },
        }
    }

    /// Returns true if the value is negative (strictly less than zero).
    pub fn is_negative(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_negative(),
            TypedDecimal::F64(value) => value.is_negative(),
            TypedDecimal::Decimal(value) => match value {
                Decimal::Finite(big_value) => big_value.is_negative(),
                Decimal::Zero => false,
                Decimal::NegZero => true,
                Decimal::Infinity => false,
                Decimal::NegInfinity => true,
                Decimal::NaN => false,
            },
        }
    }

    /// Returns true if the value is NaN (Not a Number).
    pub fn is_nan(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_nan(),
            TypedDecimal::F64(value) => value.is_nan(),
            TypedDecimal::Decimal(value) => matches!(value, Decimal::NaN),
        }
    }
}

impl Display for TypedDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedDecimal::F32(value) => write!(f, "{}", value.into_inner()),
            TypedDecimal::F64(value) => write!(f, "{}", value.into_inner()),
            TypedDecimal::Decimal(value) => write!(f, "{value}"),
        }
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
                TypedDecimal::Decimal(b) => {
                    let result = Decimal::from(a.into_inner()) + b;
                    if let Some(result_f32) = result.try_into_f32() {
                        TypedDecimal::F32(result_f32.into())
                    } else {
                        TypedDecimal::F32(f32::NAN.into())
                    }
                }
            },
            TypedDecimal::F64(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F64(OrderedFloat(
                    a.into_inner() + b.into_inner() as f64,
                )),
                TypedDecimal::F64(b) => TypedDecimal::F64(a + b),
                TypedDecimal::Decimal(b) => {
                    let result = Decimal::from(a.into_inner()) + b;
                    if let Some(result_f64) = result.try_into_f64() {
                        TypedDecimal::F64(result_f64.into())
                    } else {
                        TypedDecimal::F64(f64::NAN.into())
                    }
                }
            },
            TypedDecimal::Decimal(a) => {
                TypedDecimal::Decimal(a + Decimal::from(rhs))
            }
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
        // negate rhs
        let negated_rhs = match rhs {
            TypedDecimal::F32(value) => TypedDecimal::F32(value.neg()),
            TypedDecimal::F64(value) => TypedDecimal::F64(value.neg()),
            TypedDecimal::Decimal(value) => TypedDecimal::Decimal(value.neg()),
        };

        // perform addition with negated rhs
        TypedDecimal::add(self, negated_rhs)
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

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;
    use crate::values::core_values::decimal::decimal::Decimal;
    use ordered_float::OrderedFloat;

    #[test]
    fn zero_sign() {
        let c = TypedDecimal::from(0.0f32);
        assert_matches!(c, TypedDecimal::F32(_));
        assert!(c.is_positive());
        assert!(!c.is_negative());

        let e = TypedDecimal::from(-0.0f32);
        assert_matches!(e, TypedDecimal::F32(_));
        assert!(!e.is_positive());
        assert!(e.is_negative());

        let f = TypedDecimal::from(0.0f64);
        assert_matches!(f, TypedDecimal::F64(_));
        assert!(f.is_positive());
        assert!(!f.is_negative());

        let g = TypedDecimal::from(-0.0f64);
        assert_matches!(g, TypedDecimal::F64(_));
        assert!(!g.is_positive());
        assert!(g.is_negative());

        let h = TypedDecimal::Decimal(Decimal::from(0.0));
        assert_matches!(h, TypedDecimal::Decimal(Decimal::Zero));
        assert!(h.is_positive());
        assert!(!h.is_negative());

        let i = TypedDecimal::Decimal(Decimal::from(-0.0));
        assert_matches!(i, TypedDecimal::Decimal(Decimal::NegZero));
        assert!(!i.is_positive());
        assert!(i.is_negative());
    }

    #[test]
    fn is_positive() {
        let a = TypedDecimal::from(42.0f32);
        assert_matches!(a, TypedDecimal::F32(_));
        assert!(a.is_positive());

        let b = TypedDecimal::from(-42.0f64);
        assert_matches!(b, TypedDecimal::F64(_));
        assert!(!b.is_positive());

        let d = TypedDecimal::from(0.01f64);
        assert_matches!(d, TypedDecimal::F64(_));
        assert!(d.is_positive());

        let e = TypedDecimal::Decimal(0.0.into());
        assert_matches!(e, TypedDecimal::Decimal(Decimal::Zero));
        assert!(e.is_positive());
    }

    #[test]
    fn is_negative() {
        let a = TypedDecimal::from(-42.0f32);
        assert_matches!(a, TypedDecimal::F32(_));
        assert!(a.is_negative());

        let b = TypedDecimal::from(42.0f64);
        assert_matches!(b, TypedDecimal::F64(_));
        assert!(!b.is_negative());

        let c = TypedDecimal::from(0.0f32);
        assert_matches!(c, TypedDecimal::F32(_));
        assert!(!c.is_negative());

        let d = TypedDecimal::from(-0.01f64);
        assert_matches!(d, TypedDecimal::F64(_));
        assert!(d.is_negative());

        let e = TypedDecimal::from(-0.0f32);
        assert_matches!(e, TypedDecimal::F32(_));
        assert!(e.is_negative());

        let f = TypedDecimal::Decimal((-0.0).into());
        assert_matches!(f, TypedDecimal::Decimal(Decimal::NegZero));
        assert!(f.is_negative());
    }

    #[test]
    fn integer() {
        let a = TypedDecimal::from(42.0f32);
        assert_matches!(a, TypedDecimal::F32(_));
        assert!(a.is_integer());
        assert_eq!(a.as_integer(), Some(42));

        let b = TypedDecimal::from(-42.0f64);
        assert_matches!(b, TypedDecimal::F64(_));
        assert!(b.is_integer());
        assert_eq!(b.as_integer(), Some(-42));

        let c = TypedDecimal::from(0.0f32);
        assert_matches!(c, TypedDecimal::F32(_));
        assert!(c.is_integer());
        assert_eq!(c.as_integer(), Some(0));

        let d = TypedDecimal::from(-0.01f64);
        assert_matches!(d, TypedDecimal::F64(_));
        assert!(!d.is_integer());
        assert_eq!(d.as_integer(), None);
    }

    #[test]
    fn f32() {
        let c = TypedDecimal::from(1.5f32);
        assert_matches!(c, TypedDecimal::F32(OrderedFloat(1.5)));
        assert_eq!(c.as_f32(), 1.5);
        assert_eq!(c.as_f64(), 1.5);
    }

    #[test]
    fn f64() {
        let c = TypedDecimal::from(1.5f64);
        assert_matches!(c, TypedDecimal::F64(OrderedFloat(1.5)));
        assert_eq!(c.as_f32(), 1.5);
        assert_eq!(c.as_f64(), 1.5);
    }

    #[test]
    fn zero_and_neg_zero() {
        let a = TypedDecimal::from(0.0f32);
        assert_matches!(a, TypedDecimal::F32(OrderedFloat(0.0)));

        let a = TypedDecimal::from(-0.0f32);
        assert_matches!(a, TypedDecimal::F32(OrderedFloat(0.0)));

        // f32
        let c = TypedDecimal::F32(0.0f32.into());
        assert_matches!(c, TypedDecimal::F32(OrderedFloat(0.0)));
        assert_eq!(c.as_f32(), 0.0);
        assert_eq!(c.as_f32(), -0.0);
        assert_eq!(c.as_f64(), 0.0);
        assert_eq!(c.as_f64(), -0.0);

        // f64
        let c = TypedDecimal::F64(0.0f64.into());
        assert_matches!(c, TypedDecimal::F64(OrderedFloat(0.0)));
        assert_eq!(c.as_f32(), 0.0);
        assert_eq!(c.as_f32(), -0.0);
        assert_eq!(c.as_f64(), 0.0);
        assert_eq!(c.as_f64(), -0.0);

        // big
        let c = TypedDecimal::Decimal(Decimal::from(0.0));
        assert_matches!(c, TypedDecimal::Decimal(Decimal::Zero));

        assert_eq!(c.as_f32(), 0.0);
        assert_eq!(c.as_f32(), -0.0);
        assert_eq!(c.as_f64(), 0.0);
        assert_eq!(c.as_f64(), -0.0);
    }

    #[test]
    fn addition() {
        let a = TypedDecimal::F32(1.5.into());
        let b = TypedDecimal::F64(2.5.into());
        let result = a + b;
        assert_eq!(result.as_f32(), 4.0);
        assert_eq!(result.as_f64(), 4.0);
    }

    #[test]
    fn from_string_and_variant() {
        let a = TypedDecimal::from_string_and_variant(
            "42.0",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert_matches!(a, TypedDecimal::F32(OrderedFloat(42.0)));

        let b = TypedDecimal::from_string_and_variant(
            "42.0",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert_matches!(b, TypedDecimal::F64(OrderedFloat(42.0)));

        let c = TypedDecimal::from_string_and_variant(
            "12345678901234567890.123456789",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert_matches!(c, TypedDecimal::F64(_));

        let d = TypedDecimal::from_string_and_variant(
            "12345678901234567890.123456789",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert_matches!(
            d,
            TypedDecimal::F32(OrderedFloat(12345678901234567890.123456789f32))
        );

        let e = TypedDecimal::from_string_and_variant(
            "not_a_number",
            DecimalTypeVariant::F32,
        );
        assert!(e.is_err());

        let f = TypedDecimal::from_string_and_variant(
            "not_a_number",
            DecimalTypeVariant::F64,
        );
        assert!(f.is_err());

        let g = TypedDecimal::from_string_and_variant(
            "NaN",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert!(g.is_nan());

        let h = TypedDecimal::from_string_and_variant(
            "nan",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert!(h.is_nan());

        let i = TypedDecimal::from_string_and_variant(
            "Infinity",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert!(i.is_infinite() && i.is_positive());

        let j = TypedDecimal::from_string_and_variant(
            "-infinity",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert!(j.is_infinite() && j.is_negative());

        let k = TypedDecimal::from_string_and_variant(
            "12345678901234567890.123456789",
            DecimalTypeVariant::Big,
        )
        .unwrap();
        assert_matches!(k, TypedDecimal::Decimal(_));
        assert_eq!(k.as_f64(), 12345678901234567890.123456789);
    }

    #[test]
    fn from_string_and_variant_in_range() {
        let a = TypedDecimal::from_string_and_variant_in_range(
            "1e40",
            DecimalTypeVariant::F32,
        );
        assert!(a.is_err());
        assert_eq!(a.err().unwrap(), NumberParseError::OutOfRange);

        let b = TypedDecimal::from_string_and_variant_in_range(
            "-1e40",
            DecimalTypeVariant::F32,
        );
        assert!(b.is_err());
        assert_eq!(b.err().unwrap(), NumberParseError::OutOfRange);

        let c = TypedDecimal::from_string_and_variant_in_range(
            "1e1000",
            DecimalTypeVariant::F64,
        );
        assert!(c.is_err());
        assert_eq!(c.err().unwrap(), NumberParseError::OutOfRange);

        let d = TypedDecimal::from_string_and_variant_in_range(
            "-1e1000",
            DecimalTypeVariant::F64,
        );
        assert!(d.is_err());
        assert_eq!(d.err().unwrap(), NumberParseError::OutOfRange);
    }
}
