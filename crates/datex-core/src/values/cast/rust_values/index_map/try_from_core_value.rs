use crate::{
    random::RandomState,
    traits::convert_core_value::ConvertCoreValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
        core_values::native::DatexNativeBase,
        value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
    },
};
use core::hash::Hash;
use indexmap::IndexMap;

impl<K: DatexNativeBase + Eq + Hash + 'static, V: DatexNativeBase + 'static>
    ConvertCoreValue for IndexMap<K, V, RandomState>
{
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

impl<'a, K: DatexNativeBase + Eq + Hash + 'static, V: DatexNativeBase + 'static>
    TryFrom<BorrowedCoreValue<'a>> for Goat<'a, IndexMap<K, V, RandomState>>
{
    type Error = ();
    fn try_from(value: BorrowedCoreValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValue::Native(native) => native
                .filter_map(|v| {
                    v.as_any().downcast_ref::<IndexMap<K, V, RandomState>>()
                })
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a, K: DatexNativeBase + Eq + Hash + 'static, V: DatexNativeBase + 'static>
    TryFrom<BorrowedCoreValueMut<'a>>
    for GoatMut<'a, IndexMap<K, V, RandomState>>
{
    type Error = ();
    fn try_from(value: BorrowedCoreValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValueMut::Native(native) => native
                .filter_map(|v| {
                    v.as_any_mut().downcast_mut::<IndexMap<K, V, RandomState>>()
                })
                .ok_or(()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        random::RandomState,
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
        },
    };
    use indexmap::IndexMap;
    type TestMap = IndexMap<i32, i32, RandomState>;

    #[test]
    fn try_index_map_from_core_value() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        map.insert(2, 20);

        let core_value = CoreValue::native(map.clone());
        assert_eq!(core_value.try_as::<TestMap>().unwrap(), &map);
    }
    #[test]
    fn try_index_map_into_core_value() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        map.insert(2, 20);

        let core_value = CoreValue::native(map.clone());
        assert_eq!(core_value.try_into_value::<TestMap>().unwrap(), map);
    }

    #[test]
    fn try_index_map_mut_from_core_value() {
        let mut map = TestMap::default();
        map.insert(1, 10);

        let mut core_value = CoreValue::native(map);
        let map = core_value.try_as_mut::<TestMap>().unwrap();
        map.insert(2, 20);
        assert_eq!(core_value.try_as::<TestMap>().unwrap().get(&2), Some(&20));
    }

    #[test]
    fn try_index_map_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null;
        assert!(core_value.try_as::<TestMap>().is_none());
        assert!(core_value.try_into_value::<TestMap>().is_err());
    }

    #[test]
    fn try_borrowed_index_map() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        let core_value = CoreValue::native(map.clone());
        let borrowed = BorrowedCoreValue::from(&core_value);
        let result = Goat::<TestMap>::try_from(borrowed).unwrap();
        assert_eq!(*result, map);
    }

    #[test]
    fn try_borrowed_index_map_mut() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        let mut core_value = CoreValue::native(map);
        let borrowed = BorrowedCoreValueMut::from(&mut core_value);
        let mut result = GoatMut::<TestMap>::try_from(borrowed).unwrap();
        result.insert(2, 20);
        drop(result);
        assert_eq!(core_value.try_as::<TestMap>().unwrap().get(&2), Some(&20));
    }

    #[test]
    fn try_borrowed_index_map_wrong_type_fails() {
        let core_value =
            CoreValue::native(
                IndexMap::<String, String, RandomState>::default(),
            );
        let borrowed = BorrowedCoreValue::from(&core_value);
        assert!(Goat::<TestMap>::try_from(borrowed).is_err());
    }

    #[test]
    fn try_borrowed_index_map_mut_wrong_type_fails() {
        let mut core_value =
            CoreValue::native(
                IndexMap::<String, String, RandomState>::default(),
            );
        let borrowed = BorrowedCoreValueMut::from(&mut core_value);
        assert!(GoatMut::<TestMap>::try_from(borrowed).is_err());
    }
}
