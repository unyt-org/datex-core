use std::cell::RefCell;
use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::core_values::boolean::Boolean,
};
use crate::core_compiler::shared_value_tracking::SharedValueTracking;

impl ToInstructions for Boolean {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        self.0.to_instructions(visitor)
    }
}
