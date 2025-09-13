use super::super::core_value_trait::CoreValueTrait;
use crate::values::core_value::CoreValue;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;
use indexmap::IndexMap;
use indexmap::map::{IntoIter, Iter, IterMut};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};

// FIXME: restrict tuple keys to Integer and String only
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

    pub fn get_owned<T: Into<ValueContainer>>(
        &self,
        key: T,
    ) -> Option<&ValueContainer> {
        self.0.get(&key.into())
    }

    /// Set a key-value pair in the tuple. This method should only be used internal, since tuples
    /// are immutable after creation as per DATEX specification.
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
