use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    shared_values::{OwnedSharedContainer, SharedContainer},
};

impl ToInstructions for OwnedSharedContainer {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        let reference = SharedContainer::Referenced(self.clone_with_move_indicator());
        let instructions: Vec<Instruction> = reference.to_instructions(visitor).collect(); // FIXME: no collect
        Box::new(instructions.into_iter())
    }
}
