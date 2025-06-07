use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use ordered_float::OrderedFloat;

use crate::datex_values::{core_value_trait::CoreValueTrait, soft_eq::SoftEq};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Decimal {
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
}
impl CoreValueTrait for Decimal {}

impl SoftEq for Decimal {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Decimal::F32(a), Decimal::F32(b)) => a == b,
            (Decimal::F64(a), Decimal::F64(b)) => a == b,
            (Decimal::F32(a), Decimal::F64(b)) => {
                a.into_inner() as f64 == b.into_inner()
            }
            (Decimal::F64(a), Decimal::F32(b)) => {
                a.into_inner() == b.into_inner() as f64
            }
        }
    }
}

impl Decimal {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Decimal::F32(value) => Some(value.into_inner()),
            Decimal::F64(value) => Some(value.into_inner() as f32),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Decimal::F32(value) => Some(value.into_inner() as f64),
            Decimal::F64(value) => Some(value.into_inner()),
        }
    }

    pub fn is_f32(&self) -> bool {
        matches!(self, Decimal::F32(_))
    }
    pub fn is_f64(&self) -> bool {
        matches!(self, Decimal::F64(_))
    }
    pub fn is_positive(&self) -> bool {
        match self {
            Decimal::F32(value) => value.is_sign_positive(),
            Decimal::F64(value) => value.is_sign_positive(),
        }
    }
    pub fn is_negative(&self) -> bool {
        match self {
            Decimal::F32(value) => value.is_sign_negative(),
            Decimal::F64(value) => value.is_sign_negative(),
        }
    }
    pub fn is_nan(&self) -> bool {
        match self {
            Decimal::F32(value) => value.is_nan(),
            Decimal::F64(value) => value.is_nan(),
        }
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decimal::F32(value) => write!(f, "{:.1}", value.into_inner()),
            Decimal::F64(value) => write!(f, "{:.1}", value.into_inner()),
        }
    }
}

impl Add for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Decimal::F32(a) => match rhs {
                Decimal::F32(b) => Decimal::F32(a + b),
                Decimal::F64(b) => Decimal::F32(OrderedFloat(
                    a.into_inner() + b.into_inner() as f32,
                )),
            },
            Decimal::F64(a) => match rhs {
                Decimal::F32(b) => Decimal::F64(OrderedFloat(
                    a.into_inner() + b.into_inner() as f64,
                )),
                Decimal::F64(b) => {
                    Decimal::F64(OrderedFloat(a.into_inner() + b.into_inner()))
                }
            },
        }
    }
}

impl Add for &Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        Decimal::add(self.clone(), rhs.clone())
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = Decimal::add(self.clone(), rhs);
    }
}

impl From<f32> for Decimal {
    fn from(value: f32) -> Self {
        Decimal::F32(value.into())
    }
}
impl From<f64> for Decimal {
    fn from(value: f64) -> Self {
        Decimal::F64(value.into())
    }
}
