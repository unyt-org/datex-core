use crate::{
    prelude::*,
    traits::convert_value::ConvertValue,
    values::{core_values::native::DatexNative, value::Value},
};

impl<T> ConvertValue for Box<T>
where
    Box<T>: DatexNative + 'static,
{
    fn to_value(self) -> Value {
        Value::native(self)
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Native(native) => {
                native.try_into_value::<Box<T>>().map_err(Value::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Native(native) => native.try_as::<Box<T>>().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Native(native) => native.try_as_mut::<Box<T>>().ok_or(()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_box_from_native_core_value() {
        let core_value = Box::new(42u32).to_value();
        let result = core_value.try_into_value::<Box<u32>>().unwrap();
        assert_eq!(*result, 42);
    }
}
