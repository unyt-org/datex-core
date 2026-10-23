use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    prelude::*,
};
use crate::core_compiler::to_instructions::{yield_all, yield_all_collected};
use crate::instruction::regular_instruction::RegularInstruction;

impl<K, V, S> ToInstructions for indexmap::IndexMap<K, V, S>
where
    K: ToInstructions,
    V: ToInstructions,
{
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield RegularInstruction::map(self.len() as u32).into();

            for (key, value) in self.iter() {
                yield RegularInstruction::key_value_dynamic().into();
                yield_all_collected!(key.to_bytecode_item(visitor));
                yield_all!(value.to_bytecode_item(visitor));
            }
        })
    }
}
