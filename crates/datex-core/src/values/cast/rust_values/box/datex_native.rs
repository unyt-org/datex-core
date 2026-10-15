use crate::{
    prelude::*,
    traits::{
        classification::Classification, get_datex_type::GetDatexType,
        local_child_path_resolver::LocalChildPathResolver,
    },
    value_updates::errors::UpdateError,
    values::{
        core_values::native::{DatexNative, DatexNativeOps},
        value::Value,
        value_container::{ValueContainer},
    },
};
use core::any::Any;

impl<T: DatexNative + GetDatexType + Classification> DatexNative for Box<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl<T: DatexNative + GetDatexType + Classification> DatexNativeOps for Box<T> {}

impl<T: LocalChildPathResolver> LocalChildPathResolver for Box<T> {
    fn resolve_child(
        &mut self,
        key: &ValueContainer,
    ) -> Result<&mut ValueContainer, UpdateError> {
        self.as_mut().resolve_child(key)
    }

    fn resolve_value_for_path(
        &mut self,
        first: &ValueContainer,
        remaining_path: &[ValueContainer],
    ) -> Result<&mut Value, UpdateError> {
        self.as_mut().resolve_value_for_path(first, remaining_path)
    }
}
