use crate::{
    preludes::derive::DatexNative,
    values::core_values::{map::Map, native::DatexNativeOps},
};
use core::any::Any;

impl DatexNative for Map {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl DatexNativeOps for Map {}
