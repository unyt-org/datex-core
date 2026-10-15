use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::errors::AccessError,
    values::{
        core_values::list::List,
        value_container::{ValueContainer},
    },
};

use crate::value_updates::{
    errors::UpdateError,
    update_data::{
        AppendEntryUpdateData, DeleteEntryUpdateData, ListSpliceUpdateData,
        SetEntryUpdateData,
    },
    update_handler::{
        InternalMutabilityUpdateHandler, UpdateCallbackData,
        UpdateCallbackDataAccess, UpdateHandlerImpl,
    },
};
use core::{cell::RefCell, result::Result};

impl InternalMutabilityUpdateHandler for List {
    fn set_update_callback_data(
        &mut self,
        observe_data: Option<UpdateCallbackData>,
    ) {
        // Update the update callback data for all child values
        for (index, child) in self.iter_local_values_mut() {
            child.set_update_callback_data(
                observe_data
                    .as_ref()
                    .map(|data| data.with_child_path(index)),
            );
        }
        // Update the update callback data for the list itself
        self.update_callback_data = observe_data;
    }
}

impl UpdateCallbackDataAccess for List {
    fn get_update_callback_data(&self) -> Option<&UpdateCallbackData> {
        self.update_callback_data.as_ref()
    }
}
impl UpdateHandlerImpl for List {
    fn try_set_entry(
        &mut self,
        data: SetEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Option<ValueContainer>, UpdateError> {
        let key = data.key.try_as::<u32>().ok_or_else(
            || UpdateError::access_error(AccessError::InvalidIndexKey),
        )?;
        self.try_set_with_source(*key, data.value, None)
            .map(Some)
            .map_err(UpdateError::access_error)
    }

    fn try_delete_entry(
        &mut self,
        data: DeleteEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        let key = data.key.try_as::<u32>().ok_or_else(
            || UpdateError::access_error(AccessError::InvalidIndexKey),
        )?;
        self.try_delete_with_source(*key, None)
            .map_err(UpdateError::access_error)
    }

    fn try_append_entry(
        &mut self,
        data: AppendEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<(), UpdateError> {
        self.push_with_source(data.value, None);
        Ok(())
    }

    fn try_clear(
        &mut self,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        let previous = core::mem::take(self);
        Ok(ValueContainer::Local(previous.into()))
    }

    fn try_list_splice(
        &mut self,
        data: ListSpliceUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Vec<ValueContainer>, UpdateError> {
        Ok(self.splice_with_source(
            data.start..(data.start + data.delete_count),
            data.items,
            None,
        ))
    }
}
