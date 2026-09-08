use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
    preludes::derive::RegularInstruction,
};

impl<V> ToInstructions for Box<V>
where
    V: ToInstructions + ?Sized,
{
    fn to_instructions<'ctx, 'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor<'ctx>,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    where
        'ctx: 'a,
    {
        Box::new(gen move {
            yield RegularInstruction::boxed_value().into();
            for instruction in self.as_ref().to_instructions(ctx) {
                yield instruction;
            }
        })
    }
}
