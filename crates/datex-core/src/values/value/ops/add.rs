use core::ops::Add;

use crate::values::{value::Value, value_container::error::ValueError};

impl Add for &Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: &Value) -> Self::Output {
        match (self, rhs) {
            (Value::Core(lhs), Value::Core(rhs)) => {
                (lhs + rhs).map(Value::Core)
            }
            (_, _) => todo!("Handle add for non-Core values"),
        }
    }
}

impl Add for Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: Value) -> Self::Output {
        &self + &rhs
    }
}
