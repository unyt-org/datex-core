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
    fn to_instructions<'ctx, 'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            match self {
                None => {
                    yield Instruction::Regular(RegularInstruction::Null);
                }
                Some(value) => {
                    for instruction in value.to_instructions(_ctx) {
                        yield instruction;
                    }
                }
            }
        })
    }
}
