use crate::{
    traits::local_child_path_resolver::LocalChildPathResolver,
    value_updates::errors::UpdateError,
    values::{
        core_value::CoreValue,
        value::Value,
        value_container::{ValueContainer, value_key::ValueKey},
    },
};

impl LocalChildPathResolver for Value {
    fn resolve_child(
        &mut self,
        _key: &ValueKey,
    ) -> Result<&mut ValueContainer, UpdateError> {
        unreachable!(
            "resolve_child should not be called on Value directly, it should be called on the inner CoreValue"
        )
    }

    fn resolve_value_for_path(
        &mut self,
        first: &ValueKey,
        remaining_path: &[ValueKey],
    ) -> Result<&mut Value, UpdateError> {
        match self {
            Value::Core(core_value) => match &mut core_value.inner {
                CoreValue::Map(map) => {
                    map.resolve_value_for_path(first, remaining_path)
                }
                CoreValue::List(list) => {
                    list.resolve_value_for_path(first, remaining_path)
                }
                _ => Err(UpdateError::InvalidUpdate),
            },
            Value::Native(native_value) => {
                native_value.resolve_value_for_path(first, remaining_path)
            }
        }
    }
}
