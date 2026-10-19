use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value_container::ValueContainer,
};

impl ToInstructions for ValueContainer {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        match self {
            ValueContainer::Local(value) => value.to_bytecode_item(visitor),
            ValueContainer::Shared(shared) => {
                visitor.visit_shared_container(shared)
            }
        }
    }
}
