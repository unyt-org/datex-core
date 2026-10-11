use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue, core_values::endpoint::Endpoint, value::Value,
    },
};

impl TryClone for Endpoint {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Endpoint(self.clone()).into())
    }
}
