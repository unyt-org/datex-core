pub mod value_visitor;

use crate::{
    ast::type_expressions::TypeExpression,
    compiler::scope::CompilationScope,
    core_compiler::{
        buffer_provider::BufferProvider,
        core_compilation_context::{
            CompileInput, CoreCompilationContext, DXBWithSharedValues,
        },
        to_instructions::ToInstructions,
        value_compiler::{append_instruction, append_instruction_code},
    },
    global::stack_index::StackIndex,
    instruction::{Instruction, instruction_codes::InstructionCode},
    prelude::*,
    runtime::execution::context::ExecutionMode,
    utils::buffers::append_u32,
    values::value_container::ValueContainer,
};
use binrw::{BinWrite, io::Cursor, meta::WriteEndian};
use crate::compiler::ast_node_to_instructions::AstNodeToInstructions;
use crate::compiler::error::CompilerError;
use crate::compiler::precompiler::precompiled_ast::AstMetadata;

/// compilation context, created for each compiler call, even if compiling a script for the same scope
pub struct CompilationContext<'a> {
    pub core_context: CoreCompilationContext<'a>,
    pub inserted_value_index: usize,
    pub inserted_values: Vec<Option<ValueContainer>>,
    /// this flag is set to true if any non-static value is encountered
    pub has_non_static_value: bool,
    pub execution_mode: ExecutionMode,
    pub scope: CompilationScope,
}

impl<'a> CompilationContext<'a> {
    const INT_32_BYTES: u8 = 4;

    pub fn new(
        inserted_values: Vec<Option<ValueContainer>>,
        execution_mode: ExecutionMode,
        input: CompileInput<'a>,
        scope: CompilationScope,
    ) -> Self {
        CompilationContext {
            inserted_value_index: 0,
            core_context: CoreCompilationContext::new(input),
            inserted_values,
            has_non_static_value: false,
            execution_mode, // FIXME: execution mode is already in scope, is it necessary to have it here as well?
            scope,
        }
    }

    /// Compiles a [ToInstructions] value with shared value tracking and appends a preamble with the injected shared values.
    /// Returns the final byte buffer and the list of shared values that were moved or referenced during compilation
    pub fn compile_to_dxb_with_shared_values(self, input: impl ToInstructions) -> DXBWithSharedValues {
        self.core_context.compile_to_dxb_with_shared_values(input)
    }

    /// Compiles an AST node that implements [AstNodeToInstructions] with shared value tracking and appends a preamble with the injected shared values.
    pub fn compile_ast_node_to_dxb_with_shared_values(
        mut self,
        ast_node: &impl AstNodeToInstructions,
        metadata: &mut AstMetadata,
    ) -> Result<(DXBWithSharedValues, CompilationScope), CompilerError> {
        let dxb = ast_node.try_compile_to_dxb(
            &mut self,
            metadata,
        )?;
        Ok((self.core_context.append_preamble_to_dxb(dxb), self.scope))
    }

    pub fn core_context(&mut self) -> &mut CoreCompilationContext<'a> {
        &mut self.core_context
    }

    pub fn insert_stack_index(&mut self, stack_index: StackIndex) {
        append_u32(self.cursor(), stack_index.0);
    }

    pub fn set_u32_at_index(&mut self, u32: u32, index: usize) {
        let buf = self.cursor().get_mut();
        buf[index..index + CompilationContext::INT_32_BYTES as usize]
            .copy_from_slice(&u32.to_le_bytes());
    }

    pub fn mark_has_non_static_value(&mut self) {
        self.has_non_static_value = true;
    }

    pub fn write<T: BinWrite + WriteEndian>(&mut self, value: T)
    where
        for<'b> <T as binrw::BinWrite>::Args<'b>: core::default::Default,
    {
        self.core_context.write(value);
    }

    #[deprecated(note = "use write() instead")]
    pub fn append_instruction_code(&mut self, code: InstructionCode) {
        append_instruction_code(self.cursor(), code);
    }
}
