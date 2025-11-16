use super::super::core_value_trait::CoreValueTrait;
use crate::collections::HashMap;
use crate::std_random::RandomState;
use crate::stdlib::format;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::traits::structural_eq::StructuralEq;
use crate::values::core_value::CoreValue;
use crate::values::value::Value;
use crate::values::value_container::{ValueContainer, ValueKey};
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};
use core::prelude::rust_2024::*;
use core::result::Result;
use indexmap::IndexMap;
use crate::references::reference::KeyNotFoundError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Map {
    // most general case, allows all types of keys and values, and dynamic size
    Dynamic(IndexMap<ValueContainer, ValueContainer, RandomState>),
    // for fixed-size maps with known keys and values on construction
    Fixed(Vec<(ValueContainer, ValueContainer)>),
    // for maps with string keys
    Structural(Vec<(String, ValueContainer)>), // for structural maps with string keys
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapAccessError {
    KeyNotFound(KeyNotFoundError),
    Immutable,
}

impl Display for MapAccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MapAccessError::KeyNotFound(err) => {
                core::write!(f, "{}", err)
            }
            MapAccessError::Immutable => {
                core::write!(f, "Map is immutable")
            }
        }
    }
}

impl Default for Map {
    fn default() -> Self {
        Map::Dynamic(IndexMap::default())
    }
}

impl Map {
    pub fn new(
        entries: IndexMap<ValueContainer, ValueContainer, RandomState>,
    ) -> Self {
        Map::Dynamic(entries)
    }

    pub fn is_structural(&self) -> bool {
        core::matches!(self, Map::Structural(_))
    }

    pub fn has_fixed_size(&self) -> bool {
        core::matches!(self, Map::Fixed(_) | Map::Structural(_))
    }

    pub fn size(&self) -> usize {
        match self {
            Map::Dynamic(map) => map.len(),
            Map::Fixed(vec) => vec.len(),
            Map::Structural(vec) => vec.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// Gets a value in the map by reference.
    /// Returns None if the key is not found.
    pub fn get<'a>(&self, key: impl Into<ValueKey<'a>>) -> Result<&ValueContainer, KeyNotFoundError> {
        let key = key.into();
        Ok(match self {
            Map::Dynamic(map) => {
                key.with_value_container(|key| {
                    map.get(key)
                })
            }
            Map::Fixed(vec) => {
                key.with_value_container(|key| {
                    vec.iter().find(|(k, _)| k == key).map(|(_, v)| v)
                })
            }
            Map::Structural(vec) => {
                // only works if key is a string
                if let Some(string) = key.try_as_text()
                {
                    vec.iter().find(|(k, _)| k == string).map(|(_, v)| v)
                } else {
                    None
                }
            }
        }.ok_or_else(|| KeyNotFoundError {key: key.into()})?)
    }

    /// Checks if the map contains the given key.
    pub fn has<'a>(&self, key: impl Into<ValueKey<'a>>) -> bool {
        match self {
            Map::Dynamic(map) => {
                key.into().with_value_container(|key| {
                    map.contains_key(key)
                })
            }
            Map::Fixed(vec) => {
                key.into().with_value_container(|key| {
                    vec.iter().any(|(k, _)| k == key)
                })
            },
            Map::Structural(vec) => {
                // only works if key is a string
                if let Some(string) = key.into().try_as_text()
                {
                    vec.iter().any(|(k, _)| k == string)
                } else {
                    false
                }
            }
        }
    }

