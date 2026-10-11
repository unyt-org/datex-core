use crate::{
    traits::convert_value::ConvertValue,
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::Value,
    },
};

impl ConvertValue for CoreValue {
    fn to_value(self) -> Value {
        Value::Core(CoreValueWithClassification::new(self))
    }

    fn try_from_value(value: Value) -> Result<Self, Value>
    where
        Self: Sized,
    {
        value.try_into_core_value()
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Core(core_value) => Ok(&core_value.inner),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Core(core_value) => Ok(&mut core_value.inner),
            _ => Err(()),
        }
    }
}
