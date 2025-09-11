use super::super::core_value_trait::CoreValueTrait;
use crate::values::core_values::array::Array;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;
use indexmap::IndexMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::vec::IntoIter;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Struct(Array, Vec<String>);
impl Struct {
    pub fn new(fields: Vec<(String, ValueContainer)>) -> Self {
        let (field_names, values): (Vec<_>, Vec<_>) =
            fields.into_iter().unzip();
        Self::new_with_fields(values, field_names)
    }
    pub fn new_with_fields<T: Into<ValueContainer>>(
        vec: Vec<T>,
        fields: Vec<String>,
    ) -> Self {
        Struct(vec.into_iter().map(|v| v.into()).collect(), fields)
    }
    pub fn size(&self) -> usize {
        self.0.size()
    }
    pub fn at_unchecked(&self, index: u32) -> &ValueContainer {
        self.at(index)
            .unwrap_or_else(|| panic!("Index '{index}' not found in Struct"))
    }
    pub fn at_mut_unchecked(&mut self, index: u32) -> &mut ValueContainer {
        self.at_mut(index)
            .unwrap_or_else(|| panic!("Index '{index}' not found in Struct"))
    }
    pub fn at(&self, index: u32) -> Option<&ValueContainer> {
        self.0.get(index)
    }
    pub fn at_mut(&mut self, index: u32) -> Option<&mut ValueContainer> {
        self.0.get_mut(index)
    }

    pub fn get(&self, field: &str) -> Option<&ValueContainer> {
        if let Some(pos) = self.1.iter().position(|f| f == field) {
            self.0.get(pos as u32)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, field: &str) -> Option<&mut ValueContainer> {
        if let Some(pos) = self.1.iter().position(|f| f == field) {
            self.0.get_mut(pos as u32)
        } else {
            None
        }
    }

    pub fn get_unchecked(&self, field: &str) -> &ValueContainer {
        self.get(field)
            .unwrap_or_else(|| panic!("Field '{field}' not found in Struct"))
    }

    pub fn get_mut_unchecked(&mut self, field: &str) -> &mut ValueContainer {
        self.get_mut(field)
            .unwrap_or_else(|| panic!("Field '{field}' not found in Struct"))
    }

    pub fn has_index(&self, index: u32) -> bool {
        self.0.has_index(index)
    }

    pub fn values(&self) -> impl Iterator<Item = &ValueContainer> {
        self.0.iter()
    }
    pub fn fields(&self) -> impl Iterator<Item = &String> {
        self.1.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (String, &ValueContainer)> {
        self.1.iter().cloned().zip(self.0.iter())
    }
    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (String, &mut ValueContainer)> {
        let fields = self.1.clone();
        fields.into_iter().zip(self.0.iter_mut())
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn set_at<T: Into<ValueContainer>>(&mut self, index: u32, value: T) {
        if index as usize >= self.1.len() {
            panic!(
                "Index '{index}' out of bounds for Struct of size {}",
                self.1.len()
            );
        }
        self.0.set(index, value);
    }
    pub fn set<T: Into<ValueContainer>>(&mut self, field: &str, value: T) {
        if let Some(pos) = self.1.iter().position(|f| f == field) {
            self.0.set(pos as u32, value);
        } else {
            panic!("Field '{field}' not found in Struct");
        }
    }
}

impl StructuralEq for Struct {
    fn structural_eq(&self, _other: &Self) -> bool {
        unreachable!("Struct does not support StructuralEq")
    }
}

impl Hash for Struct {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        unreachable!("Struct does not support Hashing")
    }
}

impl CoreValueTrait for Struct {}

impl fmt::Display for Struct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (key, value) in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, r#""{}": {}"#, key, value)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl<T> From<IndexMap<String, T>> for Struct
where
    T: Into<ValueContainer>,
{
    fn from(map: IndexMap<String, T>) -> Self {
        let fields = map.keys().cloned().collect();
        let values = map.into_values().map(|v| v.into()).collect();
        Struct::new_with_fields(values, fields)
    }
}

impl<T> FromIterator<(String, T)> for Struct
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        let map: IndexMap<String, T> = iter.into_iter().collect();
        Struct::from(map)
    }
}

impl IntoIterator for Struct {
    type Item = (String, ValueContainer);
    type IntoIter = std::iter::Zip<IntoIter<String>, IntoIter<ValueContainer>>;
    fn into_iter(self) -> Self::IntoIter {
        self.1.into_iter().zip(self.0)
    }
}

impl<T> From<Vec<(String, T)>> for Struct
where
    T: Into<ValueContainer>,
{
    fn from(vec: Vec<(String, T)>) -> Self {
        Struct::new(vec.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}
