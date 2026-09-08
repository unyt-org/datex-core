use crate::values::core_values::decimal::{
    Decimal, typed_decimal::TypedDecimal,
};
use core::ops::Neg;

impl Neg for &Decimal {
    type Output = Decimal;

    fn neg(self) -> Self::Output {
        match self {
            Decimal::Finite(value) => Decimal::Finite(-value),
            Decimal::Zero => Decimal::NegZero,
            Decimal::NegZero => Decimal::Zero,
            Decimal::Infinity => Decimal::NegInfinity,
            Decimal::NegInfinity => Decimal::Infinity,
            Decimal::Nan => Decimal::Nan,
        }
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        (&self).neg()
    }
}

impl Neg for &TypedDecimal {
    type Output = TypedDecimal;

    fn neg(self) -> Self::Output {
        match self {
            TypedDecimal::F32(value) => TypedDecimal::F32(value.neg()),
            TypedDecimal::F64(value) => TypedDecimal::F64(value.neg()),
            TypedDecimal::Decimal(value) => TypedDecimal::Decimal(value.neg()),
        }
    }
}
impl Neg for TypedDecimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        (&self).neg()
    }
}
