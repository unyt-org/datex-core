use crate::{
    prelude::*,
    values::core_values::{
        decimal::{
            DECIMAL_INFINITY, DECIMAL_NAN, DECIMAL_NEG_INFINITY, Decimal,
        },
        error::NumberParseError,
    },
};
mod to_instructions;

use crate::{
    libs::core::type_id::CoreLibTypeId,
    traits::get_core_lib_type_id::GetCoreLibTypeId,
};
use binrw::{BinRead, BinWrite};
use core::{fmt::Display, hash::Hash, num::ParseFloatError, result::Result};
use num::ToPrimitive;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use num_traits::Zero;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use strum::Display;
use strum_macros::{AsRefStr, EnumIter, EnumString};

mod classification;
mod convert_parts;
mod datex_hash;
mod datex_native;
mod datex_native_structural;
pub mod equality;
mod get_core_lib_type_id;
mod get_datex_type;
pub mod primitive;
mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod try_clone;
pub mod update_handler;
mod value_access;

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
    Default,
)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum DecimalTypeVariant {
    #[default]
    F32,
    F64,
    DBig,
}

#[derive(Debug, Clone, Eq, BinRead, BinWrite)]
#[brw(little)]
pub enum TypedDecimal {
    #[brw(magic = 1u8)]
    F32(
        #[br(map = |x: f32| OrderedFloat(x))]
        #[bw(map = |x: &OrderedFloat<f32>| x.into_inner())]
        OrderedFloat<f32>,
    ),
    #[brw(magic = 2u8)]
    F64(
        #[br(map = |x: f64| OrderedFloat(x))]
        #[bw(map = |x: &OrderedFloat<f64>| x.into_inner())]
        OrderedFloat<f64>,
    ),
    #[brw(magic = 3u8)]
    Decimal(Decimal),
}

impl Serialize for TypedDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if !self.is_finite() {
            // Handle special edge cases, such as NaN and Infinity
            serializer.serialize_str(&self.to_string())
        } else {
            match self {
                TypedDecimal::F32(value) => {
                    serializer.serialize_f32(value.into_inner())
                }
                TypedDecimal::F64(value) => {
                    serializer.serialize_f64(value.into_inner())
                }
                TypedDecimal::Decimal(value) => value.serialize(serializer),
            }
        }
    }
}

impl Hash for TypedDecimal {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
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

impl From<&TypedDecimal> for CoreLibTypeId {
    fn from(value: &TypedDecimal) -> Self {
        value.core_lib_type_id()
    }
}

impl TypedDecimal {
    /// Parses a string into an [TypedDecimal::F32].
    /// A value can be parsed as f32 if it is a valid float literal and is within the range of f32.
    /// The special values "nan", "infinity" and "-infinity" are also supported, same as e-notation for f32 literals in Rust.
    pub fn parse_checked_f32(
        s: &str,
    ) -> Result<TypedDecimal, NumberParseError> {
        Ok(TypedDecimal::F32(
            match s {
                // handle special cases
                DECIMAL_INFINITY => f32::INFINITY,
                DECIMAL_NEG_INFINITY => f32::NEG_INFINITY,
                DECIMAL_NAN => f32::NAN,
                _ => {
                    let v: f64 = s.parse().map_err(|_: ParseFloatError| {
                        NumberParseError::InvalidFormat
                    })?;
                    let v: f32 =
                        v.to_f32().ok_or(NumberParseError::OutOfRange)?;
                    if !v.is_finite() {
                        return Err(NumberParseError::OutOfRange);
                    }
                    v
                }
            }
            .into(),
        ))
    }

    /// Parses a string into an [TypedDecimal::F64].
    /// A value can be parsed as f64 if it is a valid float literal and is within the range of f64.
    /// The special values "nan", "infinity" and "-infinity" are also supported, same as e-notation for f64 literals in Rust.
    pub fn parse_checked_f64(
        s: &str,
    ) -> Result<TypedDecimal, NumberParseError> {
        Ok(TypedDecimal::F64(
            match s {
                // handle special cases
                DECIMAL_INFINITY => f64::INFINITY,
                DECIMAL_NEG_INFINITY => f64::NEG_INFINITY,
                DECIMAL_NAN => f64::NAN,
                _ => {
                    let v: f64 = s.parse().map_err(|_: ParseFloatError| {
                        NumberParseError::InvalidFormat
                    })?;
                    if !v.is_finite() {
                        return Err(NumberParseError::OutOfRange);
                    }
                    v
                }
            }
            .into(),
        ))
    }

