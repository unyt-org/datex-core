use std::{
    fmt,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use indexmap::IndexMap;
use indexmap::map::{IntoIter, Iter};
use crate::datex_values::value_container::ValueContainer;
use super::super::{
    core_value_trait::CoreValueTrait,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Object(pub IndexMap<String, ValueContainer>);
impl Object {
    pub fn size(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, key: &str) -> Option<&ValueContainer> {
        self.0.get(key)
    }

    pub fn set<T: Into<ValueContainer>>(&mut self, key: &str, value: T) {
        self.0.insert(key.to_string(), value.into());
    }

    pub fn remove(&mut self, key: &str) -> Option<ValueContainer> {
        self.0.remove(key)
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

impl CoreValueTrait for Object {
}

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

impl <'a> IntoIterator for &'a Object {
    type Item = (&'a String, &'a ValueContainer);
    type IntoIter = Iter<'a, String, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}