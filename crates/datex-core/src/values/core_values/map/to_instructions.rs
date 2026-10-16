use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::map::Map,
};
impl ToInstructions for Map {
    fn to_instructions<'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::map(self.size() as u32).into();

            for (key, value) in self.iter() {
                let key_instructions =
                    key.to_instructions(ctx).collect::<Vec<_>>();
                for instr in key_instructions {
                    yield instr;
                }
                for instr in value.to_instructions(ctx) {
                    yield instr;
                }
            }
        })
    }
}
