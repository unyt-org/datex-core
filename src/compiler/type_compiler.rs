use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::precompiler::precompiled_ast::AstMetadata;
use crate::compiler::scope::CompilationScope;
use crate::core_compiler::value_compiler::append_big_integer;
use crate::global::type_instruction_codes::TypeInstructionCode;
use crate::stdlib::rc::Rc;
use crate::utils::buffers::append_u8;
use crate::values::core_values::integer::Integer;
use core::cell::RefCell;

/// Compilation functions for type expressions.
impl CompilationContext {
    pub fn append_type_instruction_code(&mut self, code: TypeInstructionCode) {
        append_u8(&mut self.buffer, code as u8);
    }

    // TODO #452: Handle other types

    pub fn insert_type_literal_integer(&mut self, integer: &Integer) {
        self.append_type_instruction_code(
            TypeInstructionCode::TYPE_LITERAL_INTEGER,
        );
        append_big_integer(&mut self.buffer, integer);
    }
}

pub fn compile_type_expression(
    ctx: &mut CompilationContext,
    expr: &TypeExpression,
    ast_metadata: Rc<RefCell<AstMetadata>>,
    scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    match &expr.data {
        TypeExpressionData::Integer(integer) => {
            ctx.insert_type_literal_integer(integer);
        }
        _ => core::todo!("#453 Undescribed by author."),
    }
    Ok(scope)
}