    /// Removes a key from the map, returning the value if it existed.
    pub fn remove<'a>(
        &mut self,
        key: impl Into<ValueKey<'a>>,
    ) -> Result<ValueContainer, MapAccessError> {
        let key = key.into();
        match self {
            Map::Dynamic(map) => {
                key.with_value_container(|key| {
                    map.shift_remove(key).ok_or_else(|| MapAccessError::KeyNotFound(KeyNotFoundError { key: key.clone() }))
                })
            }
            Map::Fixed(_) | Map::Structural(_) => {
                Err(MapAccessError::Immutable)
            }
        }
    }

    /// Clears all entries in the map, returning an error if the map is not dynamic.
    pub fn clear(&mut self) -> Result<(), MapAccessError> {
        match self {
            Map::Dynamic(map) => {
                map.clear();
                Ok(())
            }
            Map::Fixed(_) | Map::Structural(_) => {
                Err(MapAccessError::Immutable)
            }
        }
    }

    /// Sets a value in the map, panicking if it fails.
    pub(crate) fn set<'a>(
        &mut self,
        key: impl Into<ValueKey<'a>>,
        value: impl Into<ValueContainer>,
    ) {
        self.try_set(key, value)
            .expect("Setting value in map failed");
    }

    /// Sets a value in the map, returning an error if it fails.
    /// This is the preferred way to set values in the map.
    pub(crate) fn try_set<'a>(
        &mut self,
        key: impl Into<ValueKey<'a>>,
        value: impl Into<ValueContainer>,
    ) -> Result<(), KeyNotFoundError> {
        let key = key.into();
        match self {
            Map::Dynamic(map) => {
                key.with_value_container(|key| {
                    map.insert(key.clone(), value.into());
                });
                Ok(())
            }
            Map::Fixed(vec) => {
                key.with_value_container(|key| {
                    if let Some((_, v)) = vec.iter_mut().find(|(k, _)| k == key) {
                        *v = value.into();
                        Ok(())
                    } else {
                        Err(KeyNotFoundError { key: key.clone() } )
                    }
                })
            }
            Map::Structural(vec) => {
                if let Some(string) = key.try_as_text() {
                    if let Some((_, v)) =
                        vec.iter_mut().find(|(k, _)| k == string) {
                        *v = value.into();
                        Ok(())
                    } else {
                        Err(KeyNotFoundError { key: key.into() } )
                    }
                } else {
                    Err(KeyNotFoundError { key: key.into() } )
                }
            }
        }
    }
}

pub enum MapKey<'a> {
    Text(&'a str),
    Value(&'a ValueContainer),
}

impl<'a> From<MapKey<'a>> for ValueContainer {
    fn from(key: MapKey) -> Self {
        match key {
            MapKey::Text(text) => ValueContainer::Value(Value::from(text)),
            MapKey::Value(value) => value.clone(),
        }
    }
}

impl Hash for MapKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            MapKey::Text(text) => text.hash(state),
            MapKey::Value(value) => value.hash(state),
        }
    }
}

impl StructuralEq for MapKey<'_> {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MapKey::Text(a), MapKey::Text(b)) => a == b,
            (MapKey::Value(a), MapKey::Value(b)) => a.structural_eq(b),
            (MapKey::Text(a), MapKey::Value(b))
            | (MapKey::Value(b), MapKey::Text(a)) => {
                if let ValueContainer::Value(Value {
                    inner: CoreValue::Text(text),
                    ..
                }) = b
                {
                    a == &text.0
                } else {
                    false
                }
            }
        }
    }
}

impl Display for MapKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // TODO #331: escape string
            MapKey::Text(string) => core::write!(f, "\"{}\"", string),
            MapKey::Value(value) => core::write!(f, "{value}"),
        }
    }
}

pub enum OwnedMapKey {
    Text(String),
    Value(ValueContainer),
}

impl From<OwnedMapKey> for ValueContainer {
    fn from(key: OwnedMapKey) -> Self {
        match key {
            OwnedMapKey::Text(text) => ValueContainer::Value(Value::from(text)),
            OwnedMapKey::Value(value) => value,
        }
    }
}

impl Display for OwnedMapKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OwnedMapKey::Text(text) => core::write!(f, "{text}"),
            OwnedMapKey::Value(value) => core::write!(f, "{value}"),
        }
    }
}

pub struct MapIterator<'a> {
    map: &'a Map,
    index: usize,
}

impl<'a> Iterator for MapIterator<'a> {
    type Item = (MapKey<'a>, &'a ValueContainer);

    fn next(&mut self) -> Option<Self::Item> {
        match self.map {
            Map::Dynamic(map) => {
                let item = map.iter().nth(self.index);
                self.index += 1;
                item.map(|(k, v)| {
                    let key = match k {
                        ValueContainer::Value(Value {
                            inner: CoreValue::Text(text),
                            ..
                        }) => MapKey::Text(&text.0),
                        _ => MapKey::Value(k),
                    };
                    (key, v)
                })
            }
            Map::Fixed(vec) => {
                if self.index < vec.len() {
                    let item = &vec[self.index];
                    self.index += 1;
                    let key = match &item.0 {
                        ValueContainer::Value(Value {
                            inner: CoreValue::Text(text),
                            ..
                        }) => MapKey::Text(&text.0),
                        _ => MapKey::Value(&item.0),
                    };
                    Some((key, &item.1))
                } else {
                    None
                }
            }
            Map::Structural(vec) => {
                if self.index < vec.len() {
                    let item = &vec[self.index];
                    self.index += 1;
                    Some((MapKey::Text(&item.0), &item.1))
                } else {
                    None
                }
            }
        }
    }
}

