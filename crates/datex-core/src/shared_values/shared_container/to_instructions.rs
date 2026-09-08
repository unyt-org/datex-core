use crate::{
    instruction::Instruction,
    prelude::*,
    preludes::derive::{RegularInstruction, ToInstructions, ValueVisitor},
    shared_values::{
        ReferenceMutability, SharedContainer, SharedContainerOwnership,
        traits::SharedContainerCommon,
    },
};

impl ToInstructions for SharedContainer {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        let ownership = self.ownership();
        let index = ctx
            .shared_value_tracking()
            .expect("Shared value tracking not initialized")
            .borrow_mut()
            .register_shared_value(self);

        Box::new(gen move {
            yield match ownership {
                SharedContainerOwnership::Owned => {
                    RegularInstruction::take_stack_value(index)
                }
                SharedContainerOwnership::Referenced(
                    ReferenceMutability::Immutable,
                ) => RegularInstruction::get_stack_value_shared_ref(index),
                SharedContainerOwnership::Referenced(
                    ReferenceMutability::Mutable,
                ) => RegularInstruction::get_stack_value_shared_ref_mut(index),
            }
            .into()
        })
    }
}
