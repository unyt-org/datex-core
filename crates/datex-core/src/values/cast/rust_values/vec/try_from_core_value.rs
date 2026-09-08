use crate::{
    prelude::*,
    traits::convert_core_value::ConvertCoreValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
        core_values::native::DatexNativeBase,
        value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
    },
};

impl<T: DatexNativeBase + 'static> ConvertCoreValue for Vec<T> {
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

impl<'a, T: DatexNativeBase + 'static> TryFrom<BorrowedCoreValue<'a>>
    for Goat<'a, Vec<T>>
{
    type Error = ();
    fn try_from(value: BorrowedCoreValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValue::Native(native) => native
                .filter_map(|v| v.as_any().downcast_ref::<Vec<T>>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a, T: DatexNativeBase + 'static> TryFrom<BorrowedCoreValueMut<'a>>
    for GoatMut<'a, Vec<T>>
{
    type Error = ();
    fn try_from(value: BorrowedCoreValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValueMut::Native(native) => native
                .filter_map(|v| v.as_any_mut().downcast_mut::<Vec<T>>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::*,
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
        },
    };

    #[test]
    fn try_vec_from_core_value() {
        let values = vec![1, 2, 3];
        let core_value = CoreValue::native(values.clone());

        assert_eq!(core_value.try_as::<Vec<i32>>().unwrap(), &values);
        assert_eq!(core_value.try_into_value::<Vec<i32>>().unwrap(), values);
    }

    #[test]
    fn try_vec_mut_from_core_value() {
        let mut core_value = CoreValue::native(vec![1, 2, 3]);

        let values = core_value.try_as_mut::<Vec<i32>>().unwrap();
        values.push(4);

        assert_eq!(core_value.try_as::<Vec<i32>>().unwrap(), &vec![1, 2, 3, 4]);
    }

    #[test]
    fn try_vec_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null;

        assert!(core_value.try_as::<Vec<i32>>().is_none());
        assert!(core_value.try_into_value::<Vec<i32>>().is_err());
    }

    #[test]
    fn try_borrowed_vec() {
        let values = vec![1, 2, 3];
        let core_value = CoreValue::native(values.clone());

        let borrowed = BorrowedCoreValue::from(&core_value);
        let result = Goat::<Vec<i32>>::try_from(borrowed).unwrap();
        assert_eq!(*result, values);
    }

    #[test]
    fn try_borrowed_vec_mut() {
        let mut core_value = CoreValue::native(vec![1, 2, 3]);

        let borrowed = BorrowedCoreValueMut::from(&mut core_value);
        let mut result = GoatMut::<Vec<i32>>::try_from(borrowed).unwrap();

        result.push(4);
        drop(result);
        assert_eq!(core_value.try_as::<Vec<i32>>().unwrap(), &vec![1, 2, 3, 4]);
    }

    #[test]
    fn try_borrowed_vec_wrong_type_fails() {
        // FIXME allow vec!["aaa", "bbb"] refs
        let core_value =
            CoreValue::native(vec!["hello".to_string(), "world".to_string()]);
        let borrowed = BorrowedCoreValue::from(&core_value);

        assert!(Goat::<Vec<i32>>::try_from(borrowed).is_err());
    }
}
