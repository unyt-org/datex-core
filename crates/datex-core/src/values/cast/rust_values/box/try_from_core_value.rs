use crate::{
    prelude::*,
    traits::convert_core_value::ConvertCoreValue,
    values::{core_value::CoreValue, core_values::native::DatexNativeBase},
};

impl<T> ConvertCoreValue for Box<T>
where
    T: DatexNativeBase + 'static,
{
    fn try_from_core_value(value: CoreValue) -> Result<Self, CoreValue> {
        match value {
            CoreValue::Native(native) => native
                .try_into_value::<T>()
                .map(Box::new)
                .map_err(CoreValue::Native),
            _ => Err(value),
        }
    }

    fn try_borrow_from_core_value(_value: &CoreValue) -> Result<&Self, ()> {
        Err(())
    }

    fn try_borrow_mut_from_core_value(
        _value: &mut CoreValue,
    ) -> Result<&mut Self, ()> {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_value::CoreValue;

    #[test]
    fn try_box_from_native_core_value() {
        let core_value = CoreValue::from(42u32);
        let result = core_value.try_into_value::<Box<u32>>().unwrap();
        assert_eq!(*result, 42);
    }
}
