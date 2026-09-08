use core::ops::AddAssign;

use crate::values::core_values::integer::typed_integer::TypedInteger;
impl AddAssign for TypedInteger {
    // FIXME #345 error handling / wrapping if out of bounds
    fn add_assign(&mut self, rhs: Self) {
        *self = (self as &TypedInteger + &rhs).expect("Failed to add");
    }
}
