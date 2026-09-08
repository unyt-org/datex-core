use crate::values::core_values::decimal::{
    Decimal, typed_decimal::TypedDecimal,
};
use core::ops::Add;
use ordered_float::OrderedFloat;

impl Add for &Decimal {
    type Output = Decimal;

    fn add(self, rhs: &Decimal) -> Self::Output {
        match (self, rhs) {
            (Decimal::Finite(a), Decimal::Finite(b)) => Decimal::from(a + b),
            (Decimal::NegZero, Decimal::Zero)
            | (Decimal::Zero, Decimal::NegZero) => Decimal::Zero,
            (Decimal::Zero, b) | (b, Decimal::Zero) => b.clone(),
            (Decimal::NegZero, b) | (b, Decimal::NegZero) => b.clone(),
            (Decimal::Infinity, Decimal::NegInfinity)
            | (Decimal::NegInfinity, Decimal::Infinity) => Decimal::Nan,
            (Decimal::Infinity, _) | (_, Decimal::Infinity) => {
                Decimal::Infinity
            }
            (Decimal::NegInfinity, _) | (_, Decimal::NegInfinity) => {
                Decimal::NegInfinity
            }
            (Decimal::Nan, _) | (_, Decimal::Nan) => Decimal::Nan,
        }
    }
}

impl Add for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        (&self).add(&rhs)
    }
}

impl Add for &TypedDecimal {
    type Output = TypedDecimal;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            TypedDecimal::F32(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F32(a + b),
                TypedDecimal::F64(b) => TypedDecimal::F32(OrderedFloat(
                    a.into_inner() + b.into_inner() as f32,
                )),
                TypedDecimal::Decimal(b) => {
                    let result = &Decimal::from(a.into_inner()) + b;
                    TypedDecimal::F32(result.into_f32().into())
                }
            },
            TypedDecimal::F64(a) => match rhs {
                TypedDecimal::F32(b) => TypedDecimal::F64(OrderedFloat(
                    a.into_inner() + b.into_inner() as f64,
                )),
                TypedDecimal::F64(b) => TypedDecimal::F64(a + b),
                TypedDecimal::Decimal(b) => {
                    let result = &Decimal::from(a.into_inner()) + b;
                    TypedDecimal::F64(result.into_f64().into())
                }
            },
            TypedDecimal::Decimal(a) => {
                TypedDecimal::Decimal(a + &Decimal::from(rhs.clone()))
            }
        }
    }
}

impl Add for TypedDecimal {
    type Output = TypedDecimal;

    fn add(self, rhs: Self) -> Self::Output {
        (&self).add(&rhs)
    }
}
