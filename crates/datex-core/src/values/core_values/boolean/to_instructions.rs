use crate::{
    core_compiler::{
        shared_value_tracking::SharedValueTracking,
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::core_values::boolean::Boolean,
};

impl ToInstructions for Boolean {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        self.0.to_bytecode_item(visitor)
    }
}
