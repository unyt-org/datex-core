use crate::{
    random::RandomState,
    traits::convert_value::ConvertValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_values::native::DatexNativeBase,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    },
};
use core::hash::Hash;
use indexmap::IndexMap;

impl<K: DatexNativeBase + Eq + Hash + 'static, V: DatexNativeBase + 'static>
    ConvertValue for IndexMap<K, V, RandomState>
{
    fn to_value(self) -> Value {
        Value::native(self)
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Native(native) => {
                native.try_into_value().map_err(Value::Native)
            }
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

impl<'a, K: DatexNativeBase + Eq + Hash + 'static, V: DatexNativeBase + 'static>
    TryFrom<BorrowedValue<'a>> for Goat<'a, IndexMap<K, V, RandomState>>
{
    type Error = ();
    fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValue::Native(native) => native
                .filter_map(|v| {
                    v.as_any().downcast_ref::<IndexMap<K, V, RandomState>>()
                })
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a, K: DatexNativeBase + Eq + Hash + 'static, V: DatexNativeBase + 'static>
    TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, IndexMap<K, V, RandomState>>
{
    type Error = ();
    fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValueMut::Native(native) => native
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
        prelude::*,
        random::RandomState,
        traits::convert_value::ConvertValue,
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            value::{
                Value,
                borrowed_value::{BorrowedValue, BorrowedValueMut},
            },
        },
    };
    use indexmap::IndexMap;

    type TestMap = IndexMap<i32, i32, RandomState>;

    #[test]
    fn try_index_map_from_core_value() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        map.insert(2, 20);

        let core_value = Value::native(map.clone());
        assert_eq!(core_value.try_as::<TestMap>().unwrap(), &map);
    }
    #[test]
    fn try_index_map_into_core_value() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        map.insert(2, 20);

        let core_value = Value::native(map.clone());
        assert_eq!(core_value.try_into_value::<TestMap>().unwrap(), map);
    }

    #[test]
    fn try_index_map_mut_from_core_value() {
        let mut map = TestMap::default();
        map.insert(1, 10);

        let mut core_value = Value::native(map);
        let map = core_value.try_as_mut::<TestMap>().unwrap();
        map.insert(2, 20);
        assert_eq!(core_value.try_as::<TestMap>().unwrap().get(&2), Some(&20));
    }

    #[test]
    fn try_index_map_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as::<TestMap>().is_none());
        assert!(core_value.try_into_value::<TestMap>().is_err());
    }

    #[test]
    fn try_borrowed_index_map() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        let core_value = Value::native(map.clone());
        let borrowed = BorrowedValue::from(&core_value);
        let result = Goat::<TestMap>::try_from(borrowed).unwrap();
        assert_eq!(*result, map);
    }

    #[test]
    fn try_borrowed_index_map_mut() {
        let mut map = TestMap::default();
        map.insert(1, 10);
        let mut core_value = Value::native(map);
        let borrowed = BorrowedValueMut::from(&mut core_value);
        let mut result = GoatMut::<TestMap>::try_from(borrowed).unwrap();
        result.insert(2, 20);
        drop(result);
        assert_eq!(core_value.try_as::<TestMap>().unwrap().get(&2), Some(&20));
    }

    #[test]
    fn try_borrowed_index_map_wrong_type_fails() {
        let core_value =
            Value::native(IndexMap::<String, String, RandomState>::default());
        let borrowed = BorrowedValue::from(&core_value);
        assert!(Goat::<TestMap>::try_from(borrowed).is_err());
    }

    #[test]
    fn try_borrowed_index_map_mut_wrong_type_fails() {
        let mut core_value =
            Value::native(IndexMap::<String, String, RandomState>::default());
        let borrowed = BorrowedValueMut::from(&mut core_value);
        assert!(GoatMut::<TestMap>::try_from(borrowed).is_err());
    }
}
