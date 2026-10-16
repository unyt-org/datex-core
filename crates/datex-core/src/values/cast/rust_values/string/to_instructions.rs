use crate::{
    core_compiler::into_regular_instruction::{
        ToRegularInstruction,
    },
    instruction::regular_instruction::RegularInstruction,
};
use alloc::string::String;

impl ToRegularInstruction for String {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::text(self.clone())
    }
}