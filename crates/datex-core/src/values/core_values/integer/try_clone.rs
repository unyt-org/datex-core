use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue, core_values::integer::Integer, value::Value,
    },
};

impl TryClone for Integer {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Integer(self.clone()).into())
    }
}
