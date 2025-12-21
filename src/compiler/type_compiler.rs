use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::precompiler::precompiled_ast::AstMetadata;
use crate::compiler::scope::CompilationScope;
use crate::core_compiler::value_compiler::{
    append_big_integer, append_instruction_code,
};
use crate::global::instruction_codes::InstructionCode;
use crate::global::type_instruction_codes::TypeInstructionCode;
use crate::stdlib::rc::Rc;
use crate::utils::buffers::{append_u8, append_u32};
use crate::values::core_values::integer::Integer;
use core::cell::RefCell;
use num_bigint::BigInt;

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

    pub fn insert_type_literal_text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let len = bytes.len();

        if len < 256 {
            self.append_type_instruction_code(
                TypeInstructionCode::TYPE_LITERAL_SHORT_TEXT,
            );
            append_u8(&mut self.buffer, len as u8);
        } else {
            self.append_type_instruction_code(
                TypeInstructionCode::TYPE_LITERAL_TEXT,
            );
            append_u32(&mut self.buffer, len as u32);
        }

        self.buffer.extend_from_slice(bytes);
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
        TypeExpressionData::Text(text) => {
            ctx.insert_type_literal_text(text);
        }
        _ => core::todo!("#453 Undescribed by author."),
    }
    Ok(scope)
}
