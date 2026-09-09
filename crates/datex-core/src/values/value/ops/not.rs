use core::ops::Not;

use crate::values::value::Value;

impl Not for &Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        let inner = &self.inner;
        let neg = !inner;
        neg.map(Value::from)
    }
}
impl Not for Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        (&self).not()
    }
}
