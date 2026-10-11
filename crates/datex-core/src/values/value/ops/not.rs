use core::ops::Not;

use crate::values::value::Value;

impl Not for &Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        match self {
            Value::Core(core_value) => core_value.not().map(Value::Core),
            Value::Native(_native_value) => todo!(),
        }
    }
}
impl Not for Value {
    type Output = Option<Value>;

    fn not(self) -> Self::Output {
        (&self).not()
    }
}
