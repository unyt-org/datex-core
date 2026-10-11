use crate::{
    prelude::*,
    traits::try_clone::TryClone,
    values::{core_value::CoreValue, value::Value},
};

impl TryClone for String {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Text(self.clone().into()).into())
    }
}
