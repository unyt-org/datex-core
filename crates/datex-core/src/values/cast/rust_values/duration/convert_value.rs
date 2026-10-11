use crate::{
    traits::convert_value::ConvertValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    },
};
use core::time::Duration;

impl ConvertValue for Duration {
    fn to_value(self) -> Value {
        Value::native(self)
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Native(native) => {
                native.try_into_value().map_err(Value::Native)
            }
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Instant(instant),
                ..
            }) => Ok(instant.to_duration()),
            _ => Err(value),
        }
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, Duration> {
    type Error = BorrowedValue<'a>;
    fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValue::Native(native) => {
                native.try_as().map_err(BorrowedValue::Native)
            }
            _ => Err(value),
        }
    }
}

impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, Duration> {
    type Error = BorrowedValueMut<'a>;
    fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValueMut::Native(native) => {
                native.try_as_mut().map_err(BorrowedValueMut::Native)
            }
            _ => Err(value),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        traits::convert_value::ConvertValue,
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            value::borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    };
    use core::time::Duration;

    #[test]
    fn try_duration_from_native_core_value() {
        let duration = Duration::from_secs(10);
        let core_value = duration.to_value();
        assert_eq!(core_value.try_into_value::<Duration>().unwrap(), duration);
    }

    #[test]
    fn try_borrow_duration_from_native_core_value() {
        let duration = Duration::from_secs(10);
        let core_value = duration.to_value();
        assert_eq!(*core_value.try_as::<Duration>().unwrap(), duration);
    }

    #[test]
    fn try_borrow_mut_duration_from_native_core_value() {
        let mut core_value = Duration::from_secs(10).to_value();
        *core_value.try_as_mut::<Duration>().unwrap() = Duration::from_secs(20);
        assert_eq!(
            *core_value.try_as::<Duration>().unwrap(),
            Duration::from_secs(20)
        );
    }

    #[test]
    fn try_duration_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as::<Duration>().is_none());
        assert!(core_value.try_into_value::<Duration>().is_err());
    }

    #[test]
    fn try_borrowed_core_value_duration() {
        let duration = Duration::from_secs(10);
        let core_value = duration.to_value();
        let borrowed = BorrowedValue::from(&core_value);
        let result = Goat::<Duration>::try_from(borrowed).unwrap();
        assert_eq!(*result, duration);
    }

    #[test]
    fn try_borrowed_core_value_mut_duration() {
        let mut core_value = Duration::from_secs(10).to_value();
        let borrowed = BorrowedValueMut::from(&mut core_value);
        let mut result = GoatMut::<Duration>::try_from(borrowed).unwrap();
        *result = Duration::from_secs(20);
        drop(result);
        assert_eq!(
            *core_value.try_as::<Duration>().unwrap(),
            Duration::from_secs(20)
        );
    }
}
