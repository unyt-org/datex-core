use binrw::BinWrite;
use crate::{
    instruction::Instruction, shared_values::SharedContainer,
};
use crate::core_compiler::core_compilation_context::ByteCursor;
use crate::core_compiler::to_instructions::ToInstructions;
use crate::prelude::*;

pub trait ValueVisitor {
    fn visit_shared_container<'a>(
        &'a mut self,
        value_container: &'a SharedContainer,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>;
}

/// Compiles a [ToInstructions] value using the visitor and returns the final byte buffer.
pub fn compile_instructions_to_dxb(
    visitor: &mut impl ValueVisitor,
    input: &impl ToInstructions
) -> Vec<u8> {
    let mut buffer = ByteCursor::new(Vec::new());
    for instruction in input.to_instructions(visitor) {
        instruction.write(&mut buffer).unwrap();
    }
    buffer.into_inner()
}

/// Compiles an iterator of [Instruction]s and returns the final byte buffer.
pub fn compile_instruction_iterator_to_dxb(
    iterator: impl Iterator<Item = Instruction>,
) -> Vec<u8> {
    let mut buffer = ByteCursor::new(Vec::new());
    for instruction in iterator {
        instruction.write(&mut buffer).unwrap();
    }
    buffer.into_inner()
}