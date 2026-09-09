use core::ops::Not;

use crate::values::core_values::boolean::Boolean;

impl Not for &Boolean {
    type Output = Boolean;

    fn not(self) -> Self::Output {
        Boolean(!self.0)
    }
}

impl Not for Boolean {
    type Output = Boolean;

    fn not(self) -> Self::Output {
        !&self
    }
}
