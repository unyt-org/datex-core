use crate::{
    prelude::*,
    preludes::derive::DatexNative,
    values::core_values::native::{DatexNativeBase, DatexNativeOps},
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
