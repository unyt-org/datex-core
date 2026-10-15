use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    values::core_values::text::Text,
};

impl ToInstructions for Text {
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        // FIXME: more efficient way to do this without collecting?
        let instructions = self.to_string().to_instructions(ctx).collect::<Vec<_>>();
        Box::new(instructions.into_iter())
    }
}
