use crate::{
    core_compiler::into_regular_instruction::{
        ToRegularInstruction,
    },
    instruction::regular_instruction::RegularInstruction,
};

impl ToRegularInstruction for f32 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::decimal_f32(*self)
    }
}

impl ToRegularInstruction for f64 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::decimal_f64(*self)
    }
}
