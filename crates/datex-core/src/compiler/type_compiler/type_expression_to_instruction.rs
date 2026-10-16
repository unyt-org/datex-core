use core::ops::Deref;
use crate::{
    ast::type_expressions::{TypeExpression, TypeExpressionData},
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{Instruction, type_instruction::TypeInstruction},
    prelude::*,
    types::literal_type_definition::LiteralTypeDefinition,
};
use crate::core_compiler::to_instructions::yield_all;

impl ToInstructions for TypeExpression {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            match self.data() {
                TypeExpressionData::Integer(integer) => {
                    yield TypeInstruction::Literal(
                        LiteralTypeDefinition::Integer(integer.clone()),
                    )
                    .into()
                }
                TypeExpressionData::Text(text) => {
                    yield TypeInstruction::Literal(LiteralTypeDefinition::Text(
                        text.clone(),
                    ))
                    .into()
                }
                TypeExpressionData::Boolean(boolean) => {
                    yield TypeInstruction::Literal(
                        LiteralTypeDefinition::Boolean(boolean.clone()),
                    )
                    .into()
                }
                TypeExpressionData::GetCoreLibType(core_lib_id) => {
                    yield TypeInstruction::CoreType(*core_lib_id).into()
                }
                TypeExpressionData::Range(range) => {
                    yield TypeInstruction::Range.into();
                    yield_all!(range.start.deref().to_instructions(visitor));
                    yield_all!(range.end.deref().to_instructions(visitor));
                }
                e => todo!(
                    "Type expression to instruction not implemented for {:?}",
                    e
                ),
            }
        })
    }
}
