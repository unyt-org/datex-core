use crate::preludes::derive::DatexNative;

pub trait DatexNativeOps {
    fn add_native(
        &self,
        rhs: &dyn DatexNative,
    ) -> Option<Box<dyn DatexNative>> {
        None
    }
}
