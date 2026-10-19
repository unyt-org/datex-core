use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    shared_values::{ReferencedSharedContainer, SharedContainer},
};

impl ToInstructions for ReferencedSharedContainer {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        let reference = SharedContainer::Referenced(self.clone());
        let instructions: Vec<_> = reference.to_bytecode_item(visitor).collect(); // FIXME: no collect
        Box::new(instructions.into_iter())
    }
}
