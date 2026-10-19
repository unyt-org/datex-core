use crate::{
    ast::type_expressions::{TypeExpression, TypeExpressionData},
    compiler::{
        ast_node_to_instructions::AstNodeToInstructions,
        context::CompilationContext, error::CompilerError,
        metadata::CompileContextMetadata,
        precompiler::precompiled_ast::AstMetadata,
    },
    core_compiler::{
        to_instructions::{ToInstructions, yield_all, yield_all_ok},
        value_visitor::ValueVisitor,
    },
    instruction::{
        BytecodeItem, Instruction, type_instruction::TypeInstruction,
    },
    prelude::*,
    types::literal_type_definition::LiteralTypeDefinition,
};
use core::ops::Deref;

impl AstNodeToInstructions for TypeExpression {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<BytecodeItem, CompilerError>> + 'a>
    {
        Box::new(gen move {
            match self.data() {
                TypeExpressionData::Integer(integer) => {
                    yield Ok(TypeInstruction::Literal(
                        LiteralTypeDefinition::Integer(integer.clone()),
                    )
                    .into())
                }
                TypeExpressionData::Text(text) => {
                    yield Ok(TypeInstruction::Literal(
                        LiteralTypeDefinition::Text(text.clone()),
                    )
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
                    yield_all!(
                        range.start.deref().ast_node_to_instructions_inner(
                            compilation_context,
                            ast_metadata
                        )
                    );
                    yield_all!(
                        range.end.deref().ast_node_to_instructions_inner(
                            compilation_context,
                            ast_metadata
                        )
                    );
                }
                e => todo!(
                    "Type expression to instruction not implemented for {:?}",
                    e
                ),
            }
        })
    }
}
