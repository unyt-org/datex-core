use crate::values::core_values::decimal::{
    Decimal, typed_decimal::TypedDecimal,
};
use core::ops::AddAssign;
impl AddAssign for TypedDecimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = self as &TypedDecimal + &rhs;
    }
}
impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = self as &Decimal + &rhs;
    }
}
impl AddAssign<&Decimal> for Decimal {
    fn add_assign(&mut self, rhs: &Decimal) {
        *self = self as &Decimal + rhs;
    }
}
impl AddAssign<&TypedDecimal> for TypedDecimal {
    fn add_assign(&mut self, rhs: &TypedDecimal) {
        *self = self as &TypedDecimal + rhs;
    }
}
