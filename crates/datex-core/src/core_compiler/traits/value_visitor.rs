use crate::{
    core_compiler::{
        core_compilation_context::ByteCursor, to_instructions::ToInstructions,
    },
    instruction::{BytecodeItem, Instruction},
    prelude::*,
    shared_values::SharedContainer,
};
use binrw::BinWrite;

pub trait ValueVisitor {
    fn visit_shared_container<'a>(
        &'a mut self,
        value_container: &'a SharedContainer,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a>;
}

/// Compiles a [ToInstructions] value using the visitor and returns the final byte buffer.
pub fn compile_instructions_to_dxb(
    visitor: &mut impl ValueVisitor,
    input: &impl ToInstructions,
) -> Vec<u8> {
    let mut buffer = ByteCursor::new(Vec::new());
    for instruction in input.to_bytecode_item(visitor) {
        match &instruction {
            BytecodeItem::Instruction(item) => {
                item.write(&mut buffer).unwrap();
            }
            _ => {
                unreachable!()
            }
        }
    }
    buffer.into_inner()
}

/// Compiles an iterator of [Instruction]s and returns the final byte buffer.
pub fn compile_instruction_iterator_to_dxb(
    iterator: impl Iterator<Item = BytecodeItem>,
) -> Vec<u8> {
    let mut buffer = ByteCursor::new(Vec::new());
    for instruction in iterator {
        match &instruction {
            BytecodeItem::Instruction(item) => {
                item.write(&mut buffer).unwrap();
            }
            _ => {
                unreachable!()
            }
        }
    }
    buffer.into_inner()
}
