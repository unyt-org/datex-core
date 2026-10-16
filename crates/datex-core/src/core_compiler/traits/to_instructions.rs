use crate::{
    core_compiler::{
        value_visitor::ValueVisitor,
    },
    instruction::Instruction,
    prelude::*,
};


pub trait ToInstructions {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>;
}

/// A macro to yield instructions from a value that implements `ToInstructions`.
/// This expands to a loop that iterates over the instructions produced by the `to_instructions` method of the value, yielding each instruction to the caller.
pub macro yield_all($iterator:expr) {
    for instruction in $iterator {
        yield instruction;
    }
}

/// A macro to yield `Ok` instructions from a value that implements `ToInstructions`.
/// This expands to a loop that iterates over the instructions produced by the `to_instructions` method of the value, yielding each instruction wrapped in `Ok` to the caller.
pub macro yield_all_ok($iterator:expr) {
    for instruction in $iterator {
        yield Ok(instruction);
    }
}