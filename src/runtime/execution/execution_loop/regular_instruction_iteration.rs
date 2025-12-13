use core::cell::RefCell;
use datex_core::global::protocol_structures::instructions::RegularInstruction;
use crate::stdlib::rc::Rc;
use crate::runtime::execution::execution_loop::{ExecutionStep, InterruptProvider};
use crate::runtime::execution::ExecutionError;

pub(crate) fn next_regular_instruction_iteration(
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    instruction: RegularInstruction,
) -> Box<impl Iterator<Item = Result<ExecutionStep, ExecutionError>>> {
    Box::new(gen move {
        match instruction {
            _ => todo!("..."),
        }
    })
}