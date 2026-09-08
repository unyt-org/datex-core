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
                    let ownership = shared_container.ownership();
                    let index = ctx
                        .shared_value_tracking()
                        .expect("Shared value tracking not initialized")
                        .borrow_mut()
                        .register_shared_value(shared_container);

                    yield match ownership {
                        SharedContainerOwnership::Owned => {
                            RegularInstruction::take_stack_value(index)
                        }
                        SharedContainerOwnership::Referenced(
                            ReferenceMutability::Immutable,
                        ) => RegularInstruction::get_stack_value_shared_ref(
                            index,
                        ),
                        SharedContainerOwnership::Referenced(
                            ReferenceMutability::Mutable,
                        ) => {
                            RegularInstruction::get_stack_value_shared_ref_mut(
                                index,
                            )
                        }
                    }
                    .into()
                }
            }
        })
    }
}
