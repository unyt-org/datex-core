use core::ops::Add;

use crate::values::{value::Value, value_container::error::ValueError};

impl Add for &Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: &Value) -> Self::Output {
        Ok((&self.inner + &rhs.inner)?.into())
    }
}

impl Add for Value {
    type Output = Result<Value, ValueError>;
    fn add(self, rhs: Value) -> Self::Output {
        &self + &rhs
    }
}
