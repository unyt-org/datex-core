use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value::Value,
};
use crate::core_compiler::to_instructions::yield_all;

impl ToInstructions for Value {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            // append inner instructions
            match self {
                Value::Native(native) => {
                    yield_all!(native.to_instructions(visitor));
                }
                Value::Core(core) => {
                    yield_all!(core.to_instructions(visitor));
                }
            }
        })
    }
}
