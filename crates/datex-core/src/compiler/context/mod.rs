pub mod value_visitor;

use crate::{
    compiler::scope::CompilationScope,
    core_compiler::{
        core_compilation_context::{
            CompileInput, CoreCompilationContext, DXBWithSharedValues,
        },
        to_instructions::ToInstructions,
    },
    prelude::*,
    runtime::execution::context::ExecutionMode,
    values::value_container::ValueContainer,
};
use binrw::{BinWrite, io::Cursor, meta::WriteEndian};
use crate::compiler::ast_node_to_instructions::AstNodeToInstructions;
use crate::compiler::error::CompilerError;
use crate::compiler::precompiler::precompiled_ast::{AstMetadata, RichAst};

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

/// result of a compilation, including the compiled DXB and the scope of the compile context
#[derive(Debug)]
pub struct CompilationResult {
    pub dxb_with_shared_values: DXBWithSharedValues,
    pub scope: CompilationScope,
    pub has_non_static_value: bool,
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
    pub fn compile_to_dxb_with_shared_values(self, input: &impl ToInstructions) -> DXBWithSharedValues {
        self.core_context.compile_to_dxb_with_shared_values(input)
    }

    /// Compiles an AST node that implements [AstNodeToInstructions] with shared value tracking and appends a preamble with the injected shared values.
    pub fn compile_ast_node_to_dxb_with_shared_values(
        mut self,
        ast_node: &impl AstNodeToInstructions,
        metadata: &mut AstMetadata,
    ) -> Result<CompilationResult, CompilerError> {
        let dxb = ast_node.try_compile_to_dxb(
            &mut self,
            metadata,
        )?;
        Ok(CompilationResult {
            dxb_with_shared_values: self.core_context.append_preamble_to_dxb(dxb),
            scope: self.scope,
            has_non_static_value: self.has_non_static_value,
        })
    }

    /// Compiles a [RichAst] with shared value tracking and appends a preamble with the injected shared values.
    pub fn compile_ast_with_metadata_to_dxb_with_shared_values(
        self,
        rich_ast: RichAst,
    ) -> Result<CompilationResult, CompilerError> {
        self.compile_ast_node_to_dxb_with_shared_values(
            &*rich_ast.ast.data,
            &mut *rich_ast.metadata.borrow_mut(),
        )
    }

    pub fn core_context(&mut self) -> &mut CoreCompilationContext<'a> {
        &mut self.core_context
    }
    

    pub fn mark_has_non_static_value(&mut self) {
        self.has_non_static_value = true;
    }
}
