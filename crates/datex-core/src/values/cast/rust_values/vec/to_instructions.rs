use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::native::DatexNativeBase,
};

impl<K> ToInstructions for Vec<K>
where
    K: DatexNativeBase,
{
    fn to_instructions<'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::list(self.len() as u32).into();
            for item in self.iter() {
                for instruction in item.to_instructions(ctx) {
                    yield instruction;
                }
            }
        })
    }
}
