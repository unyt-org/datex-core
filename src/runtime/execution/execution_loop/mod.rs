pub mod interrupts;
mod operations;
pub mod regular_instruction_execution;
pub mod state;
pub mod type_instruction_execution;

use crate::global::protocol_structures::instructions::Instruction;
use crate::parser::body::{DXBParserError, iterate_instructions};
use crate::runtime::execution::execution_loop::interrupts::{
    ExecutionInterrupt, ExternalExecutionInterrupt, InterruptProvider,
    InterruptResult,
};
use crate::runtime::execution::execution_loop::regular_instruction_execution::execute_regular_instruction;
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::macros::{next_iter, yield_unwrap};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::stdlib::rc::Rc;
use crate::traits::apply::Apply;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;

/// Main execution loop that drives the execution of the DXB body
/// The interrupt_provider is used to provide results for synchronous or asynchronous I/O operations
pub fn execution_loop(
    state: RuntimeExecutionState,
    dxb_body: Rc<RefCell<Vec<u8>>>,
    interrupt_provider: InterruptProvider,
) -> impl Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>> {
    gen move {
        let mut instruction_iterator = iterate_instructions(dxb_body);
        let mut slots = state.slots;

        let first_instruction = instruction_iterator.next();

        let mut active_value: Option<ValueContainer> = None;

        if let Some(Ok(Instruction::RegularInstruction(first_instruction))) =
            first_instruction
        {
            // execute the root instruction, which will drive further recursive execution
            let inner_iterator = execute_regular_instruction(
                interrupt_provider.clone(),
                first_instruction,
            );
            'main: for step in inner_iterator {
                let step = yield_unwrap!(step);

                match step {
                    // yield external steps directly to be handled by the caller
                    ExecutionInterrupt::External(external_step) => {
                        yield Ok(external_step);
                    }
                    // final execution result - loop ends here
                    ExecutionInterrupt::ValueReturn(value) => {
                        return yield Ok(ExternalExecutionInterrupt::Result(
                            value,
                        ));
                    }
                    // feed new instructions to execution as long as they are requested
                    ExecutionInterrupt::GetNextRegularInstruction => {
                        loop {
                            match next_iter!(instruction_iterator, 'main) {
                                // feed next regular instruction
                                Ok(Instruction::RegularInstruction(
                                    next_instruction,
                                )) => {
                                    interrupt_provider.provide_result(
                                        InterruptResult::NextRegularInstruction(
                                            next_instruction,
                                        ),
                                    );
                                }
                                // instruction is not a regular instruction - invalid program
                                Ok(_) => {
                                    yield Err(ExecutionError::InvalidProgram(
                                        InvalidProgramError::ExpectedRegularInstruction,
                                    ));
                                }
                                // instruction iterator ran out of instructions - must wait for more
                                Err(
                                    DXBParserError::ExpectingMoreInstructions,
                                ) => {
                                    yield Err(ExecutionError::IntermediateResultWithState(active_value.clone(), None));
                                    // assume that when continuing after this yield, more instructions will have been loaded
                                    // so we run the loop again to try to get the next instruction
                                    continue;
                                }
                                // other parsing errors from instruction iterator
                                Err(err) => {
                                    return yield Err(
                                        ExecutionError::DXBParserError(err),
                                    );
                                }
                            };
                            // only run this once per default
                            break;
                        }
                    }
                    ExecutionInterrupt::GetNextTypeInstruction => {
                        loop {
                            match next_iter!(instruction_iterator, 'main) {
                                // feed next type instruction
                                Ok(Instruction::TypeInstruction(
                                    next_instruction,
                                )) => {
                                    interrupt_provider.provide_result(
                                        InterruptResult::NextTypeInstruction(
                                            next_instruction,
                                        ),
                                    );
                                }
                                // instruction is not a type instruction - invalid program
                                Ok(_) => {
                                    yield Err(ExecutionError::InvalidProgram(
                                        InvalidProgramError::ExpectedTypeInstruction,
                                    ));
                                }
                                // instruction iterator ran out of instructions - must wait for more
                                Err(
                                    DXBParserError::ExpectingMoreInstructions,
                                ) => {
                                    yield Err(ExecutionError::IntermediateResultWithState(active_value.clone(), None));
                                    // assume that when continuing after this yield, more instructions will have been loaded
                                    // so we run the loop again to try to get the next instruction
                                    continue;
                                }
                                // other parsing errors from instruction iterator
                                Err(err) => {
                                    return yield Err(
                                        ExecutionError::DXBParserError(err),
                                    );
                                }
                            }
                            // only run this once per default
                            break;
                        }
                    }
                    ExecutionInterrupt::GetSlotValue(address) => {
                        // if address is >= 0xffffff00, resolve internal slot
                        if address >= 0xffffff00 {
                            yield Ok(ExternalExecutionInterrupt::GetInternalSlotValue(
                                address,
                            ));
                        }
                        // else handle normal slot
                        else {
                            let val =
                                yield_unwrap!(slots.get_slot_value(address));
                            interrupt_provider.provide_result(
                                InterruptResult::ResolvedValue(val),
                            );
                        }
                    }
                    ExecutionInterrupt::SetSlotValue(address, value) => {
                        yield_unwrap!(slots.set_slot_value(address, value));
                    }
                    ExecutionInterrupt::DropSlot(address) => {
                        yield_unwrap!(slots.drop_slot(address));
                    }
                    ExecutionInterrupt::AllocateSlot(address, value) => {
                        slots.allocate_slot(address, Some(value));
                    }
                    ExecutionInterrupt::SetActiveValue(value) => {
                        active_value = value;
                    }
                    // only for internal interrupts
                    ExecutionInterrupt::TypeReturn(_) => unreachable!(),
                    ExecutionInterrupt::KeyValuePairReturn(_) => unreachable!(),
                    ExecutionInterrupt::StatementsEnd(_) => unreachable!(),
                }
            }
        } else {
            // the first instruction must always be a regular instruction
            return yield Err(ExecutionError::InvalidProgram(
                InvalidProgramError::ExpectedRegularInstruction,
            ));
        }

        // TODO: should this be unreachable?
        // if execution exited without value return, return None
        yield Ok(ExternalExecutionInterrupt::Result(None))
    }
}

// TODO
fn handle_apply(
    callee: &ValueContainer,
    args: &[ValueContainer],
) -> Result<Option<ValueContainer>, ExecutionError> {
    // callee is guaranteed to be Some here
    // apply_single if one arg, apply otherwise
    Ok(if args.len() == 1 {
        callee.apply_single(&args[0])?
    } else {
        callee.apply(args)?
    })
}
