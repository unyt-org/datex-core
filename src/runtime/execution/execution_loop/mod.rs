mod operations;
pub mod regular_instruction_execution;
pub mod state;
pub mod type_instruction_execution;

use core::cell::RefCell;
use crate::stdlib::rc::Rc;
use log::info;
use datex_core::parser::next_instructions_stack::NextInstructionsStack;
use datex_core::runtime::execution::execution_loop::regular_instruction_execution::execute_regular_instruction;
use datex_core::runtime::execution::execution_loop::state::ExecutionLoopState;
use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator, BitwiseUnaryOperator, ComparisonOperator, LogicalUnaryOperator, ReferenceUnaryOperator, UnaryOperator};
use crate::global::operators::binary::{ArithmeticOperator, BitwiseOperator, LogicalOperator};
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, RegularInstruction, IntegerData, RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress, RawPointerAddress, ShortTextData, SlotAddress, TextData, TypeInstruction, Instruction};
use crate::parser::body;
use crate::parser::body::{iterate_instructions, DXBParserError};
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::{ExecutionError, ExecutionInput, InvalidProgramError};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::macros::{next_iter, yield_unwrap};
use crate::traits::apply::Apply;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::types::definition::TypeDefinition;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::list::List;
use crate::values::core_values::map::{Map, OwnedMapKey};
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

#[derive(Debug)]
pub enum ExecutionInterrupt {
    /// contains an optional ValueContainer that is intercepted by the consumer of a value or passed as the final result at the end of execution
    ValueReturn(Option<ValueContainer>),
    /// contains a Type that is intercepted by a consumer of a type value
    TypeReturn(Type),
    /// contains a key-value pair that is intercepted by a map construction operation
    KeyValuePairReturn((OwnedMapKey, ValueContainer)),
    /// indicates the end of an unbounded statements block - is intercepted by a statements block loop
    StatementsEnd,
    AllocateSlot(u32, ValueContainer),
    GetSlotValue(u32),
    SetSlotValue(u32, ValueContainer),
    DropSlot(u32),
    GetNextRegularInstruction,
    GetNextTypeInstruction,
    /// yields an external interrupt to be handled by the execution loop caller (for I/O operations, pointer resolution, remote execution, etc.)
    External(ExternalExecutionInterrupt)
}

#[derive(Debug)]
pub enum ExternalExecutionInterrupt {
    Result(Option<ValueContainer>),
    IntermediateResult(ExecutionLoopState, Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    GetInternalSlotValue(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
}

#[derive(Debug)]
pub enum InterruptProvider {
    ResolvedValue(Option<ValueContainer>),
    NextRegularInstruction(RegularInstruction),
    NextTypeInstruction(TypeInstruction),
}

/// Main execution loop that drives the execution of the DXB body
/// The interrupt_provider is used to provide results for synchronous or asynchronous I/O operations
pub fn execution_loop(
    state: RuntimeExecutionState,
    dxb_body: Rc<RefCell<Vec<u8>>>,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>> {
    gen move {

        let mut instruction_iterator = iterate_instructions(dxb_body);
        let mut slots = state.slots;

        let first_instruction = instruction_iterator.next();

        if let Some(Ok(Instruction::RegularInstruction(first_instruction))) = first_instruction {
            // execute the root instruction, which will drive further recursive execution
            let inner_iterator = execute_regular_instruction(
                interrupt_provider.clone(),
                first_instruction,
            );
            'main: for step in inner_iterator {
                let step = yield_unwrap!(step);
                info!("Execution loop step: {:?}", step);
                match step {
                    // yield external steps directly to be handled by the caller
                    ExecutionInterrupt::External(external_step) => {
                        yield Ok(external_step);
                    }
                    // final execution result - loop ends here
                    ExecutionInterrupt::ValueReturn(value) => {
                        return yield Ok(ExternalExecutionInterrupt::Result(value))
                    }
                    // feed new instructions to execution as long as they are requested
                    ExecutionInterrupt::GetNextRegularInstruction => {
                        loop {
                            match next_iter!(instruction_iterator, 'main) {
                                // feed next regular instruction
                                Ok(Instruction::RegularInstruction(next_instruction)) => {
                                    interrupt_provider.borrow_mut().replace(
                                        InterruptProvider::NextRegularInstruction(
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
                                Err(DXBParserError::ExpectingMoreInstructions) => {
                                    yield Err(ExecutionError::DXBParserError(
                                        DXBParserError::ExpectingMoreInstructions,
                                    ));
                                    // assume that when continuing after this yield, more instructions will have been loaded
                                    // so we run the loop again to try to get the next instruction
                                    continue;
                                }
                                // other parsing errors from instruction iterator
                                Err(err) => {
                                    return yield Err(ExecutionError::DXBParserError(err));
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
                                Ok(Instruction::TypeInstruction(next_instruction)) => {
                                    interrupt_provider.borrow_mut().replace(
                                        InterruptProvider::NextTypeInstruction(
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
                                Err(DXBParserError::ExpectingMoreInstructions) => {
                                    yield Err(ExecutionError::DXBParserError(
                                        DXBParserError::ExpectingMoreInstructions,
                                    ));
                                    // assume that when continuing after this yield, more instructions will have been loaded
                                    // so we run the loop again to try to get the next instruction
                                    continue;
                                }
                                // other parsing errors from instruction iterator
                                Err(err) => {
                                    return yield Err(ExecutionError::DXBParserError(err));
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
                            let val = yield_unwrap!(slots.get_slot_value(address));
                            interrupt_provider
                                .borrow_mut()
                                .replace(InterruptProvider::ResolvedValue(val));
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
                    // only for internal interrupts
                    ExecutionInterrupt::TypeReturn(_) => unreachable!(),
                    ExecutionInterrupt::KeyValuePairReturn(_) => unreachable!(),
                    ExecutionInterrupt::StatementsEnd => unreachable!(),
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
