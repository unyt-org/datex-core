use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::scope::CompilationScope;
use crate::global::type_instruction_codes::TypeSpaceInstructionCode;
use crate::values::core_values::integer::Integer;
use datex_core::compiler::precompiler::AstMetadata;
use std::cell::RefCell;
use std::rc::Rc;
use crate::ast::tree::TypeExpression;

/// Compilation functions for type expressions.
impl CompilationContext {
    pub fn append_type_instruction_code(&self, code: TypeSpaceInstructionCode) {
        self.append_u8(code as u8);
    }

    // TODO #452: Handle other types

    pub fn insert_type_literal_integer(&self, integer: &Integer) {
        self.append_type_instruction_code(
            TypeSpaceInstructionCode::TYPE_LITERAL_INTEGER,
        );
        self.insert_big_integer(integer);
    }
}

pub fn compile_type_expression(
    ctx: &CompilationContext,
    expr: &TypeExpression,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    match expr {
        TypeExpression::Integer(integer) => {
            ctx.insert_type_literal_integer(integer);
        }
        _ => todo!("#453 Undescribed by author."),
    }
    Ok(scope)
}
