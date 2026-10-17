use crate::core_compiler::to_instructions::ToInstructions;
use crate::core_compiler::value_visitor::ValueVisitor;
use crate::instruction::Instruction;
use crate::types::type_definition_with_metadata::TypeMetadata;
use crate::prelude::*;

impl ToInstructions for TypeMetadata {
    fn to_instructions<'a>(&'a self, visitor: &'a mut dyn ValueVisitor) -> Box<dyn Iterator<Item=Instruction> + 'a> {
        todo!()
    }
}