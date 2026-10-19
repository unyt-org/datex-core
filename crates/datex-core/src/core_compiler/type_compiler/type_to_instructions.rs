use crate::{
    core_compiler::{
        to_instructions::{ToInstructions, yield_all},
        value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    types::r#type::Type,
};

impl ToInstructions for Type {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            match self {
                Type::Entity(entity_type) => todo!(),
                Type::Definition(def) => {
                    yield_all!(def.to_bytecode_item(visitor));
                }
            }
        })
    }
}
