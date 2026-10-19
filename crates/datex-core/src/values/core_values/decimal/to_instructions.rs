use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::decimal::Decimal,
};

impl ToInstructions for Decimal {
    fn to_bytecode_item<'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            match &self {
                Decimal::Finite(big_decimal) if big_decimal.is_integer() => {
                    if let Some(int) = big_decimal.to_i16() {
                        yield RegularInstruction::decimal_as_int16(int).into();
                    } else if let Some(int) = big_decimal.to_i32() {
                        yield RegularInstruction::decimal_as_int32(int).into();
                    } else {
                        yield RegularInstruction::decimal(self.clone()).into();
                    }
                }
                _ => {
                    yield RegularInstruction::decimal(self.clone()).into();
                }
            }
        })
    }
}
