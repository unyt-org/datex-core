use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    types::r#type::Type,
};
use crate::core_compiler::to_instructions::yield_all;

impl ToInstructions for Type {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            match self {
                Type::Entity(entity_type) => todo!(),
                Type::Definition(def) => {
                    yield_all!(def.to_instructions(visitor));
                }
            }
        })
    }
}
