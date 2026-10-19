use crate::{
    core_compiler::{
        to_instructions::{ToInstructions, yield_all},
        value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::range::Range,
};
use core::ops::Deref;

impl ToInstructions for Range {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield RegularInstruction::range().into();
            yield_all!(self.start.deref().to_bytecode_item(visitor));
            yield_all!(self.end.deref().to_bytecode_item(visitor));
        })
    }
}
