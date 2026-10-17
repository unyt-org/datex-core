use crate::{
    traits::convert_value::ConvertValue,
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        core_values::native::{DatexNative, DatexNativeBase},
        value::Value,
    },
};
use crate::values::core_values::native::NativeCoreValue;

impl<T> ConvertValue for Option<T>
where
    T: DatexNative + ConvertValue + 'static,
{
    fn to_value(self) -> Value {
        match self {
            Some(value) => Value::native(value),
            None => CoreValue::Null.into(),
        }
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            // null value, convert to None
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Null,
                ..
            }) => Ok(None),
            // native "None" value
            Value::Native(NativeCoreValue {value})
            if value.as_any().is::<Option<T>>() && value.as_any().downcast_ref::<Option<T>>().unwrap().is_none() => {
                Ok(None)
            }
            // other T values, convert to Some(T)
            value => value.try_into_value::<T>().map(Option::Some),
        }
    }

    fn try_borrow_from_value(_value: &Value) -> Result<&Self, ()> {
        // Option<T> is represented as null | T, so CoreValue never contains
        // an Option<T> that can be borrowed as &Option<T>.
        // We could return `Ok(&None)` for the null case here, but to make the API consistent
        // with the mutable version and avoid returning a reference to a temporary Option<T>,
        // we simply return an error.
        Err(())
    }

    fn try_borrow_mut_from_value(_value: &mut Value) -> Result<&mut Self, ()> {
        // Option<T> is represented as null | T, so CoreValue never contains
        // an Option<T> that can be borrowed as &mut Option<T>.
        Err(())
    }
}

impl<'a, T> TryFrom<&'a Value> for Option<&'a T>
where
    T: DatexNativeBase + ConvertValue + 'static,
{
    type Error = ();
    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Null,
                ..
            }) => Ok(None),
            _ => value.try_as::<T>().map(Some).ok_or(()),
        }
    }
}

impl<'a, T> TryFrom<&'a mut Value> for Option<&'a mut T>
where
    T: DatexNativeBase + ConvertValue + 'static,
{
    type Error = ();
    fn try_from(value: &'a mut Value) -> Result<Self, Self::Error> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Null,
                ..
            }) => Ok(None),
            _ => value.try_as_mut::<T>().map(Some).ok_or(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        traits::convert_value::ConvertValue,
        values::{
            core_value::CoreValue,
            core_values::integer::typed_integer::TypedInteger,
        },
    };

    #[test]
    fn try_option_from_null() {
        let core_value = CoreValue::Null;
        let result = Option::<u32>::try_from_value(core_value.into());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn try_option_from_native() {
        let core_value = 42u32.to_value();
        let result = Option::<u32>::try_from_value(core_value.into());
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn try_option_from_wrong_core_value() {
        let core_value = CoreValue::TypedInteger(TypedInteger::I32(42));
        let result = Option::<u32>::try_from_value(core_value.into());
        assert!(result.is_err());
    }

    #[test]
    fn try_option_ref_from_null() {
        let core_value = CoreValue::Null.to_value();
        let result = Option::<&u32>::try_from(&core_value);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn try_option_ref_from_native() {
        let core_value = 42u32.to_value();
        let result = Option::<&u32>::try_from(&core_value);
        assert_eq!(*result.unwrap().unwrap(), 42);
    }

    #[test]
    fn try_option_ref_from_wrong_core_value() {
        let core_value =
            CoreValue::TypedInteger(TypedInteger::I32(42)).to_value();
        let result = Option::<&u32>::try_from(&core_value);
        assert!(result.is_err());
    }

    #[test]
    fn try_option_mut_ref_from_null() {
        let mut core_value = CoreValue::Null.to_value();
        let result = Option::<&mut u32>::try_from(&mut core_value);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn try_option_mut_ref_from_native() {
        let mut core_value = 42u32.to_value();
        let result = Option::<&mut u32>::try_from(&mut core_value);
        let value = result.unwrap().unwrap();
        *value = 100;
        assert_eq!(*core_value.try_as::<u32>().unwrap(), 100);
    }

    #[test]
    fn try_option_mut_ref_from_wrong_core_value() {
        let mut core_value =
            CoreValue::TypedInteger(TypedInteger::I32(42)).to_value();
        let result = Option::<&mut u32>::try_from(&mut core_value);
        assert!(result.is_err());
    }
}
