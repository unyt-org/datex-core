use crate::{
    preludes::derive::DatexNative,
    values::core_values::{native::DatexNativeOps, text::Text},
};
use core::any::Any;

impl DatexNative for Text {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl DatexNativeOps for Text {}
