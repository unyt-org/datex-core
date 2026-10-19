use crate::{
    core_compiler::{
        to_instructions::{ToInstructions, yield_all},
        value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
};

impl<V> ToInstructions for Box<V>
where
    V: ToInstructions + ?Sized,
{
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield RegularInstruction::boxed_value().into();
            yield_all!(self.as_ref().to_bytecode_item(visitor))
        })
    }
}
