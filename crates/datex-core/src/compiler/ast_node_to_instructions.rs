use binrw::BinWrite;
use crate::{
    instruction::Instruction,
    prelude::*,
};
use crate::compiler::context::CompilationContext;
use crate::compiler::error::CompilerError;
use crate::compiler::metadata::CompileContextMetadata;
use crate::compiler::precompiler::precompiled_ast::{AstMetadata, RichAst};
use crate::compiler::scope::CompilationScope;
use crate::core_compiler::core_compilation_context::ByteCursor;

pub trait AstNodeToInstructions {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a>;

    /// Calls [ast_node_to_instructions] with default [CompileContextMetadata].
    /// This should be called on child nodes, but not on the root node
    fn ast_node_to_instructions_inner<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
    ) -> Box<dyn Iterator<Item=Result<Instruction, CompilerError>> + 'a> {
        self.ast_node_to_instructions(compilation_context, ast_metadata, CompileContextMetadata::default())
    }

    /// Compiles a given AST node to a DXB byte buffer, using the provided compilation context, metadata, and scope.
    fn try_compile_to_dxb(
        &self,
        compilation_context: &mut CompilationContext,
        ast_metadata: &mut AstMetadata,
    ) -> Result<Vec<u8>, CompilerError> {
        let mut buffer = ByteCursor::new(Vec::new());
        for instruction in self.ast_node_to_instructions(
            compilation_context,
            ast_metadata,
            CompileContextMetadata::outer(),
        ) {
            instruction?.write(&mut buffer)?;
        }
        Ok(buffer.into_inner())
    }
}