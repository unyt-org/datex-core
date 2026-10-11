use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::value::Value,
};

impl ToInstructions for Value {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            // append inner instructions
            match self {
                Value::Native(native) => {
                    for instruction in native.to_instructions(ctx) {
                        yield instruction;
                    }
                }
                Value::Core(core) => {
                    for instruction in core.to_instructions(ctx) {
                        yield instruction;
                    }
                }
            }
        })
    }
}
