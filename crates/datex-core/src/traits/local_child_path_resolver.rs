use crate::{
    value_updates::errors::UpdateError,
    values::{
        value::Value,
        value_container::{ValueContainer},
    },
};

/// Trait that can be implemented on core values that allows the resolution
/// of child paths for nested local values.
pub trait LocalChildPathResolver {
    fn resolve_child(
        &mut self,
        _key: &ValueContainer,
    ) -> Result<&mut ValueContainer, UpdateError> {
        Err(UpdateError::InvalidUpdate)
    }

    fn resolve_value_for_path(
        &mut self,
        first: &ValueContainer,
        remaining_path: &[ValueContainer],
    ) -> Result<&mut Value, UpdateError> {
        let child = self.resolve_child(first)?;

        if let ValueContainer::Local(child) = child {
            if let Some(([first], remaining_path)) =
                remaining_path.split_at_checked(1)
            {
                child.resolve_value_for_path(first, remaining_path)
            } else {
                Ok(child)
            }
        } else {
            Err(UpdateError::InvalidUpdate)
        }
    }
}
