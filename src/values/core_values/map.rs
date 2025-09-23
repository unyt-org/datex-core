use super::super::core_value_trait::CoreValueTrait;
use crate::values::core_value::CoreValue;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;
use indexmap::IndexMap;
use indexmap::map::{IntoIter, Iter, IterMut};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Map(IndexMap<ValueContainer, ValueContainer>);

impl Map {
    pub fn new(entries: IndexMap<ValueContainer, ValueContainer>) -> Self {
        Map(entries)
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, key: &ValueContainer) -> Option<&ValueContainer> {
        self.0.get(key)
    }

    pub fn has(&self, key: &ValueContainer) -> bool {
        self.0.contains_key(key)
    }

    pub fn get_owned<T: Into<ValueContainer>>(
        &self,
        key: T,
    ) -> Option<&ValueContainer> {
        self.0.get(&key.into())
    }

    pub(crate) fn set<K: Into<ValueContainer>, V: Into<ValueContainer>>(
        &mut self,
        key: K,
        value: V,
    ) {
        self.0.insert(key.into(), value.into());
    }

    pub fn iter(&'_ self) -> Iter<'_, ValueContainer, ValueContainer> {
        self.0.iter()
    }

    pub fn iter_mut(
        &'_ mut self,
    ) -> IterMut<'_, ValueContainer, ValueContainer> {
        self.0.iter_mut()
    }
}

impl StructuralEq for Map {
    fn structural_eq(&self, other: &Self) -> bool {
        if self.size() != other.size() {
            return false;
        }
        for ((key, value), (other_key, other_value)) in
            self.0.iter().zip(other.0.iter())
        {
            if !key.structural_eq(other_key)
                || !value.structural_eq(other_value)
            {
                return false;
            }
        }
        true
    }
}

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl CoreValueTrait for Map {}

impl Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        for (i, (key, value)) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{key}: {value}")?;
        }
        write!(f, ")")
    }
}

