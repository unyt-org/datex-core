use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::scope::CompilationScope;
use crate::global::type_instruction_codes::TypeSpaceInstructionCode;
use crate::values::core_values::integer::Integer;
use core::cell::RefCell;
use crate::compiler::precompiler::precompiled_ast::AstMetadata;
use crate::core_compiler::value_compiler::append_big_integer;
use crate::stdlib::rc::Rc;
use crate::utils::buffers::append_u8;

/// Compilation functions for type expressions.
impl CompilationContext {
    pub fn append_type_instruction_code(&self, code: TypeSpaceInstructionCode) {
        append_u8(self.buffer.borrow_mut().as_mut(), code as u8);
    }

    // TODO #452: Handle other types

    pub fn insert_type_literal_integer(&self, integer: &Integer) {
        self.append_type_instruction_code(
            TypeSpaceInstructionCode::TYPE_LITERAL_INTEGER,
        );
        append_big_integer(self.buffer.borrow_mut().as_mut(), integer);
    }
}

pub fn compile_type_expression(
    ctx: &CompilationContext,
    expr: &TypeExpression,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    match &expr.data {
        TypeExpressionData::Integer(integer) => {
            ctx.insert_type_literal_integer(integer);
        }
        _ => todo!("#453 Undescribed by author."),
    }
    Ok(scope)
}
