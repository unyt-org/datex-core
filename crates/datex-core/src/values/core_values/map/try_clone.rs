use crate::{
    traits::try_clone::TryClone,
    values::{core_value::CoreValue, core_values::map::Map, value::Value},
};

impl TryClone for Map {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Map(self.clone()).into())
    }
}
