use crate::{
    compiler::{
        context::CompilationContext,
        error::CompilerError,
        metadata::CompileContextMetadata,
        precompiler::precompiled_ast::{AstMetadata, RichAst},
        scope::CompilationScope,
    },
    core_compiler::core_compilation_context::ByteCursor,
    instruction::{
        BytecodeItem, Instruction, Label, instruction_data::JumpData,
        regular_instruction::RegularInstruction,
    },
    prelude::*,
};
use binrw::BinWrite;

pub trait AstNodeToInstructions {
    fn ast_node_to_instructions<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
        ctx_metadata: CompileContextMetadata,
    ) -> Box<dyn Iterator<Item = Result<BytecodeItem, CompilerError>> + 'a>;

    /// Calls [ast_node_to_instructions] with default [CompileContextMetadata].
    /// This should be called on child nodes, but not on the root node
    fn ast_node_to_instructions_inner<'a>(
        &'a self,
        compilation_context: &'a mut CompilationContext,
        ast_metadata: &'a mut AstMetadata,
    ) -> Box<dyn Iterator<Item = Result<BytecodeItem, CompilerError>> + 'a>
    {
        self.ast_node_to_instructions(
            compilation_context,
            ast_metadata,
            CompileContextMetadata::default(),
        )
    }

    /// Compiles a given AST node to a DXB byte buffer, using the provided compilation context, metadata, and scope.
    fn try_compile_to_dxb(
        &self,
        compilation_context: &mut CompilationContext,
        ast_metadata: &mut AstMetadata,
    ) -> Result<Vec<u8>, CompilerError> {
        let mut buffer = ByteCursor::new(Vec::new());
        let mut labels: HashMap<Label, usize> = HashMap::new();

        // byte position, offset base label, target label
        let mut fixes: Vec<(usize, Label, Label)> = Vec::new();

        for item in self.ast_node_to_instructions(
            compilation_context,
            ast_metadata,
            CompileContextMetadata::outer(),
        ) {
            match item? {
                BytecodeItem::Instruction(instruction) => {
                    instruction.write(&mut buffer)?;
                }
                BytecodeItem::Label(label) => {
                    labels.insert(label, buffer.position() as usize);
                }
                BytecodeItem::Jump { target } => {
                    let instruction = Instruction::Regular(
                        RegularInstruction::Jump(JumpData { offset: 0 }),
                    );
                    let start = buffer.position() as usize;
                    instruction.write(&mut buffer)?;
                    fixes.push((start + 1, Label(start), target));
                }

                BytecodeItem::JumpIfFalse {
                    target,
                    condition_end,
                } => {
                    let instruction = Instruction::Regular(
                        RegularInstruction::JumpIfFalse(JumpData { offset: 0 }),
                    );
                    let start = buffer.position() as usize;
                    instruction.write(&mut buffer)?;
                    fixes.push((start + 1, condition_end, target));
                }
            }
        }

        let mut bytes = buffer.into_inner();

        for (operand_pos, base_label, target_label) in fixes {
            let base = match base_label {
                Label(_) if !labels.contains_key(&base_label) => {
                    operand_pos + 4 // FIXME 32 vs 64 arch??
                }
                _ => *labels
                    .get(&base_label)
                    .ok_or_else(|| CompilerError::InvalidJump)?,
            };

            let target = *labels
                .get(&target_label)
                .ok_or_else(|| CompilerError::InvalidJump)?;

            let offset = i32::try_from(target as i64 - base as i64)
                .map_err(|_| CompilerError::InvalidJump)?;

            bytes[operand_pos..operand_pos + 4]
                .copy_from_slice(&offset.to_le_bytes());
        }

        Ok(bytes)
    }
}
