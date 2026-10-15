use crate::{
    core_compiler::into_regular_instruction::{
        IntoRegularInstruction, impl_regular_to_instructions,
    },
    instruction::regular_instruction::RegularInstruction,
};
use alloc::string::String;

impl IntoRegularInstruction for String {
    fn into_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::text(self.clone())
    }
}
impl_regular_to_instructions!(String);