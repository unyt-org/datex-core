use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value_container::borrowed_value_container::BorrowedValueContainer,
};

impl ToInstructions for BorrowedValueContainer<'_> {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        match self {
            BorrowedValueContainer::Local(value) => value.to_instructions(visitor),
            BorrowedValueContainer::Shared(shared_container) => shared_container.to_instructions(visitor),
        }
    }
}
