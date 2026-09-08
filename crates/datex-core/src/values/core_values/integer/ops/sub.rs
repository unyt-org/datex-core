use crate::values::core_values::integer::Integer;

use crate::values::core_values::integer::typed_integer::TypedInteger;
use core::ops::{Add, Neg, Sub};

impl Sub for &Integer {
    type Output = Integer;

    fn sub(self, rhs: Self) -> Self::Output {
        self + &(-rhs)
    }
}

impl Sub for Integer {
    type Output = Integer;

    fn sub(self, rhs: Self) -> Self::Output {
        &self - &rhs
    }
}

impl Sub for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn sub(self, rhs: Self) -> Self::Output {
        let neg_rhs = match rhs {
            TypedInteger::I8(v) => TypedInteger::I8(v.neg()),
            TypedInteger::I16(v) => TypedInteger::I16(v.neg()),
            TypedInteger::I32(v) => TypedInteger::I32(v.neg()),
            TypedInteger::I64(v) => TypedInteger::I64(v.neg()),
            TypedInteger::I128(v) => TypedInteger::I128(v.neg()),
            TypedInteger::U8(v) => TypedInteger::I16((*v as i16).neg()),
            TypedInteger::U16(v) => TypedInteger::I32((*v as i32).neg()),
            TypedInteger::U32(v) => TypedInteger::I64((*v as i64).neg()),
            TypedInteger::U64(v) => TypedInteger::I128((*v as i128).neg()),
            TypedInteger::U128(v) => {
                TypedInteger::I128((i128::try_from(*v).ok()?).neg())
            }
            TypedInteger::IBig(v) => TypedInteger::IBig(v.neg()),
        };
        self + &neg_rhs
    }
}

impl Sub for TypedInteger {
    type Output = Option<TypedInteger>;

    fn sub(self, rhs: Self) -> Self::Output {
        (&self).sub(&rhs)
    }
}
