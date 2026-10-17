use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::map::Map,
};
use crate::core_compiler::to_instructions::{yield_all, yield_all_collected};

impl ToInstructions for Map {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield RegularInstruction::map(self.size() as u32).into();

            for (key, value) in self.iter() {

                match key.try_as::<str>() {
                    // text -> insert key string
                    Ok(string) => {
                        if string.len() < 256 {
                            yield RegularInstruction::key_value_short_text(
                                string.to_string(),
                            )
                                .into();
                        } else {
                            yield RegularInstruction::key_value_dynamic()
                                .into();
                            yield RegularInstruction::text(string.to_string())
                                .into();
                        }
                    }
                    // other -> insert key as dynamic
                    Err(key) => {
                        yield RegularInstruction::key_value_dynamic().into();
                        yield_all_collected!(key.to_instructions(visitor));
                    }
                }
                
                yield_all!(value.to_instructions(visitor));
            }
        })
    }
}
