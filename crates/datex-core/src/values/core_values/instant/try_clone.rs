use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue, core_values::instant::Instant, value::Value,
    },
};

impl TryClone for Instant {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Instant(*self).into())
    }
}
