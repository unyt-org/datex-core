use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    types::entities::entity_type_definition::EntityTypeDefinition,
};

impl ToInstructions for EntityTypeDefinition {
    fn to_instructions<'a>(
        &'a self,
        _visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            todo!("Implement ToInstructions for EntityTypeDefinition")
        })
    }
}
