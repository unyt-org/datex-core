use crate::libs::core::CoreLibPointerId;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::values::core_value_trait::CoreValueTrait;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::error::NumberParseError;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use num_traits::Zero;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::num::ParseFloatError;
use std::ops::Neg;
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub},
};
use strum::Display;
use strum_macros::{AsRefStr, EnumIter, EnumString};

/// The decimal type variants to be used as a inline
/// definition in DATEX (such as 42.4f32 or -42.4f32).
/// Note that changing the enum variants will change
/// the way decimals are parsed in DATEX scripts.
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
)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum DecimalTypeVariant {
    F32 = 1, // rationale: We need to start with 1 here, as the core lib pointer id for the base type is using OFFSET_X + variant as index
    F64,
    Big,
}

#[derive(Debug, Clone, Eq)]
pub enum TypedDecimal {
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
    Decimal(Decimal),
}

impl Serialize for TypedDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TypedDecimal::F32(value) => {
                serializer.serialize_f32(value.into_inner())
            }
            TypedDecimal::F64(value) => {
                // FIXME: Improve serialization, as this can take references instead of copying (maybe :D)
                serializer.serialize_f64(value.into_inner())
            }
            TypedDecimal::Decimal(value) => value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for TypedDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TypedDecimal::from_string(&s).map_err(serde::de::Error::custom)
    }
}

impl Hash for TypedDecimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypedDecimal::F32(value) => {
                // hash -0.0 and 0.0 to the same value
                if value.into_inner() == 0.0 {
                    0.0f32.to_bits().hash(state)
                }
                // normal hash
                else {
                    value.into_inner().to_bits().hash(state)
                }
            }
            TypedDecimal::F64(value) => {
                // hash -0.0 and 0.0 to the same value
                if value.into_inner() == 0.0 {
                    0.0f64.to_bits().hash(state);
                }
                // normal hash
                else {
                    value.into_inner().to_bits().hash(state)
                }
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

impl ValueEq for TypedDecimal {
    fn value_eq(&self, other: &Self) -> bool {
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
            (TypedDecimal::Decimal(a), TypedDecimal::Decimal(b)) => {
                a.value_eq(b)
            }
            _ => false,
        }
    }
}

impl PartialEq for TypedDecimal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // F32 and F32
            (TypedDecimal::F32(a), TypedDecimal::F32(b)) => {
                let a = a.into_inner();
                let b = b.into_inner();
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            // F64 and F64
            (TypedDecimal::F64(a), TypedDecimal::F64(b)) => {
                let a = a.into_inner();
                let b = b.into_inner();
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            // Big and Big
            (TypedDecimal::Decimal(a), TypedDecimal::Decimal(b)) => a == b,
            _ => false,
        }
    }
}

impl From<&TypedDecimal> for CoreLibPointerId {
    fn from(value: &TypedDecimal) -> Self {
        CoreLibPointerId::Decimal(Some(value.variant()))
    }
}

/// Parses a string into an f32, ensuring the value is finite and within the range of f32.
/// Returns an error if the value is out of range, NaN, or cannot be parsed.
fn parse_checked_f32(s: &str) -> Result<f32, NumberParseError> {
    // handle special cases
    match s {
        "inf" => return Ok(f32::INFINITY),
        "-inf" => return Ok(f32::NEG_INFINITY),
        "NaN" => return Ok(f32::NAN),
        _ => {}
    }

    let v: f64 = s
        .parse()
        .map_err(|_: ParseFloatError| NumberParseError::InvalidFormat)?;
    if v > f32::MAX as f64 || v < f32::MIN as f64 {
        return Err(NumberParseError::OutOfRange);
    }
    Ok(v as f32)
}

/// Parses a string into an f64, ensuring the value is finite and within the range of f64.
/// Returns an error if the value is out of range, NaN, or cannot be parsed.
fn parse_checked_f64(s: &str) -> Result<f64, NumberParseError> {
    // handle special cases
    match s {
        "inf" => return Ok(f64::INFINITY),
        "-inf" => return Ok(f64::NEG_INFINITY),
        "NaN" => return Ok(f64::NAN),
        _ => {}
    }

    let v: Decimal = Decimal::from_string(s)?;
    let res = v.into_f64();
    if res.is_finite() {
        Ok(res)
    } else {
        Err(NumberParseError::OutOfRange)
    }
}

