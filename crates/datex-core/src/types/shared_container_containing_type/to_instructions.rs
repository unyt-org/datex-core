use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    types::shared_container_containing_type::SharedContainerContainingType,
};
use core::ops::Deref;

impl ToInstructions for SharedContainerContainingType {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        visitor.visit_shared_container(self.deref())
    }
}
