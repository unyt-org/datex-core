use crate::{
    prelude::*,
    traits::local_child_path_resolver::LocalChildPathResolver,
    value_updates::errors::UpdateError,
    values::{
        core_values::native::{DatexNative, DatexNativeBase, DatexNativeOps},
        value::Value,
        value_container::ValueContainer,
    },
};
use core::any::Any;

impl<T: DatexNativeBase + 'static> DatexNative for Vec<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl<T: DatexNativeBase + 'static> DatexNativeOps for Vec<T> {}
impl<T: DatexNativeBase + 'static> LocalChildPathResolver for Vec<T> {
    fn resolve_value_for_path(
        &mut self,
        _first: &ValueContainer,
        _remaining_path: &[ValueContainer],
    ) -> Result<&mut Value, UpdateError> {
        todo!()
    }
}
