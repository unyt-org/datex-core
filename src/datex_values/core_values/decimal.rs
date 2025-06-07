use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};
use std::hash::Hash;
use ordered_float::OrderedFloat;

use crate::datex_values::{core_value_trait::CoreValueTrait, soft_eq::SoftEq};

// TODO: currently not required
pub fn smallest_fitting_float(value: f64) -> TypedDecimal {
    if value.is_nan() {
        TypedDecimal::F32(OrderedFloat(f32::NAN))
    }
    else if value >= f32::MIN as f64 && value <= f32::MAX as f64 {
        TypedDecimal::F32(OrderedFloat(value as f32))
    }
    // otherwise use f64
    else {
        TypedDecimal::F64(OrderedFloat(value))
    }
}

#[derive(Debug, Clone, Eq, Copy)]
pub struct Decimal(pub TypedDecimal);
impl SoftEq for Decimal {
    fn soft_eq(&self, other: &Self) -> bool {
        self.0.soft_eq(&other.0)
    }
}
impl<T: Into<TypedDecimal>> From<T> for Decimal {
    fn from(value: T) -> Self {
        Decimal(value.into())
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Decimal {
    type Output = Option<Decimal>;

    fn add(self, rhs: Self) -> Self::Output {
        Some(Decimal(
            match (self.0, rhs.0) {
                (TypedDecimal::F32(a), TypedDecimal::F32(b)) => {
                    let res = a + b;
                    // if out of f32 range, try adding as f64
                    if res.is_infinite() {
                        TypedDecimal::F64(OrderedFloat(a.into_inner() as f64 + b.into_inner() as f64))
                    } else {
                        TypedDecimal::F32(res)
                    }
                }
                (TypedDecimal::F64(a), TypedDecimal::F64(b)) => {
                    TypedDecimal::F64(OrderedFloat(a.into_inner() + b.into_inner()))
                }
                (TypedDecimal::F32(a), TypedDecimal::F64(b)) => {
                    TypedDecimal::F64(OrderedFloat(a.into_inner() as f64 + b.into_inner()))
                }
                (TypedDecimal::F64(a), TypedDecimal::F32(b)) => {
                    TypedDecimal::F64(OrderedFloat(a.into_inner() + b.into_inner() as f64))
                }
            }
        ))
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.soft_eq(other)
    }
}

impl Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TypedDecimal {
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
}
impl CoreValueTrait for TypedDecimal {}

impl SoftEq for TypedDecimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypedDecimal::F32(a), TypedDecimal::F32(b)) => a == b,
            (TypedDecimal::F64(a), TypedDecimal::F64(b)) => a == b,
            (TypedDecimal::F32(a), TypedDecimal::F64(b)) => {
                a.into_inner() as f64 == b.into_inner()
            }
            (TypedDecimal::F64(a), TypedDecimal::F32(b)) => {
                a.into_inner() == b.into_inner() as f64
            }
        }
    }
}

impl TypedDecimal {
    pub fn as_f32(&self) -> f32 {
        match self {
            TypedDecimal::F32(value) => value.into_inner(),
            TypedDecimal::F64(value) => value.into_inner() as f32,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            TypedDecimal::F32(value) => value.into_inner() as f64,
            TypedDecimal::F64(value) => value.into_inner(),
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
        }
    }
    pub fn is_negative(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_sign_negative(),
            TypedDecimal::F64(value) => value.is_sign_negative(),
        }
    }
    pub fn is_nan(&self) -> bool {
        match self {
            TypedDecimal::F32(value) => value.is_nan(),
            TypedDecimal::F64(value) => value.is_nan(),
        }
    }
}

impl Display for TypedDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypedDecimal::F32(value) => write!(f, "{:.1}", value.into_inner()),
            TypedDecimal::F64(value) => write!(f, "{:.1}", value.into_inner()),
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
            },
            TypedDecimal::F64(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F64(OrderedFloat(
                    a.into_inner() + b.into_inner() as f64,
                )),
                TypedDecimal::F64(b) => {
                    TypedDecimal::F64(OrderedFloat(a.into_inner() + b.into_inner()))
                }
            },
        }
    }
}

impl Add for &TypedDecimal {
    type Output = TypedDecimal;

    fn add(self, rhs: Self) -> Self::Output {
        TypedDecimal::add(*self, *rhs)
    }
}

impl AddAssign for TypedDecimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = TypedDecimal::add(*self, rhs);
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
    use super::*;

    #[test]
    fn test_smallest_fitting_float() {
        assert_eq!(smallest_fitting_float(1.0), TypedDecimal::F32(OrderedFloat(1.0)));
        assert_eq!(smallest_fitting_float(1.5), TypedDecimal::F32(OrderedFloat(1.5)));
        assert_eq!(smallest_fitting_float(1e200), TypedDecimal::F64(OrderedFloat(1e200)));
        assert_eq!(smallest_fitting_float(f64::NAN), TypedDecimal::F32(OrderedFloat(f32::NAN)));
    }

    #[test]
    fn test_decimal_addition() {
        let a = Decimal::from(TypedDecimal::F32(OrderedFloat(1.0)));
        let b = Decimal::from(TypedDecimal::F32(OrderedFloat(2.0)));
        let result = a + b;
        assert_eq!(result, Some(Decimal::from(TypedDecimal::F32(OrderedFloat(3.0)))));

        let c = Decimal::from(TypedDecimal::F64(OrderedFloat(1.5)));
        let d = Decimal::from(TypedDecimal::F64(OrderedFloat(2.5)));
        let result2 = c + d;
        assert_eq!(result2, Some(Decimal::from(TypedDecimal::F64(OrderedFloat(4.0)))));

        let e = Decimal::from(TypedDecimal::F32(OrderedFloat(0.1)));
        let f = Decimal::from(TypedDecimal::F32(OrderedFloat(0.2)));
        let result3 = e + f;
        assert_eq!(result3, Some(Decimal::from(TypedDecimal::F32(OrderedFloat(0.3)))));
    }
}