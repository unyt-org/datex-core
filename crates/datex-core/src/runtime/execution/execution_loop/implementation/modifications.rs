//! This module contains the implementation of modifications that can be performed on [ValueContainer]s
use core::cell::RefCell;

use crate::{
    prelude::*,
    runtime::{
        cache::shared_references_cache::SharedReferencesCache,
        execution::ExecutionError,
    },
    shared_values::base_shared_value_container::observers::TransceiverId,
    value_updates::{
        update_data::{SetEntryUpdateData, UpdateOperation},
        update_handler::UpdateHandler,
    },
    values::value_container::{ValueContainer},
};

/// Sets a property on the target [ValueContainer] using the provided key and value.
/// If the property cannot be set, an [ExecutionError::UpdateError] is returned.
pub fn try_set_property(
    target: &mut ValueContainer,
    key: ValueContainer,
    value: ValueContainer,
    path: Vec<ValueContainer>,
    transceiver_id: TransceiverId,
    cache: &RefCell<SharedReferencesCache>,
) -> Result<Option<ValueContainer>, ExecutionError> {
    target
        .try_set_entry(
            path,
            transceiver_id,
            SetEntryUpdateData::new(key, value),
            cache,
        )
        .map_err(ExecutionError::from)
}

/// Sets the value of a shared container to the provided [ValueContainer].
/// If the target [ValueContainer] is not a shared container, an [ExecutionError::ExpectedSharedValue] is returned.
pub fn try_set_shared_container_value(
    target: &mut ValueContainer,
    new_value: ValueContainer,
    source_id: TransceiverId,
    cache: &RefCell<SharedReferencesCache>,
) -> Result<(), ExecutionError> {
    // TODO: check if caller endpoint can actually mutate the container
    if let Some(reference) = target.maybe_shared_mut() {
        let update =
            UpdateOperation::replace(new_value).with_source_root(source_id); // here we defintely want to update the container, we do not have a path to a value update
        Ok(reference.try_handle_update(update, cache).map(|_| ())?) // FIXME do we want to return the old value that was replaced from the execution?
    } else {
        Err(ExecutionError::ExpectedSharedValue)
    }
}