impl<K, V> From<HashMap<K, V>> for Map
where
    K: Into<ValueContainer>,
    V: Into<ValueContainer>,
{
    fn from(map: HashMap<K, V>) -> Self {
        Map::new(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl IntoIterator for Map {
    type Item = (ValueContainer, ValueContainer);
    type IntoIter = IntoIter<ValueContainer, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (&'a ValueContainer, &'a ValueContainer);
    type IntoIter = Iter<'a, ValueContainer, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<(ValueContainer, ValueContainer)>> for Map {
    fn from(vec: Vec<(ValueContainer, ValueContainer)>) -> Self {
        Map::new(vec.into_iter().collect())
    }
}

impl From<Vec<(String, ValueContainer)>> for Map {
    fn from(vec: Vec<(String, ValueContainer)>) -> Self {
        Map::new(vec.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<K, V> FromIterator<(K, V)> for Map
where
    K: Into<ValueContainer>,
    V: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Map(iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect())
    }
}

impl From<IndexMap<ValueContainer, ValueContainer>> for Map {
    fn from(map: IndexMap<ValueContainer, ValueContainer>) -> Self {
        Map::new(map)
    }
}
impl From<IndexMap<String, ValueContainer>> for Map {
    fn from(map: IndexMap<String, ValueContainer>) -> Self {
        Map::new(
            map.into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect::<IndexMap<ValueContainer, ValueContainer>>(),
        )
    }
}
impl TryFrom<CoreValue> for Map {
    type Error = String;

    fn try_from(value: CoreValue) -> Result<Self, Self::Error> {
        if let CoreValue::Map(map) = value {
            Ok(map)
        } else {
            Err(format!("Expected CoreValue::Map, found {value:?}"))
        }
    }
}


#[cfg(test)]
mod tests {
    use datex_core::values::core_values::decimal::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::map::Map;
    use crate::values::value_container::ValueContainer;

    #[test]
    fn test_map() {
        let mut map = Map::default();
        map.set("key1", 42);
        map.set("key2", "value2");
        assert_eq!(map.size(), 2);
        assert_eq!(map.get_owned("key1").unwrap().to_string(), "42");
        assert_eq!(map.get_owned("key2").unwrap().to_string(), "\"value2\"");
        assert_eq!(map.to_string(), r#"("key1": 42, "key2": "value2")"#);
    }

    #[test]
    fn test_duplicate_keys() {
        let mut map = Map::default();
        map.set("key1", 42);
        map.set("key1", "new_value");
        assert_eq!(map.size(), 1);
        assert_eq!(map.get_owned("key1").unwrap().to_string(), "\"new_value\"");
    }

    #[test]
    fn test_ref_keys() {
        let mut map = Map::default();
        let key = ValueContainer::new_reference(ValueContainer::from(42));
        map.set(key.clone(), "value");
        // same reference should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&key));
        assert_eq!(map.get(&key).unwrap().to_string(), "\"value\"");

        // new reference with same value should not be found
        let new_key = ValueContainer::new_reference(ValueContainer::from(42));
        assert!(!map.has(&new_key));
        assert!(map.get(&new_key).is_none());
    }

    #[test]
    fn test_decimal_nan_value_key() {
        let mut map = Map::default();
        let nan_value = ValueContainer::from(Decimal::NaN);
        map.set(nan_value.clone(), "value");
        // same NaN value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&nan_value));

        // new NaN value should also be found
        let new_nan_value = ValueContainer::from(Decimal::NaN);
        assert!(map.has(&new_nan_value));
        
        // adding new_nan_value should not increase size
        map.set(new_nan_value.clone(), "new_value");
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn test_float_nan_value_key() {
        let mut map = Map::default();
        let nan_value = ValueContainer::from(f64::NAN);
        map.set(nan_value.clone(), "value");
        // same NaN value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&nan_value));

        // new f64 NaN value should also be found
        let new_nan_value = ValueContainer::from(f64::NAN);
        assert!(map.has(&new_nan_value));

        // new f32 NaN should not be found
        let float32_nan_value = ValueContainer::from(f32::NAN);
        assert!(!map.has(&float32_nan_value));
        
        // adding new_nan_value should not increase size
        map.set(new_nan_value.clone(), "new_value");
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn test_decimal_zero_value_key() {
        let mut map = Map::default();
        let zero_value = ValueContainer::from(Decimal::Zero);
        map.set(zero_value.clone(), "value");
        // same Zero value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&zero_value));

        // new Zero value should also be found
        let new_zero_value = ValueContainer::from(Decimal::Zero);
        println!("new_zero_value: {:?}", new_zero_value);
        assert!(map.has(&new_zero_value));

        // new NegZero value should also be found
        let neg_zero_value = ValueContainer::from(Decimal::NegZero);
        assert!(map.has(&neg_zero_value));
        
        // adding neg_zero_value should not increase size
        map.set(neg_zero_value.clone(), "new_value");
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn test_float_zero_value_key() {
        let mut map = Map::default();
        let zero_value = ValueContainer::from(0.0f64);
        map.set(zero_value.clone(), "value");
        // same 0.0 value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&zero_value));
        // new 0.0 value should also be found
        let new_zero_value = ValueContainer::from(0.0f64);
        assert!(map.has(&new_zero_value));
        // new -0.0 value should also be found
        let neg_zero_value = ValueContainer::from(-0.0f64);
        assert!(map.has(&neg_zero_value));

        // adding neg_zero_value should not increase size
        map.set(neg_zero_value.clone(), "new_value");
        assert_eq!(map.size(), 1);

        // new 0.0f32 value should not be found
        let float32_zero_value = ValueContainer::from(0.0f32);
        assert!(!map.has(&float32_zero_value));
    }

    #[test]
    fn test_typed_big_decimal_key() {
        let mut map = Map::default();
        let zero_big_decimal = ValueContainer::from(TypedDecimal::Decimal(Decimal::Zero));
        map.set(zero_big_decimal.clone(), "value");
        // same Zero value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&zero_big_decimal));
        // new Zero value should also be found
        let new_zero_big_decimal = ValueContainer::from(TypedDecimal::Decimal(Decimal::Zero));
        assert!(map.has(&new_zero_big_decimal));
        // new NegZero value should also be found
        let neg_zero_big_decimal = ValueContainer::from(TypedDecimal::Decimal(Decimal::NegZero));
        assert!(map.has(&neg_zero_big_decimal));

        // adding neg_zero_big_decimal should not increase size
        map.set(neg_zero_big_decimal.clone(), "new_value");
        assert_eq!(map.size(), 1);
    }
}


