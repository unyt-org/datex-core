use crate::preludes::derive::DatexNative;

use crate::prelude::*;
pub trait DatexNativeOps {
    fn add_native(
        &self,
        _rhs: &dyn DatexNative,
    ) -> Option<Box<dyn DatexNative>> {
        None
    }
}
