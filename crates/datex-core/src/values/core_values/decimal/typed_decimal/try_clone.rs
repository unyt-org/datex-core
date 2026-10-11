use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue,
        core_values::decimal::typed_decimal::TypedDecimal, value::Value,
    },
};

impl TryClone for TypedDecimal {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::TypedDecimal(self.clone()).into())
    }
}
