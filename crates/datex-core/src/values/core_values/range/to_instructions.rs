use std::ops::Deref;
use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::range::Range,
};
use crate::core_compiler::to_instructions::yield_all;

impl ToInstructions for Range {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::range().into();
            yield_all!(self.start.deref().to_instructions(visitor));
            yield_all!(self.end.deref().to_instructions(visitor));
        })
    }
}
