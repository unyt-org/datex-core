use core::ops::Sub;

use crate::values::{value::Value, value_container::error::ValueError};

impl Sub for &Value {
    type Output = Result<Value, ValueError>;
    fn sub(self, rhs: &Value) -> Self::Output {
        Ok((&self.inner - &rhs.inner)?.into())
    }
}

impl Sub for Value {
    type Output = Result<Value, ValueError>;
    fn sub(self, rhs: Value) -> Self::Output {
        &self - &rhs
    }
}
