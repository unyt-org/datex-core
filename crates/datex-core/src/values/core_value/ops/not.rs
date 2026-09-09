use crate::values::core_value::CoreValue;
use core::ops::Not;

impl Not for &CoreValue {
    type Output = Option<CoreValue>;

    fn not(self) -> Self::Output {
        match self {
            CoreValue::Boolean(bool) => Some(CoreValue::Boolean(!bool)),
            _ => None, // Not applicable for other types
        }
    }
}

impl Not for CoreValue {
    type Output = Option<CoreValue>;

    fn not(self) -> Self::Output {
        (&self).not()
    }
}
