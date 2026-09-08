use crate::{
    preludes::derive::DatexNative,
    values::core_values::{
        integer::{Integer, typed_integer::TypedInteger},
        native::{DatexNativeOps, add_native_impl_option},
    },
};
use core::{any::Any, ops::Add};

impl DatexNative for TypedInteger {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
// impl DatexNativeOps for TypedInteger {
//     fn add_native(
//         &self,
//         rhs: &dyn DatexNative,
//     ) -> Option<Box<dyn DatexNative>> {
//         if let Some(rhs) = rhs.as_any().downcast_ref::<TypedInteger>()
//             && let Some(lhs) = self.as_any().downcast_ref::<TypedInteger>()
//         {
//             Some(Box::new(lhs + rhs))
//         } else {
//             None
//         }
//     }
// }

impl DatexNativeOps for TypedInteger {
    fn add_native(
        &self,
        rhs: &dyn DatexNative,
    ) -> Option<Box<dyn DatexNative>> {
        add_native_impl_option(self, rhs)
    }
}
