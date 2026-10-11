use core::ops::Neg;

use crate::values::{value::Value, value_container::error::ValueError};

impl Neg for Value {
    type Output = Result<Value, ValueError>;

    fn neg(self) -> Self::Output {
        match self {
            Value::Core(core_value) => (-core_value).map(Value::Core),
            Value::Native(_native_value) => {
                todo!()
            }
        }
    }
}
