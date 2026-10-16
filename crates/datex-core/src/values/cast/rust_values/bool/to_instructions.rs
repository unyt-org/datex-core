use crate::{
    instruction::{Instruction, regular_instruction::RegularInstruction},
};
use crate::core_compiler::into_regular_instruction::ToRegularInstruction;

impl ToRegularInstruction for bool {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::boolean(*self)
    }
}