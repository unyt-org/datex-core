use super::super::core_value_trait::CoreValueTrait;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::integer::Integer;
use crate::datex_values::value_container::ValueContainer;
use indexmap::map::{IntoIter, Iter};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tuple(pub IndexMap<ValueContainer, ValueContainer>);
impl Tuple {
    pub fn size(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, key: &ValueContainer) -> Option<&ValueContainer> {
        self.0.get(key)
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
}

impl Hash for Tuple {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl CoreValueTrait for Tuple {}

impl fmt::Display for Tuple {
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

impl<K, V> From<HashMap<K, V>> for Tuple
where
    K: Into<ValueContainer>,
    V: Into<ValueContainer>,
{
    fn from(map: HashMap<K, V>) -> Self {
        Tuple(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<T> FromIterator<T> for Tuple
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Tuple(
            iter.into_iter()
                .enumerate()
                .map(|(i, v)| (Integer::from(i as u64).into(), v.into()))
                .collect(),
        )
    }
}

impl IntoIterator for Tuple {
    type Item = (ValueContainer, ValueContainer);
    type IntoIter = IntoIter<ValueContainer, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tuple {
    type Item = (&'a ValueContainer, &'a ValueContainer);
    type IntoIter = Iter<'a, ValueContainer, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl From<Vec<(ValueContainer, ValueContainer)>> for Tuple {
    fn from(vec: Vec<(ValueContainer, ValueContainer)>) -> Self {
        Tuple(vec.into_iter().collect())
    }
}

impl From<IndexMap<ValueContainer, ValueContainer>> for Tuple {
    fn from(map: IndexMap<ValueContainer, ValueContainer>) -> Self {
        Tuple(map)
    }
}
impl From<IndexMap<String, ValueContainer>> for Tuple {
    fn from(map: IndexMap<String, ValueContainer>) -> Self {
        Tuple(
            map.into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect::<IndexMap<ValueContainer, ValueContainer>>(),
        )
    }
}
impl TryFrom<CoreValue> for Tuple {
    type Error = String;

    fn try_from(value: CoreValue) -> Result<Self, Self::Error> {
        if let CoreValue::Tuple(tuple) = value {
            Ok(tuple)
        } else {
            Err(format!("Expected CoreValue::Tuple, found {:?}", value))
        }
    }
}
