use crate::{
    traits::try_clone::TryClone,
    values::{core_value::CoreValue, core_values::range::Range, value::Value},
};

impl TryClone for Range {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Range(self.clone()).into())
    }
}
