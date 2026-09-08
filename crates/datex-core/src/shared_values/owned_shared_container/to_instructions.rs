use crate::{
    instruction::Instruction,
    prelude::*,
    preludes::derive::{ToInstructions, ValueVisitor},
    shared_values::{OwnedSharedContainer, SharedContainer},
};

impl ToInstructions for OwnedSharedContainer {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        let reference =
            SharedContainer::Referenced(self.clone_with_move_indicator());
        Box::new(gen move {
            for instruction in
                reference.to_instructions(ctx).collect::<Vec<_>>()
            {
                yield instruction;
            }
        })
    }
}
