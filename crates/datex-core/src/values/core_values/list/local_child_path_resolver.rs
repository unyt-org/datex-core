use crate::{
    shared_values::errors::AccessError,
    traits::local_child_path_resolver::LocalChildPathResolver,
    value_updates::errors::UpdateError,
    values::{
        core_values::list::List,
        value_container::{ValueContainer},
    },
};

impl LocalChildPathResolver for List {
    fn resolve_child(
        &mut self,
        key: &ValueContainer,
    ) -> Result<&mut ValueContainer, UpdateError> {
        // TODO: cast integers to u32?
        if let Some(index) = key.try_as::<u32>() {
            self.try_get_mut(*index as i64).map_err(UpdateError::access_error)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidIndexKey))
        }
    }
}
