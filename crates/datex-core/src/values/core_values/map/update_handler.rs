use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    values::{
        core_values::map::Map,
        value_container::{ValueContainer},
    },
};

use crate::value_updates::{
    errors::UpdateError,
    update_data::{DeleteEntryUpdateData, SetEntryUpdateData},
    update_handler::{
        InternalMutabilityUpdateHandler, UpdateCallbackData,
        UpdateCallbackDataAccess, UpdateHandlerImpl,
    },
};
use core::{cell::RefCell, result::Result};
use crate::values::value_container::borrowed_value_container::BorrowedValueContainer;

impl InternalMutabilityUpdateHandler for Map {
    fn set_update_callback_data(
        &mut self,
        observe_data: Option<UpdateCallbackData>,
    ) {
        // Update the update callback data for all child values
        for (key, child) in self.iter_local_values_mut() {
            child.set_update_callback_data(
                observe_data.as_ref().map(|data| {
                    data.with_child_path(
                        key.clone(),
                    )
                }), // TODO: what happens if key can't be cloned here? (should not be allowed)
            );
        }
        // Update the update callback data for the list itself
        self.update_callback_data = observe_data;
    }
}

impl UpdateCallbackDataAccess for Map {
    fn get_update_callback_data(&self) -> Option<&UpdateCallbackData> {
        self.update_callback_data.as_ref()
    }
}

impl UpdateHandlerImpl for Map {
    fn try_set_entry(
        &mut self,
        data: SetEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Option<ValueContainer>, UpdateError> {
        self.try_set_with_source(data.key, data.value, None)
            .map_err(UpdateError::access_error)
    }

    fn try_delete_entry(
        &mut self,
        data: DeleteEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        self.try_delete_with_source(&data.key, None)
            .map_err(UpdateError::access_error)
    }

    fn try_clear(
        &mut self,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        self.try_clear_with_source(None)
            .map_err(UpdateError::access_error)
    }
}
