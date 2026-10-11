use crate::{
    traits::try_clone::TryClone,
    values::{core_value::CoreValue, core_values::list::List, value::Value},
};

impl TryClone for List {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::List(self.clone()).into())
    }
}
