use crate::datex_values::{
    core_value_trait::CoreValueTrait,
    core_values::decimal::{
        big_decimal::ExtendedBigDecimal, utils::decimal_to_string,
    },
    traits::soft_eq::SoftEq,
};
use num_traits::Signed;
use num_traits::ToPrimitive;
use num_traits::Zero;
use ordered_float::OrderedFloat;
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Neg, Sub},
};

#[derive(Debug, Clone, Eq, Hash)]
pub enum TypedDecimal {
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
    Big(ExtendedBigDecimal),
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
            (TypedDecimal::Big(a), TypedDecimal::Big(b)) => a == b,

            // F32 and F64
            (TypedDecimal::F32(a), TypedDecimal::F64(b))
            | (TypedDecimal::F64(b), TypedDecimal::F32(a)) => {
                a.into_inner() as f64 == b.into_inner()
            }

            // Big and F32
            (TypedDecimal::Big(a), TypedDecimal::F32(b))
            | (TypedDecimal::F32(b), TypedDecimal::Big(a)) => {
                a == &ExtendedBigDecimal::from(b.into_inner())
            }

            // Big and F64
            (TypedDecimal::Big(a), TypedDecimal::F64(b))
            | (TypedDecimal::F64(b), TypedDecimal::Big(a)) => {
                a == &ExtendedBigDecimal::from(b.into_inner())
            }
        }
    }
}

impl CoreValueTrait for TypedDecimal {}

impl SoftEq for TypedDecimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypedDecimal::F32(a), TypedDecimal::F32(b)) => a == b,
            (TypedDecimal::F64(a), TypedDecimal::F64(b)) => a == b,
            (TypedDecimal::F32(a), TypedDecimal::F64(b))
            | (TypedDecimal::F64(b), TypedDecimal::F32(a)) => {
                a.into_inner() as f64 == b.into_inner()
            }
            (TypedDecimal::Big(a), TypedDecimal::Big(b)) => a.soft_eq(b),
            (a, TypedDecimal::Big(b)) | (TypedDecimal::Big(b), a) => match a {
                TypedDecimal::F32(value) => {
                    b.try_into_f32().is_some_and(|v| v == value.into_inner())
                }
                TypedDecimal::F64(value) => {
                    b.try_into_f64().is_some_and(|v| v == value.into_inner())
                }
                _ => false,
            },
        }
    }
}