    /// Creates a TypedDecimal from a string and a variant, ensuring the value is within the valid range.
    /// Values that cannot be parsed or are out of range for the specified variant will result in an error.
    /// Special values like "infinity", "-infinity", and "nan" are also supported.
    pub fn try_from_string_and_variant(
        value: &str,
        variant: DecimalTypeVariant,
    ) -> Result<Self, NumberParseError> {
        match variant {
            DecimalTypeVariant::F32 => Self::parse_checked_f32(value),
            DecimalTypeVariant::F64 => Self::parse_checked_f64(value),
            DecimalTypeVariant::DBig => {
                Decimal::try_from_string(value).map(TypedDecimal::Decimal)
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

    /// Tries to borrow the inner f32 value if the TypedDecimal is of variant F32.
    pub fn borrow_as_f32(&self) -> Option<&f32> {
        match self {
            TypedDecimal::F32(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Tries to borrow the inner f32 value if the TypedDecimal is of variant F32.
    pub fn borrow_mut_as_f32(&mut self) -> Option<&mut f32> {
        match self {
            TypedDecimal::F32(value) => Some(value.as_mut()),
            _ => None,
        }
    }

    /// Tries to borrow the inner f64 value if the TypedDecimal is of variant F64.
    pub fn borrow_as_f64(&self) -> Option<&f64> {
        match self {
            TypedDecimal::F64(value) => Some(value.as_ref()),
            _ => None,
        }
    }

    /// Tries to borrow the inner f64 value if the TypedDecimal is of variant F64.
    pub fn borrow_mut_as_f64(&mut self) -> Option<&mut f64> {
        match self {
            TypedDecimal::F64(value) => Some(value.as_mut()),
            _ => None,
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
                    && core::f32::math::fract(value.into_inner()) == 0.0
            }
            TypedDecimal::F64(value) => {
                value.into_inner() >= i64::MIN as f64
                    && value.into_inner() <= i64::MAX as f64
                    && core::f64::math::fract(value.into_inner()) == 0.0
            }
            TypedDecimal::Decimal(value) => value.is_integer(),
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
                core::matches!(value, Decimal::Infinity | Decimal::NegInfinity)
            }
        }
    }

    /// Returns the value as an integer if it is an exact integer, otherwise returns None.
    pub fn as_integer(&self) -> Option<i64> {
        if self.is_integer() {
            match self {
                TypedDecimal::F32(value) => Some(value.into_inner() as i64),
                TypedDecimal::F64(value) => Some(value.into_inner() as i64),
                TypedDecimal::Decimal(value) => value.as_integer(),
            }
        } else {
            None
        }
    }

    /// Returns true if the TypedDecimal is of variant F32.
    pub fn is_f32(&self) -> bool {
        core::matches!(self, TypedDecimal::F32(_))
    }

    /// Returns true if the TypedDecimal is of variant F64.
    pub fn is_f64(&self) -> bool {
        core::matches!(self, TypedDecimal::F64(_))
    }

    /// Returns true if the value is NaN (Not a Number).
    pub fn is_nan(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_nan(),
            TypedDecimal::F64(value) => value.is_nan(),
            TypedDecimal::Decimal(value) => core::matches!(value, Decimal::Nan),
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
            TypedDecimal::Decimal(_) => DecimalTypeVariant::DBig,
        }
    }

    // TODO #338: Handle nan and infinity cases as nanf32 is ugly
    // Let's use nan_f32 or TBD
    pub fn to_string_with_suffix(&self) -> String {
        match self {
            TypedDecimal::F32(value) => format!("{}f32", value.into_inner()),
            TypedDecimal::F64(value) => format!("{}f64", value.into_inner()),
            TypedDecimal::Decimal(value) => format!("{}dbig", value),
        }
    }
}

impl Display for TypedDecimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_finite() {
            match self {
                TypedDecimal::F32(value) => {
                    core::write!(f, "{}", value.into_inner())
                }
                TypedDecimal::F64(value) => {
                    core::write!(f, "{}", value.into_inner())
                }
                TypedDecimal::Decimal(value) => core::write!(f, "{}", value),
            }
        } else if self.is_nan() {
            core::write!(f, "{}", DECIMAL_NAN)
        } else if self.is_sign_positive() {
            core::write!(f, "{}", DECIMAL_INFINITY)
        } else {
            core::write!(f, "{}", DECIMAL_NEG_INFINITY)
        }
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
    use crate::{
        prelude::*,
        traits::{
            structural_eq::{StructuralEq, assert_structural_eq},
            value_eq::{ValueEq, assert_value_eq},
        },
        values::core_values::{
            decimal::{
                Decimal,
                typed_decimal::{DecimalTypeVariant, TypedDecimal},
            },
            error::NumberParseError,
        },
    };
    use core::assert_matches;
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
    fn zero_equality() {
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
        let a = TypedDecimal::parse_checked_f32("42.0").unwrap();
        assert_matches!(a, TypedDecimal::F32(OrderedFloat(42.0)));

        let b = TypedDecimal::parse_checked_f32("42.0").unwrap();
        assert_matches!(b, TypedDecimal::F32(OrderedFloat(42.0)));

        let c =
            TypedDecimal::parse_checked_f32("12345678901234567890.123456789")
                .unwrap();
        assert_matches!(c, TypedDecimal::F32(_));
        assert_eq!(c.as_f32(), 12345678901234567890.123456789);

        let d = TypedDecimal::parse_checked_f32("not_a_number");
        assert!(d.is_err());

        let f = TypedDecimal::parse_checked_f32("nan").unwrap();
        assert!(f.is_nan());

        let g = TypedDecimal::parse_checked_f64("infinity").unwrap();
        assert!(g.is_infinite() && g.is_sign_positive());

        let h = TypedDecimal::parse_checked_f32("-infinity").unwrap();
        assert!(h.is_infinite() && h.is_sign_negative());
    }

    #[test]
    fn from_string_and_variant_out_of_range() {
        let a = TypedDecimal::parse_checked_f32(&f64::MAX.to_string());
        assert_eq!(a.err().unwrap(), NumberParseError::OutOfRange);

        let a = TypedDecimal::parse_checked_f32(&f64::MIN.to_string());
        assert_eq!(a.err().unwrap(), NumberParseError::OutOfRange);

        let a = TypedDecimal::parse_checked_f32("1e40"); // larger than f32::MAX
        assert_eq!(a.err().unwrap(), NumberParseError::OutOfRange);

        let a = TypedDecimal::parse_checked_f64("1e400"); // larger than f64::MAX
        assert_eq!(a.err().unwrap(), NumberParseError::OutOfRange);
    }

    #[test]
    fn from_string_and_variant() {
        let a = TypedDecimal::try_from_string_and_variant(
            "42.0",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert_matches!(a, TypedDecimal::F32(OrderedFloat(42.0)));

        let b = TypedDecimal::try_from_string_and_variant(
            "42.0",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert_matches!(b, TypedDecimal::F64(OrderedFloat(42.0)));

        let c = TypedDecimal::try_from_string_and_variant(
            "12345678901234567890.123456789",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert_matches!(c, TypedDecimal::F64(_));

        let d = TypedDecimal::try_from_string_and_variant(
            "12345678901234567890.123456789",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert_matches!(
            d,
            TypedDecimal::F32(OrderedFloat(12345678901234567890.123456789f32))
        );

        let e = TypedDecimal::try_from_string_and_variant(
            "not_a_number",
            DecimalTypeVariant::F32,
        );
        assert!(e.is_err());

        let f = TypedDecimal::try_from_string_and_variant(
            "not_a_number",
            DecimalTypeVariant::F64,
        );
        assert!(f.is_err());

        let g = TypedDecimal::try_from_string_and_variant(
            "NaN",
            DecimalTypeVariant::F32,
        );
        assert!(g.is_err());

        let h = TypedDecimal::try_from_string_and_variant(
            "nan",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert!(h.is_nan());

        let i = TypedDecimal::try_from_string_and_variant(
            "infinity",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        assert!(i.is_infinite() && i.is_sign_positive());

        let j = TypedDecimal::try_from_string_and_variant(
            "-infinity",
            DecimalTypeVariant::F64,
        )
        .unwrap();
        assert!(j.is_infinite() && j.is_sign_negative());

        let k = TypedDecimal::try_from_string_and_variant(
            "12345678901234567890.123456789",
            DecimalTypeVariant::DBig,
        )
        .unwrap();
        assert_matches!(k, TypedDecimal::Decimal(_));
        assert_eq!(k.as_f64(), 12345678901234567890.123456789);
    }

    #[test]
    fn try_from_string_and_variant() {
        let a = TypedDecimal::try_from_string_and_variant(
            "1e40",
            DecimalTypeVariant::F32,
        );
        assert!(a.is_err());
        assert_eq!(a.err().unwrap(), NumberParseError::OutOfRange);

        let b = TypedDecimal::try_from_string_and_variant(
            "-1e40",
            DecimalTypeVariant::F32,
        );
        assert!(b.is_err());
        assert_eq!(b.err().unwrap(), NumberParseError::OutOfRange);

        let c = TypedDecimal::try_from_string_and_variant(
            "1e1000",
            DecimalTypeVariant::F64,
        );
        assert!(c.is_err());
        assert_eq!(c.err().unwrap(), NumberParseError::OutOfRange);

        let d = TypedDecimal::try_from_string_and_variant(
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
        let nan_big_a = TypedDecimal::Decimal(Decimal::Nan);
        let nan_big_b = TypedDecimal::Decimal(Decimal::Nan);

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
