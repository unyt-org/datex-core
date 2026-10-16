use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value_container::ValueContainer,
};
use crate::core_compiler::shared_value_tracking::SharedValueTracking;

impl ToInstructions for ValueContainer {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        match self {
            ValueContainer::Local(value) => value.to_instructions(visitor),
            ValueContainer::Shared(shared) => visitor.visit_shared_container(shared)
        }
    }
}
