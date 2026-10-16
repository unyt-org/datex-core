//! Helper trait to implement [into_regular_instruction] if a type can (without context) be converted
//! to a [RegularInstruction] directly.

use crate::instruction::regular_instruction::RegularInstruction;
use crate::core_compiler::to_instructions::ToInstructions;
use crate::core_compiler::value_visitor::ValueVisitor;
use crate::instruction::Instruction;


pub trait ToRegularInstruction {
    fn to_regular_instruction(&self) -> RegularInstruction;
}
impl<V> ToInstructions for V
where
    V: ToRegularInstruction,
{
    fn to_instructions<'a>(
        &'a self,
        _visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(core::iter::once(self.to_regular_instruction().into()))
    }
}

// TODO: still needed?
// #[macro_export]
// macro_rules! impl_regular_to_instructions {
//     ($($ty:ty),* $(,)?) => {
//         $(
//             impl $crate::core_compiler::traits::to_instructions::ToInstructions for $ty {
//                 fn to_instructions<'a>(
//                     &'a self,
//                     _ctx: &'a mut dyn $crate::core_compiler::value_visitor::ValueVisitor,
//                 ) -> alloc::boxed::Box<dyn Iterator<Item = $crate::instruction::Instruction> + 'a>
//                 {
//                     alloc::boxed::Box::new(core::iter::once(
//                         self.into_regular_instruction().into()
//                     ))
//                 }
//             }
//         )*
//     };
// }
// pub use impl_regular_to_instructions;