impl TypedDecimal {
    /// Creates a TypedDecimal from a string representation.
    /// Tries f32, then f64, then Big.
    pub fn from_string(value: &str) -> Result<Self, NumberParseError> {
        match value {
            "Infinity" | "infinity" => Ok(f32::INFINITY.into()),
            "-Infinity" | "-infinity" => Ok(f32::NEG_INFINITY.into()),
            "nan" | "NaN" | "-nan" | "-NaN" => Ok(f32::NAN.into()),
            _ => TypedDecimal::from_string_and_variant(
                value,
                DecimalTypeVariant::F32,
            )
            .or_else(|_| {
                TypedDecimal::from_string_and_variant(
                    value,
                    DecimalTypeVariant::F64,
                )
            })
            .or_else(|_| {
                TypedDecimal::from_string_and_variant(
                    value,
                    DecimalTypeVariant::Big,
                )
            }),
        }
    }

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
                Decimal::from_string(value).map(TypedDecimal::Decimal)
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
                Decimal::from_string(value).map(TypedDecimal::Decimal)
            }
        }
    }

    /// Converts the TypedDecimal to f32, potentially losing precision.
    /// Returns NaN if the value cannot be represented as f32.
    pub fn as_f32(&self) -> f32 {
        match self {
            TypedDecimal::F32(value) => value.into_inner(),
            TypedDecimal::F64(value) => value.into_inner() as f32,
            TypedDecimal::Decimal(value) => value.into_f32(),
        }
    }

    /// Converts the TypedDecimal to f64, potentially losing precision.
    /// Returns NaN if the value cannot be represented as f64.
    pub fn as_f64(&self) -> f64 {
        match self {
            TypedDecimal::F32(value) => value.into_inner() as f64,
            TypedDecimal::F64(value) => value.into_inner(),
            TypedDecimal::Decimal(value) => value.into_f64(),
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
                    big_value.is_integer() && big_value.to_f64().is_finite()
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

    /// Returns true if the value is NaN (Not a Number).
    pub fn is_nan(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_nan(),
            TypedDecimal::F64(value) => value.is_nan(),
            TypedDecimal::Decimal(value) => matches!(value, Decimal::NaN),
        }
    }

    /// Returns true if the value has a positive sign.
    pub fn is_sign_positive(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.into_inner().is_sign_positive(),
            TypedDecimal::F64(value) => value.into_inner().is_sign_positive(),
            TypedDecimal::Decimal(value) => value.is_sign_positive(),
        }
    }

    /// Returns true if the value has a negative sign.
    pub fn is_sign_negative(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.into_inner().is_sign_negative(),
            TypedDecimal::F64(value) => value.into_inner().is_sign_negative(),
            TypedDecimal::Decimal(value) => value.is_sign_negative(),
        }
    }
    pub fn variant(&self) -> DecimalTypeVariant {
        match self {
            TypedDecimal::F32(_) => DecimalTypeVariant::F32,
            TypedDecimal::F64(_) => DecimalTypeVariant::F64,
            TypedDecimal::Decimal(_) => DecimalTypeVariant::Big,
        }
    }

    // TODO: Handle nan and infinity cases as nanf32 is ugly
    // Let's use nan_f32 or TBD
    pub fn to_string_with_suffix(&self) -> String {
        match self {
            TypedDecimal::F32(value) => format!("{}f32", value.into_inner()),
            TypedDecimal::F64(value) => format!("{}f64", value.into_inner()),
            TypedDecimal::Decimal(value) => format!("{}big", value),
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
                    TypedDecimal::F32(result.into_f32().into())
                }
            },
            TypedDecimal::F64(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F64(OrderedFloat(
                    a.into_inner() + b.into_inner() as f64,
                )),
                TypedDecimal::F64(b) => TypedDecimal::F64(a + b),
                TypedDecimal::Decimal(b) => {
                    let result = Decimal::from(a.into_inner()) + b;
                    TypedDecimal::F64(result.into_f64().into())
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
        // FIXME: Avoid cloning, as add should be applicable for refs only
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

impl Neg for TypedDecimal {
    type Output = TypedDecimal;

    fn neg(self) -> Self::Output {
        match self {
            TypedDecimal::F32(value) => TypedDecimal::F32(value.neg()),
            TypedDecimal::F64(value) => TypedDecimal::F64(value.neg()),
            TypedDecimal::Decimal(value) => TypedDecimal::Decimal(value.neg()),
        }
    }
}

impl Sub for &TypedDecimal {
    type Output = TypedDecimal;

    fn sub(self, rhs: Self) -> Self::Output {
        // FIXME: Avoid cloning, as sub should be applicable for refs only
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
    use crate::values::core_values::decimal::Decimal;
    use crate::{assert_structural_eq, assert_value_eq};
    use ordered_float::OrderedFloat;

    #[test]
    fn zero_sign() {
        let c = TypedDecimal::from(0.0f32);
        assert_matches!(c, TypedDecimal::F32(_));
        assert!(c.is_sign_positive());
        assert!(!c.is_sign_negative());

        let e = TypedDecimal::from(-0.0f32);
        assert_matches!(e, TypedDecimal::F32(_));
        assert!(!e.is_sign_positive());
        assert!(e.is_sign_negative());

        let f = TypedDecimal::from(0.0f64);
        assert_matches!(f, TypedDecimal::F64(_));
        assert!(f.is_sign_positive());
        assert!(!f.is_sign_negative());

        let g = TypedDecimal::from(-0.0f64);
        assert_matches!(g, TypedDecimal::F64(_));
        assert!(!g.is_sign_positive());
        assert!(g.is_sign_negative());

        let h = TypedDecimal::Decimal(Decimal::from(0.0));
        assert_matches!(h, TypedDecimal::Decimal(Decimal::Zero));
        assert!(h.is_sign_positive());
        assert!(!h.is_sign_negative());

        let i = TypedDecimal::Decimal(Decimal::from(-0.0));
        assert_matches!(i, TypedDecimal::Decimal(Decimal::NegZero));
        assert!(!i.is_sign_positive());
        assert!(i.is_sign_negative());
    }

    #[test]
    fn is_positive() {
        let a = TypedDecimal::from(42.0f32);
        assert_matches!(a, TypedDecimal::F32(_));
        assert!(a.is_sign_positive());

        let b = TypedDecimal::from(-42.0f64);
        assert_matches!(b, TypedDecimal::F64(_));
        assert!(!b.is_sign_positive());

        let d = TypedDecimal::from(0.01f64);
        assert_matches!(d, TypedDecimal::F64(_));
        assert!(d.is_sign_positive());

        let e = TypedDecimal::Decimal(0.0.into());
        assert_matches!(e, TypedDecimal::Decimal(Decimal::Zero));
        assert!(e.is_sign_positive());
    }

    #[test]
    fn is_negative() {
        let a = TypedDecimal::from(-42.0f32);
        assert_matches!(a, TypedDecimal::F32(_));
        assert!(a.is_sign_negative());

        let b = TypedDecimal::from(42.0f64);
        assert_matches!(b, TypedDecimal::F64(_));
        assert!(!b.is_sign_negative());

        let c = TypedDecimal::from(0.0f32);
        assert_matches!(c, TypedDecimal::F32(_));
        assert!(!c.is_sign_negative());

        let d = TypedDecimal::from(-0.01f64);
        assert_matches!(d, TypedDecimal::F64(_));
        assert!(d.is_sign_negative());

        let e = TypedDecimal::from(-0.0f32);
        assert_matches!(e, TypedDecimal::F32(_));
        assert!(e.is_sign_negative());

        let f = TypedDecimal::Decimal((-0.0).into());
        assert_matches!(f, TypedDecimal::Decimal(Decimal::NegZero));
        assert!(f.is_sign_negative());
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
    fn test_zero_equality() {
        let zero_f32 = TypedDecimal::from(0.0f32);
        let neg_zero_f32 = TypedDecimal::from(-0.0f32);
        assert_eq!(zero_f32, neg_zero_f32);
        assert_structural_eq!(zero_f32, neg_zero_f32);
        assert_value_eq!(zero_f32, neg_zero_f32);

        let zero_f64 = TypedDecimal::from(0.0f64);
        let neg_zero_f64 = TypedDecimal::from(-0.0f64);
        assert_eq!(zero_f64, neg_zero_f64);
        assert_structural_eq!(zero_f64, neg_zero_f64);
        assert_value_eq!(zero_f64, neg_zero_f64);

        let zero_big = TypedDecimal::Decimal(Decimal::from(0.0));
        let neg_zero_big = TypedDecimal::Decimal(Decimal::from(-0.0));
        assert_eq!(zero_big, neg_zero_big);
        assert_structural_eq!(zero_big, neg_zero_big);
        assert_value_eq!(zero_big, neg_zero_big);
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
    fn from_string() {
        let a = TypedDecimal::from_string("42.0").unwrap();
        assert_matches!(a, TypedDecimal::F32(OrderedFloat(42.0)));

        let b = TypedDecimal::from_string("42.0").unwrap();
        assert_matches!(b, TypedDecimal::F32(OrderedFloat(42.0)));

        let c = TypedDecimal::from_string("12345678901234567890.123456789")
            .unwrap();
        assert_matches!(c, TypedDecimal::F32(_));
        assert_eq!(c.as_f32(), 12345678901234567890.123456789);

        let d = TypedDecimal::from_string("not_a_number");
        assert!(d.is_err());

        let e = TypedDecimal::from_string("NaN").unwrap();
        assert!(e.is_nan());

        let f = TypedDecimal::from_string("nan").unwrap();
        assert!(f.is_nan());

        let g = TypedDecimal::from_string("Infinity").unwrap();
        assert!(g.is_infinite() && g.is_sign_positive());

        let h = TypedDecimal::from_string("-infinity").unwrap();
        assert!(h.is_infinite() && h.is_sign_negative());
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
        assert!(i.is_infinite() && i.is_sign_positive());

        let j = TypedDecimal::from_string_and_variant(
            "-infinity",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert!(j.is_infinite() && j.is_sign_negative());

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

    #[test]
    fn test_nan_equality() {
        let nan_f32_a = TypedDecimal::from(f32::NAN);
        let nan_f32_b = TypedDecimal::from(f32::NAN);
        let nan_f64_a = TypedDecimal::from(f64::NAN);
        let nan_f64_b = TypedDecimal::from(f64::NAN);
        let nan_big_a = TypedDecimal::Decimal(Decimal::NaN);
        let nan_big_b = TypedDecimal::Decimal(Decimal::NaN);

        // Structural equality (always false)
        assert!(!nan_f32_a.structural_eq(&nan_f32_b));
        assert!(!nan_f64_a.structural_eq(&nan_f64_b));
        assert!(!nan_big_a.structural_eq(&nan_big_b));
        assert!(!nan_f32_a.structural_eq(&nan_f64_a));
        assert!(!nan_f32_a.structural_eq(&nan_big_a));
        assert!(!nan_f64_a.structural_eq(&nan_big_a));

        // Value equality (always false for NaN)
        assert!(!nan_f32_a.value_eq(&nan_f32_b));
        assert!(!nan_f64_a.value_eq(&nan_f64_b));
        assert!(!nan_big_a.value_eq(&nan_big_b));
        assert!(!nan_f32_a.value_eq(&nan_f64_a));
        assert!(!nan_f32_a.value_eq(&nan_big_a));
        assert!(!nan_f64_a.value_eq(&nan_big_a));

        // Standard equality (always true for same decimal types)
        assert_eq!(nan_f32_a, nan_f32_b);
        assert_eq!(nan_f64_a, nan_f64_b);
        assert_eq!(nan_big_a, nan_big_b);
        assert_ne!(nan_f32_a, nan_f64_a);
        assert_ne!(nan_f32_a, nan_big_a);
        assert_ne!(nan_f64_a, nan_big_a);
    }
}
