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
use crate::compiler::ast_node_to_instructions::AstNodeToInstructions;
use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::metadata::CompileContextMetadata;
use crate::compiler::precompiler::precompiled_ast::AstMetadata;
use crate::core_compiler::to_instructions::{yield_all, yield_all_ok};

impl AstNodeToInstructions for TypeExpression {
    fn ast_node_to_instructions<'a>(
        &'a self, 
        compilation_context: &'a mut CompilationContext, 
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a> {
        Box::new(gen move {
            match self.data() {
                TypeExpressionData::Integer(integer) => {
                    yield Ok(TypeInstruction::Literal(
                        LiteralTypeDefinition::Integer(integer.clone()),
                    )
                    .into())
                }
                TypeExpressionData::Text(text) => {
                    yield Ok(TypeInstruction::Literal(LiteralTypeDefinition::Text(
                        text.clone(),
                    ))
                    .into())
                }
                TypeExpressionData::Boolean(boolean) => {
                    yield Ok(TypeInstruction::Literal(
                        LiteralTypeDefinition::Boolean(boolean.clone()),
                    )
                    .into())
                }
                TypeExpressionData::GetCoreLibType(core_lib_id) => {
                    yield Ok(TypeInstruction::CoreType(*core_lib_id).into())
                }
                TypeExpressionData::Range(range) => {
                    yield Ok(TypeInstruction::Range.into());
                    yield_all!(range.start.deref().ast_node_to_instructions_inner(compilation_context, ast_metadata));
                    yield_all!(range.end.deref().ast_node_to_instructions_inner(compilation_context, ast_metadata));
                }
                e => todo!(
                    "Type expression to instruction not implemented for {:?}",
                    e
                ),
            }
        })
    }
}
