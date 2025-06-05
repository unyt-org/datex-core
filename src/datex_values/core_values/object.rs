use std::{
    fmt,
};
use std::collections::HashMap;
use crate::datex_values::value_container::ValueContainer;
use super::super::{
    core_value::CoreValue,
    datex_type::CoreValueType,
    value::Value,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object(pub HashMap<String, ValueContainer>);
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
impl CoreValue for Object {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: CoreValueType) -> Option<Value> {
        match target {
            CoreValueType::Object => Some(self.as_datex_value()),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(self.clone())
    }

    fn get_type(&self) -> CoreValueType {
        Self::static_type()
    }

    fn static_type() -> CoreValueType {
        CoreValueType::Object
    }
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
    type IntoIter = std::collections::hash_map::IntoIter<String, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl <'a> IntoIterator for &'a Object {
    type Item = (&'a String, &'a ValueContainer);
    type IntoIter = std::collections::hash_map::Iter<'a, String, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}