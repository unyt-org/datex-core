use crate::{
    traits::local_child_path_resolver::LocalChildPathResolver,
    value_updates::errors::UpdateError,
    values::{
        core_values::map::Map,
        value_container::{ValueContainer},
    },
};

impl LocalChildPathResolver for Map {
    fn resolve_child(
        &mut self,
        key: &ValueContainer,
    ) -> Result<&mut ValueContainer, UpdateError> {
        self.try_get_mut(key).map_err(UpdateError::access_error)
    }
}