pub enum MapMutIterator<'a> {
    Dynamic(indexmap::map::IterMut<'a, ValueContainer, ValueContainer>),
    Fixed(core::slice::IterMut<'a, (ValueContainer, ValueContainer)>),
    Structural(core::slice::IterMut<'a, (String, ValueContainer)>),
}

impl<'a> Iterator for MapMutIterator<'a> {
    type Item = (MapKey<'a>, &'a mut ValueContainer);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MapMutIterator::Dynamic(iter) => iter.next().map(|(k, v)| {
                let key = match k {
                    ValueContainer::Value(Value {
                        inner: CoreValue::Text(text),
                        ..
                    }) => MapKey::Text(&text.0),
                    _ => MapKey::Value(k),
                };
                (key, v)
            }),
            MapMutIterator::Fixed(iter) => iter.next().map(|(k, v)| {
                let key = match k {
                    ValueContainer::Value(Value {
                        inner: CoreValue::Text(text),
                        ..
                    }) => MapKey::Text(&text.0),
                    _ => MapKey::Value(k),
                };
                (key, v)
            }),
            MapMutIterator::Structural(iter) => {
                iter.next().map(|(k, v)| (MapKey::Text(k.as_str()), v))
            }
        }
    }
}

pub struct IntoMapIterator {
    map: Map,
    index: usize,
}

impl Iterator for IntoMapIterator {
    type Item = (OwnedMapKey, ValueContainer);

    fn next(&mut self) -> Option<Self::Item> {
        // TODO #332: optimize to avoid cloning keys and values
        match &self.map {
            Map::Dynamic(map) => {
                let item = map.iter().nth(self.index);
                self.index += 1;
                item.map(|(k, v)| {
                    let key = match k {
                        ValueContainer::Value(Value {
                            inner: CoreValue::Text(text),
                            ..
                        }) => OwnedMapKey::Text(text.0.clone()),
                        _ => OwnedMapKey::Value(k.clone()),
                    };
                    (key, v.clone())
                })
            }
            Map::Fixed(vec) => {
                if self.index < vec.len() {
                    let item = &vec[self.index];
                    self.index += 1;
                    let key = match &item.0 {
                        ValueContainer::Value(Value {
                            inner: CoreValue::Text(text),
                            ..
                        }) => OwnedMapKey::Text(text.0.clone()),
                        _ => OwnedMapKey::Value(item.0.clone()),
                    };
                    Some((key, item.1.clone()))
                } else {
                    None
                }
            }
            Map::Structural(vec) => {
                if self.index < vec.len() {
                    let item = &vec[self.index];
                    self.index += 1;
                    Some((OwnedMapKey::Text(item.0.clone()), item.1.clone()))
                } else {
                    None
                }
            }
        }
    }
}

