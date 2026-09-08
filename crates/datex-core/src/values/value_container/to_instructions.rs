use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    preludes::derive::RegularInstruction,
    shared_values::{
        ReferenceMutability, SharedContainerOwnership,
        traits::SharedContainerCommon,
    },
    values::value_container::ValueContainer,
};

impl ToInstructions for ValueContainer {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            match self {
                ValueContainer::Local(value) => {
                    for instruction in value.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                ValueContainer::Shared(shared_container) => {
                    for instruction in shared_container.to_instructions(ctx) {
                        yield instruction;
                    }
                }
            }
        })
    }
}
