use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue, core_values::callable::Callable, value::Value,
    },
};

impl TryClone for Callable {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Callable(self.clone()).into())
    }
}
