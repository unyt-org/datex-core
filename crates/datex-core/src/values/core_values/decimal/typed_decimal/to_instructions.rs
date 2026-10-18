use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, regular_instruction::RegularInstruction},
    prelude::*,
    values::core_values::decimal::typed_decimal::TypedDecimal,
};
impl ToInstructions for TypedDecimal {
    fn to_instructions<'a>(
        &'a self,
        _ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            yield match &self {
                TypedDecimal::F32(val) => {
                    RegularInstruction::decimal_f32(val.into_inner()).into()
                }
                TypedDecimal::F64(val) => {
                    RegularInstruction::decimal_f64(val.into_inner()).into()
                }
                TypedDecimal::Decimal(val) => {
                    RegularInstruction::decimal_big(val.clone()).into()
                }
            }
        })
    }
}