impl TypedDecimal {
    pub fn as_big(&self) -> ExtendedBigDecimal {
        match self {
            TypedDecimal::F32(value) => {
                ExtendedBigDecimal::from(value.into_inner())
            }
            TypedDecimal::F64(value) => {
                ExtendedBigDecimal::from(value.into_inner())
            }
            TypedDecimal::Big(value) => value.clone(),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self {
            TypedDecimal::F32(value) => value.into_inner(),
            TypedDecimal::F64(value) => value.into_inner() as f32,
            TypedDecimal::Big(value) => {
                value.try_into_f32().unwrap_or(f32::NAN)
            }
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            TypedDecimal::F32(value) => value.into_inner() as f64,
            TypedDecimal::F64(value) => value.into_inner(),
            TypedDecimal::Big(value) => {
                value.try_into_f64().unwrap_or(f64::NAN)
            }
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.into_inner().is_zero(),
            TypedDecimal::F64(value) => value.into_inner().is_zero(),
            TypedDecimal::Big(value) => value.is_zero(),
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
                    big_value.is_integer()
                        && big_value.to_f64().unwrap_or(f64::NAN).is_finite()
                }
                ExtendedBigDecimal::Zero => true,
                ExtendedBigDecimal::NegZero => true,
                ExtendedBigDecimal::Inf
                | ExtendedBigDecimal::NegInf
                | ExtendedBigDecimal::NaN => false,
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
                    ExtendedBigDecimal::NegZero => Some(0),
                    ExtendedBigDecimal::Inf
                    | ExtendedBigDecimal::NegInf
                    | ExtendedBigDecimal::NaN => None,
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
                ExtendedBigDecimal::Finite(big_value) => {
                    big_value.is_positive()
                }
                ExtendedBigDecimal::Zero => true,
                ExtendedBigDecimal::NegZero => false,
                ExtendedBigDecimal::Inf => true,
                ExtendedBigDecimal::NegInf | ExtendedBigDecimal::NaN => false,
            },
        }
    }
    pub fn is_negative(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_sign_negative(),
            TypedDecimal::F64(value) => value.is_sign_negative(),
            TypedDecimal::Big(value) => match value {
                ExtendedBigDecimal::Finite(big_value) => {
                    big_value.is_negative()
                }
                ExtendedBigDecimal::Zero => false,
                ExtendedBigDecimal::NegZero => true,
                ExtendedBigDecimal::Inf | ExtendedBigDecimal::NaN => false,
                ExtendedBigDecimal::NegInf => true,
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
                TypedDecimal::Big(b) => {
                    let result = b + TypedDecimal::F64(a);
                    if let Some(result_f64) = result.try_into_f64() {
                        TypedDecimal::F64(result_f64.into())
                    } else {
                        TypedDecimal::F64(f64::NAN.into())
                    }
                }
            },
            TypedDecimal::Big(a) => TypedDecimal::Big(a + rhs),
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
            TypedDecimal::F32(v) => {
                TypedDecimal::F32(OrderedFloat(v.into_inner().neg()))
            }
            TypedDecimal::F64(v) => {
                TypedDecimal::F64(OrderedFloat(v.into_inner().neg()))
            }
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
    use ordered_float::OrderedFloat;

    use crate::datex_values::core_values::decimal::{
        big_decimal::ExtendedBigDecimal, typed_decimal::TypedDecimal,
    };

    #[test]
    fn test_f32() {
        let c = TypedDecimal::from(1.5f32);
        matches!(c, TypedDecimal::F32(OrderedFloat(1.5)));
        assert_eq!(c.as_f32(), 1.5);
        assert_eq!(c.as_f64(), 1.5);
    }

    #[test]
    fn test_f64() {
        let c = TypedDecimal::from(1.5f64);
        matches!(c, TypedDecimal::F64(OrderedFloat(1.5)));
        assert_eq!(c.as_f32(), 1.5);
        assert_eq!(c.as_f64(), 1.5);
    }

    #[test]
    fn test_zero() {
        let a = TypedDecimal::from(0.0);
        matches!(a, TypedDecimal::Big(ExtendedBigDecimal::Zero));

        let a = TypedDecimal::from(-0.0);
        matches!(a, TypedDecimal::Big(ExtendedBigDecimal::NegZero));

        // f32
        let c = TypedDecimal::F32(0.0.into());
        matches!(c, TypedDecimal::F32(OrderedFloat(0.0)));
        assert_eq!(c.as_f32(), 0.0);
        assert_eq!(c.as_f32(), -0.0);
        assert_eq!(c.as_f64(), 0.0);
        assert_eq!(c.as_f64(), -0.0);

        // f64
        let c = TypedDecimal::F64(0.0.into());
        matches!(c, TypedDecimal::F64(OrderedFloat(0.0)));
        assert_eq!(c.as_f32(), 0.0);
        assert_eq!(c.as_f32(), -0.0);
        assert_eq!(c.as_f64(), 0.0);
        assert_eq!(c.as_f64(), -0.0);

        // big
        let c = TypedDecimal::Big(ExtendedBigDecimal::from(0.0));
        matches!(c, TypedDecimal::Big(ExtendedBigDecimal::Zero));

        assert_eq!(c.as_f32(), 0.0);
        assert_eq!(c.as_f32(), -0.0);
        assert_eq!(c.as_f64(), 0.0);
        assert_eq!(c.as_f64(), -0.0);
    }

    #[test]
    fn test_addition() {
        let a = TypedDecimal::F32(1.5.into());
        let b = TypedDecimal::F64(2.5.into());
        let result = a + b;
        assert_eq!(result.as_f32(), 4.0);
        assert_eq!(result.as_f64(), 4.0);
    }
}
