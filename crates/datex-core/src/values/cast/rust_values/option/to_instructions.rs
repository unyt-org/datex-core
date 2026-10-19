use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
};

impl<K> ToInstructions for Option<K>
where
    K: ToInstructions,
{
    fn to_bytecode_item<'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            match self {
                None => yield RegularInstruction::Null.into(),
                Some(value) => {
                    for instruction in value.to_bytecode_item(_ctx) {
                        yield instruction;
                    }
                }
            }
        })
    }
}
