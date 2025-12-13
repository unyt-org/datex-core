pub mod state;
pub mod type_instruction_iteration;
pub  mod regular_instruction_iteration;
mod operations;

use core::cell::RefCell;
use crate::stdlib::rc::Rc;
use log::info;
use datex_core::runtime::execution::execution_loop::regular_instruction_iteration::next_regular_instruction_iteration;
use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator, BitwiseUnaryOperator, ComparisonOperator, LogicalUnaryOperator, ReferenceUnaryOperator, UnaryOperator};
use crate::global::operators::binary::{ArithmeticOperator, BitwiseOperator, LogicalOperator};
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, RegularInstruction, IntegerData, RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress, RawPointerAddress, ShortTextData, SlotAddress, TextData, TypeInstruction, Instruction};
use crate::parser::body;
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::{ExecutionError, ExecutionInput, InvalidProgramError};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::execution_loop::type_instruction_iteration::{get_type_from_instructions, next_type_instruction_iteration};
use crate::runtime::execution::macros::{handle_steps, intercept_steps, interrupt, interrupt_with_maybe_value, next_iter, yield_unwrap};
use crate::runtime::execution::stack::Scope;
use crate::traits::apply::Apply;
use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::types::definition::TypeDefinition;
use crate::utils::buffers::append_u32;
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
pub enum ExecutionStep {
    ValueReturn(Option<ValueContainer>),
    TypeReturn(Type),
    KeyValuePairReturn((OwnedMapKey, ValueContainer)),
    Result(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    AllocateSlot(u32, ValueContainer),
    GetSlotValue(u32),
    SetSlotValue(u32, ValueContainer),
    DropSlot(u32),
    GetInternalSlotValue(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
    Pause,
    GetNextInstruction,
}

// TODO ExecutionStep::External
#[derive(Debug)]
pub enum ExternalExecutionStep {
    Result(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    GetInternalSlotValue(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
    Pause,
}


#[derive(Debug)]
pub enum InterruptProvider {
    ResolvedValue(Option<ValueContainer>),
    NextRegularInstruction(RegularInstruction),
    NextTypeInstruction(TypeInstruction),
}

/// Main execution loop that drives the execution of the DXB body
/// The interrupt_provider is used to provide synchronous or asynchronous I/O operations
pub fn execute_loop(
    input: ExecutionInput,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        let dxb_body = input.dxb_body;
        let end_execution = input.end_execution;
        let state = input.state;
        let next_instructions_stack = state.borrow_mut().next_instructions_stack.clone();

        let mut instruction_iterator = body::iterate_instructions(dxb_body, next_instructions_stack);

        while let Some(instruction) = instruction_iterator.next() {
            let instruction = yield_unwrap!(instruction);
            if input.options.verbose {
                info!("[Exec]: {instruction}");
            }

            match instruction {
                Instruction::RegularInstruction(regular_instruction) => {
                    let inner_iterator = next_regular_instruction_iteration(
                        interrupt_provider.clone(),
                        regular_instruction,
                    );
                    intercept_steps!(
                        inner_iterator,
                        // feed new type instructions as long as they are requested
                        Ok(ExecutionStep::GetNextInstruction) => {
                            match yield_unwrap!(next_iter!(instruction_iterator)) {
                                Instruction::RegularInstruction(next_instruction) => {
                                    interrupt_provider.borrow_mut().replace(
                                        InterruptProvider::NextRegularInstruction(
                                            next_instruction,
                                        ),
                                    );
                                }
                                _ => unreachable!()
                            }
                        },
                        // Ok(ExecutionStep::GetSlotValue(address)) => {
                        //     // if address is >= 0xffffff00, resolve internal slot
                        //     if address >= 0xffffff00 {
                        //         interrupt_with_maybe_value!(
                        //             interrupt_provider,
                        //             ExecutionStep::GetSlotValue(address)
                        //         )
                        //     }
                        //     // else handle normal slot
                        //     else {
                        //         let res = state.borrow_mut().get_slot_value(address);
                        //         // get value from slot
                        //         let slot_value = yield_unwrap!(res);
                        //         if slot_value.is_none() {
                        //             return yield Err(ExecutionError::SlotNotInitialized(
                        //                 address,
                        //             ));
                        //         }
                        //         slot_value
                        //     }
                        // }
                    )
                }
                Instruction::TypeInstruction(type_instruction) => {
                    let inner_iterator = next_type_instruction_iteration(
                        interrupt_provider.clone(),
                        type_instruction,
                    );
                    intercept_steps!(
                        inner_iterator,
                        // feed new type instructions as long as they are requested
                        Ok(ExecutionStep::GetNextInstruction) => {
                            match yield_unwrap!(next_iter!(instruction_iterator)) {
                                Instruction::TypeInstruction(next_instruction) => {
                                    interrupt_provider.borrow_mut().replace(
                                        InterruptProvider::NextTypeInstruction(
                                            next_instruction,
                                        ),
                                    );
                                }
                                _ => unreachable!()
                            }
                        }
                    )
                }
            }
        }


        //////////////////////////////////////////////////////////////// OLD ////////////////////////////////////////////////////////

        // for instruction in instruction_iterator {
        //     let instruction = yield_unwrap!(instruction);
        //     if input.options.verbose {
        //         info!("[Exec]: {instruction}");
        //     }
        //
        //     // get initial value from instruction
        //     let mut result_value = None;
        //
        //     // TODO:
        //     // handle_steps!(
        //     //     get_result_value_from_instruction(
        //     //         context.clone(),
        //     //         instruction,
        //     //         interrupt_provider.clone(),
        //     //     ),
        //     //     Ok(ExecutionStep::InternalReturn(result)) => {
        //     //         result_value = result;
        //     //     },
        //     //     Ok(ExecutionStep::InternalTypeReturn(result)) => {
        //     //         context.borrow_mut().scope_stack.get_current_scope_mut().active_type = Some(result);
        //     //         // result_value = Some(ValueContainer::from(result));
        //     //     },
        //     //     step => {
        //     //         let step = yield_unwrap!(step);
        //     //         *interrupt_provider.borrow_mut() =
        //     //             Some(interrupt!(interrupt_provider, step));
        //     //     }
        //     // );
        //
        //
        //     // 1. if value is Some, handle it
        //     // 2. while pop_next_scope is true: pop current scope and repeat
        //     loop {
        //         let mut context_mut = state.borrow_mut();
        //         context_mut.pop_next_scope = false;
        //         if let Some(value) = result_value {
        //             let res = handle_value(&mut context_mut, value);
        //             drop(context_mut);
        //             yield_unwrap!(res);
        //         } else {
        //             drop(context_mut);
        //         }
        //
        //         let mut context_mut = state.borrow_mut();
        //
        //         if context_mut.pop_next_scope {
        //             let res = context_mut.scope_stack.pop();
        //             drop(context_mut);
        //             result_value = yield_unwrap!(res);
        //         } else {
        //             break;
        //         }
        //     }
        // }

        if end_execution {
            // cleanup...
            // TODO #101: check for other unclosed stacks
            // if we have an active key here, this is invalid and leads to an error
            // if context.scope_stack.get_active_key().is_some() {
            //     return Err(ExecutionError::InvalidProgram(
            //         InvalidProgramError::UnterminatedSequence,
            //     ));
            // }

            // removes the current active value from the scope stack
            let res = match state.borrow_mut().scope_stack.take_active_value()
            {
                None => ExecutionStep::Result(None),
                Some(val) => ExecutionStep::Result(Some(val)),
            };
            yield Ok(res);
        } else {
            yield Ok(ExecutionStep::Pause);
        }
    }
}

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