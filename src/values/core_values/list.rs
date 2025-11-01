use super::super::core_value_trait::CoreValueTrait;
use crate::values::{
    core_value::CoreValue,
    value_container::{ValueContainer, ValueError},
};
use crate::stdlib::{fmt, ops::Index};

use crate::traits::structural_eq::StructuralEq;
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct List(Vec<ValueContainer>);
impl List {
    pub fn new<T: Into<ValueContainer>>(values: Vec<T>) -> Self {
        List(values.into_iter().map(Into::into).collect())
    }
    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn get(&self, index: u32) -> Option<&ValueContainer> {
        self.0.get(index as usize)
    }

    /// Sets the value at the specified index.
    /// If the index is equal to the current length of the list, the value is pushed to the end.
    /// If the index is greater than the current length, None is returned.
    /// Returns the previous value at the index if it was replaced.
    // FIXME #321: Add set error when out of bounds, deprecate push case here?
    pub fn set(
        &mut self,
        index: u32,
        value: ValueContainer,
    ) -> Option<ValueContainer> {
        // replace
        if (index as usize) < self.0.len() {
            Some(std::mem::replace(&mut self.0[index as usize], value))
        }
        // push
        else if (index as usize) == self.0.len() {
            self.0.push(value);
            None
        }
        // out of bounds
        else {
            None
        }
    }

    pub fn push<T: Into<ValueContainer>>(&mut self, value: T) {
        self.0.push(value.into());
    }

    pub fn pop(&mut self) -> Option<ValueContainer> {
        self.0.pop()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn as_vec(&self) -> &Vec<ValueContainer> {
        &self.0
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<ValueContainer> {
        &mut self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ValueContainer> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, ValueContainer> {
        self.0.iter_mut()
    }
}

impl CoreValueTrait for List {}

impl StructuralEq for List {
    fn structural_eq(&self, other: &Self) -> bool {
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

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "[")?;
        for (i, value) in self.0.iter().enumerate() {
            if i > 0 {
                core::write!(f, ", ")?;
            }
            core::write!(f, "{value}")?;
        }
        core::write!(f, "]")
    }
}

impl<T> From<Vec<T>> for List
where
    T: Into<ValueContainer>,
{
    fn from(vec: Vec<T>) -> Self {
        List(vec.into_iter().map(Into::into).collect())
    }
}

impl<T> FromIterator<T> for List
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        List(iter.into_iter().map(Into::into).collect())
    }
}

impl Index<usize> for List {
    type Output = ValueContainer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IntoIterator for List {
    type Item = ValueContainer;
    type IntoIter = std::vec::IntoIter<ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a List {
    type Item = &'a ValueContainer;
    type IntoIter = std::slice::Iter<'a, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[macro_export]
macro_rules! datex_list {
    ( $( $x:expr ),* ) => {
        {
            let list = vec![$( $crate::values::value_container::ValueContainer::from($x) ),*];
            $crate::values::core_values::list::List::new(list)
        }
    };
}

impl TryFrom<CoreValue> for List {
    type Error = ValueError;
    fn try_from(value: CoreValue) -> Result<Self, Self::Error> {
        if let Some(list) = value.cast_to_list() {
            return Ok(list);
        }
        Err(ValueError::TypeConversionError)
    }
}

impl From<List> for Vec<ValueContainer> {
    fn from(list: List) -> Self {
        list.0
    }
}
