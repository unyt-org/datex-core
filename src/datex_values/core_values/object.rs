use super::super::core_value_trait::CoreValueTrait;
use crate::datex_values::traits::soft_eq::SoftEq;
use crate::datex_values::value_container::ValueContainer;
use indexmap::map::{IntoIter, Iter};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::zip;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Object(pub IndexMap<String, ValueContainer>);
impl Object {
    pub fn size(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, key: &str) -> &ValueContainer {
        self.try_get(key)
            .unwrap_or_else(|| panic!("Key '{key}' not found in Object"))
    }
    pub fn try_get(&self, key: &str) -> Option<&ValueContainer> {
        self.0.get(key)
    }
    pub fn get_or_insert_with<F>(
        &mut self,
        key: &str,
        default: F,
    ) -> &mut ValueContainer
    where
        F: FnOnce() -> ValueContainer,
    {
        self.0.entry(key.to_string()).or_insert_with(default)
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut ValueContainer> {
        self.0.get_mut(key)
    }
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
    pub fn values(&self) -> impl Iterator<Item = &ValueContainer> {
        self.0.values()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ValueContainer)> {
        self.0.iter()
    }
    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&String, &mut ValueContainer)> {
        self.0.iter_mut()
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn set<T: Into<ValueContainer>>(&mut self, key: &str, value: T) {
        self.0.insert(key.to_string(), value.into());
    }

    pub fn remove(&mut self, key: &str) -> Option<ValueContainer> {
        self.0.shift_remove(key)
    }
}

impl SoftEq for Object {
    fn soft_eq(&self, other: &Self) -> bool {
        if self.size() != other.size() {
            return false;
        }
        for (key, value) in zip(self.0.iter(), other.0.iter()) {
            if key.0 != value.0 || !key.1.soft_eq(value.1) {
                return false;
            }
        }
        true
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl CoreValueTrait for Object {}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (key, value)) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "\"{key}\": {value}")?;
        }
        write!(f, "}}")
    }
}

impl<T> From<HashMap<String, T>> for Object
where
    T: Into<ValueContainer>,
{
    fn from(map: HashMap<String, T>) -> Self {
        Object(map.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl<T> FromIterator<(String, T)> for Object
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        Object(iter.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl IntoIterator for Object {
    type Item = (String, ValueContainer);
    type IntoIter = IntoIter<String, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = (&'a String, &'a ValueContainer);
    type IntoIter = Iter<'a, String, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<IndexMap<ValueContainer, ValueContainer>> for Object {
    fn from(map: IndexMap<ValueContainer, ValueContainer>) -> Self {
        Object(map.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
}
