use super::super::core_value_trait::CoreValueTrait;
use crate::values::core_value::CoreValue;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use indexmap::IndexMap;
use indexmap::map::{IntoIter, Iter};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tuple {
    pub entries: IndexMap<ValueContainer, ValueContainer>,
    next_int_key: u32,
}
impl Tuple {
    pub fn default() -> Self {
        Tuple {
            entries: IndexMap::new(),
            next_int_key: 0,
        }
    }
    pub fn new(entries: IndexMap<ValueContainer, ValueContainer>) -> Self {
        Tuple {
            entries,
            ..Tuple::default()
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// returns the next integer key in the tuple, starting from 0
    pub(crate) fn next_int_key(&self) -> u32 {
        self.next_int_key
    }

    pub fn get(&self, key: &ValueContainer) -> Option<&ValueContainer> {
        self.entries.get(key)
    }
    
    pub fn at(&self, index: usize) -> Option<(&ValueContainer, &ValueContainer)> {
        self.entries.get_index(index)
    }

    /// Set a key-value pair in the tuple. This method should only be used internal, since tuples
    /// are immutable after creation as per DATEX specification.
    pub(crate) fn set<K: Into<ValueContainer>, V: Into<ValueContainer>>(
        &mut self,
        key: K,
        value: V,
    ) {
        let key = key.into();
        // if key is integer and the expected next int key, increment the next_int_key
        if let ValueContainer::Value(Value {
            inner: CoreValue::Integer(typed_int),
            ..
        }) = key
            && let Some(int) = typed_int.0.as_i128()
            && int == self.next_int_key as i128
        {
            self.next_int_key += 1;
        }
        self.entries.insert(key, value.into());
    }
    pub fn insert<V: Into<ValueContainer>>(&mut self, value: V) {
        self.entries
            .insert(self.next_int_key().into(), value.into());
        self.next_int_key += 1;
    }
}

impl StructuralEq for Tuple {
    fn structural_eq(&self, other: &Self) -> bool {
        if self.size() != other.size() {
            return false;
        }
        for ((key, value), (other_key, other_value)) in
            self.entries.iter().zip(other.entries.iter())
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

impl Hash for Tuple {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (k, v) in &self.entries {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl CoreValueTrait for Tuple {}

impl Display for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        for (i, (key, value)) in self.entries.iter().enumerate() {
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
        Tuple::new(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<T> FromIterator<T> for Tuple
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Tuple::new(
            iter.into_iter()
                .enumerate()
                .map(|(i, v)| (TypedInteger::from(i as u64).into(), v.into()))
                .collect(),
        )
    }
}

impl IntoIterator for Tuple {
    type Item = (ValueContainer, ValueContainer);
    type IntoIter = IntoIter<ValueContainer, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tuple {
    type Item = (&'a ValueContainer, &'a ValueContainer);
    type IntoIter = Iter<'a, ValueContainer, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
impl From<Vec<(ValueContainer, ValueContainer)>> for Tuple {
    fn from(vec: Vec<(ValueContainer, ValueContainer)>) -> Self {
        Tuple::new(vec.into_iter().collect())
    }
}

impl From<IndexMap<ValueContainer, ValueContainer>> for Tuple {
    fn from(map: IndexMap<ValueContainer, ValueContainer>) -> Self {
        Tuple::new(map)
    }
}
impl From<IndexMap<String, ValueContainer>> for Tuple {
    fn from(map: IndexMap<String, ValueContainer>) -> Self {
        Tuple::new(
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
            Err(format!("Expected CoreValue::Tuple, found {value:?}"))
        }
    }
}
