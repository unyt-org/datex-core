use core::cell::RefCell;

use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::base_shared_value_container::BaseSharedValueContainer,
    types::traits::type_match::TypeSuperset,
    value_updates::{
        UpdateReturn, errors::UpdateError, update_data::UpdateOperation,
        update_handler::UpdateResult,
    },
    values::value_container::{ValueContainer, value_key::ValueKey},
};

impl BaseSharedValueContainer {
    pub(crate) fn try_handle_update(
        &mut self,
        operation: UpdateOperation,
        path: Vec<ValueKey>,
        cache: &RefCell<SharedReferencesCache>,
    ) -> UpdateResult {
        self.assert_can_mutate()?;

        // Validate while borrowing `update`.
        if let UpdateOperation::Replace(replace) = operation {
            if !self
                .allowed_type()
                .is_superset_of(&replace.value.actual_type())
            {
                // FIXE type check

                // return Err(UpdateError::type_error(
                //     TypeError::InvalidSharedReference,
                // ));
            }
            let previous =
                core::mem::replace(self.value_container_mut(), replace.value);
            return Ok(UpdateReturn::SingleValue(previous));
        }

        // Only perform updates on inner local values.
        // If the inner is not a local value, it should have been triggered via a direct
        // update for the inner shared container
        if let ValueContainer::Local(local_value) = &mut self.value_container {
            // Set source_id to None since we don't want the inner container to trigger its own observers.
            // The observers are already triggered from the parent shared container
            local_value
                .try_update_collapsed_local_inner(operation, path, None, cache)
        } else {
            Err(UpdateError::InvalidUpdate)
        }
    }
}
