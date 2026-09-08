use crate::{
    traits::convert_core_value::ConvertCoreValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
        value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
    },
};
use core::time::Duration;

impl ConvertCoreValue for Duration {
    fn try_from_core_value(value: CoreValue) -> Result<Self, CoreValue> {
        match value {
            CoreValue::Native(native) => {
                native.try_into_value().map_err(CoreValue::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_core_value(value: &CoreValue) -> Result<&Self, ()> {
        match value {
            CoreValue::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_core_value(
        value: &mut CoreValue,
    ) -> Result<&mut Self, ()> {
        match value {
            CoreValue::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedCoreValue<'a>> for Goat<'a, Duration> {
    type Error = ();
    fn try_from(value: BorrowedCoreValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValue::Native(native) => native
                .filter_map(|v| v.as_any().downcast_ref::<Duration>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedCoreValueMut<'a>> for GoatMut<'a, Duration> {
    type Error = ();
    fn try_from(value: BorrowedCoreValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValueMut::Native(native) => native
                .filter_map(|v| v.as_any_mut().downcast_mut::<Duration>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
        },
    };
    use core::time::Duration;

    #[test]
    fn try_duration_from_native_core_value() {
        let duration = Duration::from_secs(10);
        let core_value = CoreValue::from(duration);
        assert_eq!(core_value.try_into_value::<Duration>().unwrap(), duration);
    }

    #[test]
    fn try_borrow_duration_from_native_core_value() {
        let duration = Duration::from_secs(10);
        let core_value = CoreValue::from(duration);
        assert_eq!(*core_value.try_as::<Duration>().unwrap(), duration);
    }

    #[test]
    fn try_borrow_mut_duration_from_native_core_value() {
        let mut core_value = CoreValue::from(Duration::from_secs(10));
        *core_value.try_as_mut::<Duration>().unwrap() = Duration::from_secs(20);
        assert_eq!(
            *core_value.try_as::<Duration>().unwrap(),
            Duration::from_secs(20)
        );
    }

    #[test]
    fn try_duration_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null;
        assert!(core_value.try_as::<Duration>().is_none());
        assert!(core_value.try_into_value::<Duration>().is_err());
    }

    #[test]
    fn try_borrowed_core_value_duration() {
        let duration = Duration::from_secs(10);
        let core_value = CoreValue::from(duration);
        let borrowed = BorrowedCoreValue::from(&core_value);
        let result = Goat::<Duration>::try_from(borrowed).unwrap();
        assert_eq!(*result, duration);
    }

    #[test]
    fn try_borrowed_core_value_mut_duration() {
        let mut core_value = CoreValue::from(Duration::from_secs(10));
        let borrowed = BorrowedCoreValueMut::from(&mut core_value);
        let mut result = GoatMut::<Duration>::try_from(borrowed).unwrap();
        *result = Duration::from_secs(20);
        drop(result);
        assert_eq!(
            *core_value.try_as::<Duration>().unwrap(),
            Duration::from_secs(20)
        );
    }
}
