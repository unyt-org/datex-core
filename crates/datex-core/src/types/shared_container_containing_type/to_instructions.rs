use core::ops::Deref;
use crate::core_compiler::to_instructions::ToInstructions;
use crate::core_compiler::value_visitor::ValueVisitor;
use crate::instruction::Instruction;
use crate::types::shared_container_containing_type::SharedContainerContainingType;

impl ToInstructions for SharedContainerContainingType {
    fn to_instructions<'a>(&'a self, visitor: &'a mut dyn ValueVisitor) -> Box<dyn Iterator<Item=Instruction> + 'a> {
        visitor.visit_shared_container(self.deref())
    }
}