use super::super::core_value_trait::CoreValueTrait;
use crate::references::reference::IndexOutOfBoundsError;
use crate::stdlib::ops::Index;
use crate::stdlib::vec::Vec;
use crate::traits::structural_eq::StructuralEq;
use crate::values::{
    core_value::CoreValue,
    value_container::{ValueContainer, ValueError},
};
use core::fmt::Display;
use core::ops::Range;
use core::prelude::rust_2024::*;
use core::result::Result;

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
    pub fn get(
        &self,
        index: i64,
    ) -> Result<&ValueContainer, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        self.0
            .get(index as usize)
            .ok_or(IndexOutOfBoundsError { index })
    }

    /// Sets the value at the specified index.
    /// If the index is equal to the current length of the list, the value is pushed to the end.
    /// If the index is greater than the current length, None is returned.
    /// Returns the previous value at the index if it was replaced.
    pub fn set(
        &mut self,
        index: i64,
        value: ValueContainer,
    ) -> Result<ValueContainer, IndexOutOfBoundsError> {
        let index = self.get_valid_index(index)?;
        // replace
        Ok(core::mem::replace(&mut self.0[index], value))
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

    pub fn iter(&self) -> core::slice::Iter<'_, ValueContainer> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, ValueContainer> {
        self.0.iter_mut()
    }

    pub fn splice(
        &mut self,
        range: Range<u32>,
        replace_with: impl IntoIterator<Item = ValueContainer>,
    ) {
        let range = Range {
            start: range.start as usize,
            end: range.end as usize,
        };
        let _: Vec<_> = self.0.splice(range, replace_with).collect();
    }

    /// if index is negative, count from the end
    #[inline]
    fn wrap_index(&self, index: i64) -> u32 {
        if index < 0 {
            (index + self.0.len() as i64) as u32
        } else {
            index as u32
        }
    }

    #[inline]
    fn get_valid_index(
        &self,
        index: i64,
    ) -> Result<usize, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        if (index as usize) < self.0.len() {
            Ok(index as usize)
        } else {
            Err(IndexOutOfBoundsError { index })
        }
    }

    pub fn delete(
        &mut self,
        index: i64,
    ) -> Result<ValueContainer, IndexOutOfBoundsError> {
        let index = self.get_valid_index(index)?;
        Ok(self.0.remove(index))
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

impl Display for List {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
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
    type IntoIter = crate::stdlib::vec::IntoIter<ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a List {
    type Item = &'a ValueContainer;
    type IntoIter = core::slice::Iter<'a, ValueContainer>;

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
