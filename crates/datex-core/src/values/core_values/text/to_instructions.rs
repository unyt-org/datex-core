use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::core_values::text::Text,
};

impl ToInstructions for Text {
    fn to_bytecode_item<'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        // FIXME: more efficient way to do this without collecting?
        let instructions =
            self.as_string().to_bytecode_item(ctx).collect::<Vec<_>>();
        Box::new(instructions.into_iter())
    }
}
