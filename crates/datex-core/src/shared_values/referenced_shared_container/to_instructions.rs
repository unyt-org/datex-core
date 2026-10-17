use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    shared_values::{SharedContainer},
};
use crate::shared_values::ReferencedSharedContainer;

impl ToInstructions for ReferencedSharedContainer {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        let reference = SharedContainer::Referenced(self.clone());
        let instructions: Vec<Instruction> = reference.to_instructions(visitor).collect(); // FIXME: no collect
        Box::new(instructions.into_iter())
    }
}
