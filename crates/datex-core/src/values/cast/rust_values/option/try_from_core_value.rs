use crate::{
    traits::convert_core_value::ConvertCoreValue,
    values::{core_value::CoreValue, core_values::native::DatexNativeBase},
};

impl<T> ConvertCoreValue for Option<T>
where
    T: DatexNativeBase + 'static,
{
    fn try_from_core_value(value: CoreValue) -> Result<Self, CoreValue> {
        match value {
            CoreValue::Null => Ok(None),
            CoreValue::Native(native) => {
                // native T -> Some(T)
                match native.try_into_value::<T>() {
                    Ok(value) => Ok(Some(value)),
                    Err(native) => {
                        // Native Option<T> -> Option<T>
                        native
                            .try_into_value::<Option<T>>()
                            .map_err(CoreValue::Native)
                    }
                }
            }
            value => Err(value),
        }
    }

    fn try_borrow_from_core_value(value: &CoreValue) -> Result<&Self, ()> {
        match value {
            CoreValue::Native(native) => native.try_as::<Option<T>>().ok_or(()),
            CoreValue::Null => Ok(&None),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_core_value(
        value: &mut CoreValue,
    ) -> Result<&mut Self, ()> {
        match value {
            CoreValue::Native(native) => {
                native.try_as_mut::<Option<T>>().ok_or(())
            }
            // We can not cover the CoreValue::Null here because we need to return a mutable reference to an Option<T>
            _ => Err(()),
        }
    }
}

impl<'a, T> TryFrom<&'a CoreValue> for Option<&'a T>
where
    T: DatexNativeBase + 'static,
{
    type Error = ();
    fn try_from(value: &'a CoreValue) -> Result<Self, Self::Error> {
        match value {
            CoreValue::Null => Ok(None),
            CoreValue::Native(native) => {
                native.try_as::<T>().map(Some).ok_or(())
            }

            _ => Err(()),
        }
    }
}

impl<'a, T> TryFrom<&'a mut CoreValue> for Option<&'a mut T>
where
    T: DatexNativeBase + 'static,
{
    type Error = ();
    fn try_from(value: &'a mut CoreValue) -> Result<Self, Self::Error> {
        match value {
            CoreValue::Null => Ok(None),
            CoreValue::Native(native) => {
                native.try_as_mut::<T>().map(Some).ok_or(())
            }

            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        traits::convert_core_value::ConvertCoreValue,
        values::{
            core_value::CoreValue,
            core_values::integer::typed_integer::TypedInteger,
        },
    };

    #[test]
    fn try_option_from_null() {
        let core_value = CoreValue::Null;
        let result = Option::<u32>::try_from_core_value(core_value);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn try_option_from_native() {
        let core_value = CoreValue::from(42u32);
        let result = Option::<u32>::try_from_core_value(core_value);
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn try_option_from_wrong_core_value() {
        let core_value = CoreValue::TypedInteger(TypedInteger::I32(42));
        let result = Option::<u32>::try_from_core_value(core_value);
        assert!(result.is_err());
    }

    #[test]
    fn try_option_ref_from_null() {
        let core_value = CoreValue::Null;
        let result = Option::<&u32>::try_from(&core_value);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn try_option_ref_from_native() {
        let core_value = CoreValue::from(42u32);
        let result = Option::<&u32>::try_from(&core_value);
        assert_eq!(*result.unwrap().unwrap(), 42);
    }

    #[test]
    fn try_option_ref_from_wrong_core_value() {
        let core_value = CoreValue::TypedInteger(TypedInteger::I32(42));
        let result = Option::<&u32>::try_from(&core_value);
        assert!(result.is_err());
    }

    #[test]
    fn try_option_mut_ref_from_null() {
        let mut core_value = CoreValue::Null;
        let result = Option::<&mut u32>::try_from(&mut core_value);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn try_option_mut_ref_from_native() {
        let mut core_value = CoreValue::from(42u32);
        let result = Option::<&mut u32>::try_from(&mut core_value);
        let value = result.unwrap().unwrap();
        *value = 100;
        assert_eq!(*core_value.try_as::<u32>().unwrap(), 100);
    }

    #[test]
    fn try_option_mut_ref_from_wrong_core_value() {
        let mut core_value = CoreValue::TypedInteger(TypedInteger::I32(42));
        let result = Option::<&mut u32>::try_from(&mut core_value);
        assert!(result.is_err());
    }
}
