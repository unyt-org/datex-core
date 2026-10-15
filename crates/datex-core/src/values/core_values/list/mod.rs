//! This module contains the implementation of the [List] struct, which represents a list of values in the type system.

use crate::{
    prelude::*, shared_values::errors::IndexOutOfBoundsError,
    values::value_container::ValueContainer,
};
pub mod equality;
pub mod serde_dif;
mod to_instructions;
use crate::{
    shared_values::base_shared_value_container::observers::TransceiverId,
    utils::impl_display_for_datex_value::impl_display_for_datex_value,
    value_updates::update_handler::{
        InternalMutabilityUpdateHandler, UpdateCallbackData,
    },
    values::value::Value,
};
use core::{
    fmt::Display,
    ops::{Index, Range},
    result::Result,
};

mod classification;
mod convert_parts;
mod datex_hash;
mod datex_native;
mod datex_native_structural;
mod get_core_lib_type_id;
mod get_datex_type;
mod iter_parts;
pub mod local_child_path_resolver;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod try_clone;
pub mod update_handler;
pub mod updates;
mod value_access;

#[derive(Debug, Default)]
pub struct List {
    items: Vec<ValueContainer>,
    /// Optional observer callback for local values. This is used to notify observers of changes to the value.
    pub update_callback_data: Option<UpdateCallbackData>,
}

impl Clone for List {
    fn clone(&self) -> Self {
        List {
            items: self.items.clone(),
            update_callback_data: None,
        }
    }
}

impl List {
    pub fn new<T: Into<ValueContainer>>(values: Vec<T>) -> Self {
        List {
            items: values.into_iter().map(Into::into).collect(),
            update_callback_data: None,
        }
    }
    pub fn with_capacity(capacity: u32) -> Self {
        List {
            items: Vec::with_capacity(capacity as usize),
            update_callback_data: None,
        }
    }
    pub fn len(&self) -> u32 {
        self.items.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn try_get(
        &self,
        index: i64,
    ) -> Result<&ValueContainer, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        self.items
            .get(index as usize)
            .ok_or(IndexOutOfBoundsError { index })
    }

    pub fn try_get_mut(
        &mut self,
        index: i64,
    ) -> Result<&mut ValueContainer, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        self.items
            .get_mut(index as usize)
            .ok_or(IndexOutOfBoundsError { index })
    }

    pub fn as_vec(&self) -> &Vec<ValueContainer> {
        &self.items
    }

    pub fn into_vec(self) -> Vec<ValueContainer> {
        self.items
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<ValueContainer> {
        &mut self.items
    }

    pub fn iter(&self) -> core::slice::Iter<'_, ValueContainer> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, ValueContainer> {
        self.items.iter_mut()
    }

    /// Returns an iterator over the local values in the list,
    /// skipping any children that are [ValueContainer::Shared]
    pub fn iter_local_values_mut(
        &mut self,
    ) -> impl Iterator<Item = (u32, &mut Value)> {
        self.items
            .iter_mut()
            .enumerate()
            .filter_map(|(index, item)| {
                if let ValueContainer::Local(local_value) = item {
                    Some((index as u32, local_value))
                } else {
                    None
                }
            })
    }

    /// if index is negative, count from the end
    #[inline]
    fn wrap_index(&self, index: i64) -> u32 {
        if index < 0 {
            (index + self.items.len() as i64) as u32
        } else {
            index as u32
        }
    }

    #[inline]
    fn get_valid_index(
        &self,
        index: i64,
    ) -> Result<u32, IndexOutOfBoundsError> {
        let index = self.wrap_index(index);
        if (index as usize) < self.items.len() {
            Ok(index)
        } else {
            Err(IndexOutOfBoundsError { index })
        }
    }

    /// Sets the value at the specified index.
    /// If the index is equal to the current length of the list, the value is pushed to the end.
    /// If the index is greater than the current length, None is returned.
    /// Returns the previous value at the index if it was replaced.
    pub fn try_set(
        &mut self,
        index: u32,
        value: ValueContainer,
    ) -> Result<ValueContainer, IndexOutOfBoundsError> {
        self.try_set_with_source(index, value, Some(TransceiverId::Local))
    }

    /// Tries to delete the value at the specified index, returning it if successful.
    /// If the index is out of bounds, an error is returned.
    pub fn try_delete(
        &mut self,
        index: u32,
    ) -> Result<ValueContainer, IndexOutOfBoundsError> {
        self.try_delete_with_source(index, Some(TransceiverId::Local))
    }

    pub fn push<T: Into<ValueContainer>>(&mut self, value: T) {
        self.push_with_source(value, Some(TransceiverId::Local))
    }

    pub fn pop(&mut self) -> Option<ValueContainer> {
        self.pop_with_source(Some(TransceiverId::Local))
    }

    pub fn clear(&mut self) {
        self.clear_with_source(Some(TransceiverId::Local))
    }

    pub fn splice(
        &mut self,
        range: Range<u32>,
        replace_with: impl IntoIterator<Item = ValueContainer>,
    ) -> Vec<ValueContainer> {
        self.splice_with_source(range, replace_with, Some(TransceiverId::Local))
    }
}

impl_display_for_datex_value!(
    List,
    impl core::fmt::Display for List {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            core::write!(f, "[")?;
            for (i, value) in self.items.iter().enumerate() {
                if i > 0 {
                    core::write!(f, ", ")?;
                }
                core::write!(f, "{value}")?;
            }
            core::write!(f, "]")
        }
    }
);

impl<T> From<Vec<T>> for List
where
    T: Into<ValueContainer>,
{
    fn from(vec: Vec<T>) -> Self {
        List {
            items: vec.into_iter().map(Into::into).collect(),
            update_callback_data: None,
        }
    }
}

impl<T> FromIterator<T> for List
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        List {
            items: iter.into_iter().map(Into::into).collect(),
            update_callback_data: None,
        }
    }
}

impl Index<usize> for List {
    type Output = ValueContainer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl IntoIterator for List {
    type Item = ValueContainer;
    type IntoIter = vec::IntoIter<ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a List {
    type Item = &'a ValueContainer;
    type IntoIter = core::slice::Iter<'a, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

pub macro datex_list {
    ( $( $x:expr ),* ) => {
        {
            let list = alloc::vec![$( $crate::values::value_container::ValueContainer::from($x) ),*];
            $crate::values::core_values::list::List::new(list)
        }
    }
}

impl From<List> for Vec<ValueContainer> {
    fn from(list: List) -> Self {
        list.items
    }
}
