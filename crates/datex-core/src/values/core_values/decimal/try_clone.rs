use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue, core_values::decimal::Decimal, value::Value,
    },
};

impl TryClone for Decimal {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::Decimal(self.clone()).into())
    }
}
