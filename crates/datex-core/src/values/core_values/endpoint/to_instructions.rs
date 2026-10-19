use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::endpoint::Endpoint,
};

impl ToInstructions for Endpoint {
    fn to_bytecode_item<'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(core::iter::once(
            RegularInstruction::endpoint(self.clone()).into(),
        ))
    }
}
