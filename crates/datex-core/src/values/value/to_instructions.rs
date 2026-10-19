use crate::{
    core_compiler::{
        to_instructions::{ToInstructions, yield_all},
        value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value::Value,
};

impl ToInstructions for Value {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            // append inner instructions
            match self {
                Value::Native(native) => {
                    yield_all!(native.to_bytecode_item(visitor));
                }
                Value::Core(core) => {
                    yield_all!(core.to_bytecode_item(visitor));
                }
            }
        })
    }
}