impl StructuralEq for Map {
    fn structural_eq(&self, other: &Self) -> bool {
        if self.size() != other.size() {
            return false;
        }
        for ((key, value), (other_key, other_value)) in
            self.into_iter().zip(other.into_iter())
        {
            if !key.structural_eq(&other_key)
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
        for (k, v) in self.into_iter() {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl CoreValueTrait for Map {}

impl Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{{")?;
        for (i, (key, value)) in self.into_iter().enumerate() {
            if i > 0 {
                core::write!(f, ", ")?;
            }
            core::write!(f, "{key}: {value}")?;
        }
        core::write!(f, "}}")
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
    type Item = (OwnedMapKey, ValueContainer);
    type IntoIter = IntoMapIterator;

    fn into_iter(self) -> Self::IntoIter {
        IntoMapIterator {
            map: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (MapKey<'a>, &'a ValueContainer);
    type IntoIter = MapIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        MapIterator {
            map: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a mut Map {
    type Item = (MapKey<'a>, &'a mut ValueContainer);
    type IntoIter = MapMutIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Map::Dynamic(map) => MapMutIterator::Dynamic(map.iter_mut()),
            Map::Fixed(vec) => MapMutIterator::Fixed(vec.iter_mut()),
            Map::Structural(vec) => MapMutIterator::Structural(vec.iter_mut()),
        }
    }
}

impl From<Vec<(ValueContainer, ValueContainer)>> for Map {
    /// Create a dynamic map from a vector of value containers.
    fn from(vec: Vec<(ValueContainer, ValueContainer)>) -> Self {
        Map::new(vec.into_iter().collect())
    }
}

impl From<Vec<(String, ValueContainer)>> for Map {
    /// Create a dynamic map from a vector of string keys and value containers.
    fn from(vec: Vec<(String, ValueContainer)>) -> Self {
        Map::new(
            vec.into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect::<IndexMap<ValueContainer, ValueContainer, RandomState>>(),
        )
    }
}

impl<K, V> FromIterator<(K, V)> for Map
where
    K: Into<ValueContainer>,
    V: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Map::Dynamic(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl From<IndexMap<ValueContainer, ValueContainer, RandomState>> for Map {
    fn from(
        map: IndexMap<ValueContainer, ValueContainer, RandomState>,
    ) -> Self {
        Map::new(map)
    }
}
impl From<IndexMap<String, ValueContainer, RandomState>> for Map {
    fn from(map: IndexMap<String, ValueContainer, RandomState>) -> Self {
        Map::new(
            map.into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect::<IndexMap<ValueContainer, ValueContainer, RandomState>>(),
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
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::map::Map;
    use crate::values::value_container::ValueContainer;
    use datex_core::values::core_values::decimal::Decimal;

    #[test]
    fn test_map() {
        let mut map = Map::default();
        map.set("key1", 42);
        map.set("key2", "value2");
        assert_eq!(map.size(), 2);
        assert_eq!(map.get("key1").unwrap().to_string(), "42");
        assert_eq!(map.get("key2").unwrap().to_string(), "\"value2\"");
        assert_eq!(map.to_string(), r#"{"key1": 42, "key2": "value2"}"#);
    }

    #[test]
    fn test_duplicate_keys() {
        let mut map = Map::default();
        map.set("key1", 42);
        map.set("key1", "new_value");
        assert_eq!(map.size(), 1);
        assert_eq!(map.get("key1").unwrap().to_string(), "\"new_value\"");
    }

    #[test]
    fn test_ref_keys() {
        let mut map = Map::default();
        let key = ValueContainer::new_reference(ValueContainer::from(42));
        map.set(&key, "value");
        // same reference should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&key));
        assert_eq!(map.get(&key).unwrap().to_string(), "\"value\"");

        // new reference with same value should not be found
        let new_key = ValueContainer::new_reference(ValueContainer::from(42));
        assert!(!map.has(&new_key));
        assert!(map.get(&new_key).is_err());
    }

    #[test]
    fn test_decimal_nan_value_key() {
        let mut map = Map::default();
        let nan_value = ValueContainer::from(Decimal::NaN);
        map.set(&nan_value, "value");
        // same NaN value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&nan_value));

        // new NaN value should also be found
        let new_nan_value = ValueContainer::from(Decimal::NaN);
        assert!(map.has(&new_nan_value));

        // adding new_nan_value should not increase size
        map.set(&new_nan_value, "new_value");
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn test_float_nan_value_key() {
        let mut map = Map::default();
        let nan_value = ValueContainer::from(f64::NAN);
        map.set(&nan_value, "value");
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
        map.set(&new_nan_value, "new_value");
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn test_decimal_zero_value_key() {
        let mut map = Map::default();
        let zero_value = ValueContainer::from(Decimal::Zero);
        map.set(&zero_value, "value");
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
        map.set(&neg_zero_value, "new_value");
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn test_float_zero_value_key() {
        let mut map = Map::default();
        let zero_value = ValueContainer::from(0.0f64);
        map.set(&zero_value, "value");
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
        map.set(&neg_zero_value, "new_value");
        assert_eq!(map.size(), 1);

        // new 0.0f32 value should not be found
        let float32_zero_value = ValueContainer::from(0.0f32);
        assert!(!map.has(&float32_zero_value));
    }

    #[test]
    fn test_typed_big_decimal_key() {
        let mut map = Map::default();
        let zero_big_decimal =
            ValueContainer::from(TypedDecimal::Decimal(Decimal::Zero));
        map.set(&zero_big_decimal, "value");
        // same Zero value should be found
        assert_eq!(map.size(), 1);
        assert!(map.has(&zero_big_decimal));
        // new Zero value should also be found
        let new_zero_big_decimal =
            ValueContainer::from(TypedDecimal::Decimal(Decimal::Zero));
        assert!(map.has(&new_zero_big_decimal));
        // new NegZero value should also be found
        let neg_zero_big_decimal =
            ValueContainer::from(TypedDecimal::Decimal(Decimal::NegZero));
        assert!(map.has(&neg_zero_big_decimal));

        // adding neg_zero_big_decimal should not increase size
        map.set(&neg_zero_big_decimal, "new_value");
        assert_eq!(map.size(), 1);
    }
}
