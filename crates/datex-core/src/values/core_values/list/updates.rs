use crate::{
    prelude::*,
    shared_values::{
        base_shared_value_container::observers::TransceiverId,
        errors::IndexOutOfBoundsError,
    },
    value_updates::{
        update_data::UpdateOperation,
        update_handler::InternalMutabilityUpdateHandler,
    },
    values::{
        core_values::list::List,
        value_container::{ValueContainer},
    },
};
use core::ops::Range;

impl List {
    /// Sets the value at the specified index.
    /// If the index is equal to the current length of the list, the value is pushed to the end.
    /// If the index is greater than the current length, None is returned.
    /// Returns the previous value at the index if it was replaced.
    pub fn try_set_with_source(
        &mut self,
        index: u32,
        mut value: ValueContainer,
        source_id: Option<TransceiverId>,
    ) -> Result<ValueContainer, IndexOutOfBoundsError> {
        // set update callback data for the new value if needed
        if self.update_callback_data.is_some() {
            self.set_child_update_callback_data_if_local(&index, &mut value);
        }

        // replace
        let res =
            core::mem::replace(&mut self.items[index as usize], value);

        // trigger update callback if needed
        self.maybe_trigger_update_callback(source_id, || {
            UpdateOperation::set_entry(
                ValueContainer::local(index),
                self.items[index as usize].clone(),
            )
        });

        Ok(res)
    }

    /// Tries to delete the value at the specified index, returning it if successful.
    /// If the index is out of bounds, an error is returned.
    pub fn try_delete_with_source(
        &mut self,
        index: u32,
        source_id: Option<TransceiverId>,
    ) -> Result<ValueContainer, IndexOutOfBoundsError> {
        let res = self
            .items
            .remove(index as usize)
            .without_local_observers();

        // trigger update callback if needed
        self.maybe_trigger_update_callback(source_id, || {
            UpdateOperation::delete_entry(ValueContainer::local(index))
        });

        Ok(res)
    }

    pub fn push_with_source<T: Into<ValueContainer>>(
        &mut self,
        value: T,
        source_id: Option<TransceiverId>,
    ) {
        let mut value = value.into();
        if self.update_callback_data.is_some() {
            self.set_child_update_callback_data_if_local(
                &(self.items.len() as u32),
                &mut value,
            );
        }
        self.items.push(value);

        // trigger update callback if needed
        self.maybe_trigger_update_callback(source_id, || {
            UpdateOperation::append_entry(self.items.last().unwrap().clone())
        });
    }

    pub fn pop_with_source(
        &mut self,
        source_id: Option<TransceiverId>,
    ) -> Option<ValueContainer> {
        let res = self
            .items
            .pop()
            .map(|value| value.without_local_observers());

        self.maybe_trigger_update_callback(source_id, || {
            UpdateOperation::delete_entry(ValueContainer::local(
                self.items.len() as i64
            ))
        });

        res
    }

    pub fn clear_with_source(&mut self, source_id: Option<TransceiverId>) {
        self.items.clear();

        self.maybe_trigger_update_callback(source_id, || {
            UpdateOperation::clear()
        });
    }

    pub fn splice_with_source(
        &mut self,
        range: Range<u32>,
        replace_with: impl IntoIterator<Item = ValueContainer>,
        source_id: Option<TransceiverId>,
    ) -> Vec<ValueContainer> {
        let range = Range {
            start: range.start as usize,
            end: range.end as usize,
        };
        let start = range.start;
        let res = self.items.splice(range, replace_with).collect::<Vec<_>>();

        self.maybe_trigger_update_callback(source_id, || {
            UpdateOperation::list_splice(
                start as u32,
                res.len() as u32,
                self.items[start..].to_vec(),
            )
        });

        res
    }
}
