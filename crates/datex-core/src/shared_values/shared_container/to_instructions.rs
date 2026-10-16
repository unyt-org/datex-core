use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction},
    prelude::*,
    shared_values::{
        SharedContainer,
    },
};

impl ToInstructions for SharedContainer {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        visitor.visit_shared_container(self)
    }
}
