use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue,
        core_values::decimal::typed_decimal::TypedDecimal, value::Value,
    },
};

impl TryClone for f32 {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::TypedDecimal(TypedDecimal::F32((*self).into())).into())
    }
}

impl TryClone for f64 {
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(CoreValue::TypedDecimal(TypedDecimal::F64((*self).into())).into())
    }
}
