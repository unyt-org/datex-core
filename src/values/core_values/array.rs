use super::super::core_value_trait::CoreValueTrait;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::value_container::ValueContainer;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::vec::IntoIter;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Array(Vec<ValueContainer>);
impl Array {
    pub fn new<T: Into<ValueContainer>>(vec: Vec<T>) -> Self {
        Array(vec.into_iter().map(|v| v.into()).collect())
    }
    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }
    pub fn get_unchecked(&self, index: u32) -> &ValueContainer {
        self.get(index)
            .unwrap_or_else(|| panic!("Index '{index}' not found in Array"))
    }
    pub fn get_mut_unchecked(&mut self, index: u32) -> &mut ValueContainer {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("Index '{index}' not found in Array"))
    }
    pub fn get(&self, index: u32) -> Option<&ValueContainer> {
        self.0.get(index as usize)
    }
    pub fn get_mut(&mut self, index: u32) -> Option<&mut ValueContainer> {
        self.0.get_mut(index as usize)
    }
    pub fn has_index(&self, index: u32) -> bool {
        (index as usize) < self.0.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = &ValueContainer> {
        self.0.iter()
    }

    pub fn iter_slice(&'_ self) -> core::slice::Iter<'_, ValueContainer> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ValueContainer> {
        self.0.iter_mut()
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn set<T: Into<ValueContainer>>(&mut self, index: u32, value: T) {
        // check if index exists
        if index as usize >= self.0.len() {
            panic!(
                "Invalid index '{index}' for Array of size {}",
                self.0.len()
            );
        }
        self.0.insert(index as usize, value.into());
    }

    pub(crate) fn _push<T: Into<ValueContainer>>(&mut self, value: T) {
        self.0.push(value.into());
    }
}

impl StructuralEq for Array {
    fn structural_eq(&self, other: &Self) -> bool {
        // check size first
        if self.len() != other.len() {
            return false;
        }
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            if !a.structural_eq(b) {
                return false;
            }
        }
        true
    }
}

impl Hash for Array {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for v in &self.0 {
            v.hash(state);
        }
    }
}

impl CoreValueTrait for Array {}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for value in &self.0 {
            write!(f, "{value}")?;
        }
        write!(f, "]")
    }
}

impl<T> From<Vec<T>> for Array
where
    T: Into<ValueContainer>,
{
    fn from(map: Vec<T>) -> Self {
        Array::new(map)
    }
}

impl<T> FromIterator<T> for Array
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Array::new(iter.into_iter().collect())
    }
}

impl IntoIterator for Array {
    type Item = ValueContainer;
    type IntoIter = IntoIter<ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a ValueContainer;
    type IntoIter = std::slice::Iter<'a, ValueContainer>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[macro_export]
macro_rules! datex_array {
    ( $( $x:expr ),* ) => {
        {
            let arr = vec![$( $crate::values::value_container::ValueContainer::from($x) ),*];
            $crate::values::core_values::array::Array::new(arr)
        }
    };
}
