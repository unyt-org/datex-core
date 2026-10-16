use std::cell::RefCell;
use crate::{
    ast::expressions::DatexExpression,
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
};
use crate::core_compiler::shared_value_tracking::SharedValueTracking;

impl ToInstructions for DatexExpression {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(self.data().to_instructions(visitor))
    }
}
