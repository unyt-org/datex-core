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
    GetInternalSlot(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Apply(ValueContainer, Vec<ValueContainer>),
    Pause,
    GetNextInstruction,
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
                        state
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
                        }
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

        for instruction in instruction_iterator {
            let instruction = yield_unwrap!(instruction);
            if input.options.verbose {
                info!("[Exec]: {instruction}");
            }

            // get initial value from instruction
            let mut result_value = None;

            // TODO:
            // handle_steps!(
            //     get_result_value_from_instruction(
            //         context.clone(),
            //         instruction,
            //         interrupt_provider.clone(),
            //     ),
            //     Ok(ExecutionStep::InternalReturn(result)) => {
            //         result_value = result;
            //     },
            //     Ok(ExecutionStep::InternalTypeReturn(result)) => {
            //         context.borrow_mut().scope_stack.get_current_scope_mut().active_type = Some(result);
            //         // result_value = Some(ValueContainer::from(result));
            //     },
            //     step => {
            //         let step = yield_unwrap!(step);
            //         *interrupt_provider.borrow_mut() =
            //             Some(interrupt!(interrupt_provider, step));
            //     }
            // );


            // 1. if value is Some, handle it
            // 2. while pop_next_scope is true: pop current scope and repeat
            loop {
                let mut context_mut = state.borrow_mut();
                context_mut.pop_next_scope = false;
                if let Some(value) = result_value {
                    let res = handle_value(&mut context_mut, value);
                    drop(context_mut);
                    yield_unwrap!(res);
                } else {
                    drop(context_mut);
                }

                let mut context_mut = state.borrow_mut();

                if context_mut.pop_next_scope {
                    let res = context_mut.scope_stack.pop();
                    drop(context_mut);
                    result_value = yield_unwrap!(res);
                } else {
                    break;
                }
            }
        }

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

/// Takes a produced value and handles it according to the current scope
fn handle_value(
    context: &mut RuntimeExecutionState,
    mut value_container: ValueContainer,
) -> Result<(), ExecutionError> {
    let active_type = context.scope_stack.take_active_type();
    let scope_container = context.scope_stack.get_current_scope_mut();

    // cast to active type if exists
    if let Some(active_type) = active_type {
        match &mut value_container {
            ValueContainer::Value(value) => {
                // FIXME: only using type definition here, refactor and/or add checks
                value.actual_type = Box::new(active_type.type_definition);
            }
            _ => panic!("Expected ValueContainer::Value for type casting"),
        }
    }

    let result_value = match &mut scope_container.scope {
        Scope::KeyValuePair => {
            let key = &scope_container.active_value;
            match key {
                // set key as active_value for key-value pair (for dynamic keys)
                None => Some(value_container),

                // set value for key-value pair
                Some(_) => {
                    let key = context.scope_stack.pop()?.unwrap();
                    match context.scope_stack.get_active_value_mut() {
                        Some(collector) => {
                            // handle active value collector
                            handle_key_value_pair(
                                collector,
                                &key,
                                value_container,
                            )?;
                        }
                        None => unreachable!(
                            "Expected active value for key-value pair, but got None"
                        ),
                    }
                    None
                }
            }
        }

        Scope::SlotAssignment { address } => {
            // set value for slot
            let address = *address;
            context.set_slot_value(address, value_container.clone())?;
            Some(value_container)
        }

        Scope::Deref => {
            // set value for slot
            if let ValueContainer::Reference(reference) = value_container {
                Some(reference.value_container())
            } else {
                return Err(ExecutionError::DerefOfNonReference);
            }
        }

        Scope::AssignToReference {
            operator,
            reference,
        } => {
            if (reference.is_none()) {
                // set value for slot
                if let ValueContainer::Reference(new_reference) =
                    value_container
                {
                    reference.replace(new_reference);
                    None
                } else {
                    return Err(ExecutionError::DerefOfNonReference);
                }
            } else {
                let operator = *operator;
                let reference = reference.as_ref().unwrap();
                let lhs = reference.value_container();
                let res = handle_assignment_operation(
                    lhs,
                    value_container,
                    operator,
                )?;
                reference.set_value_container(res)?;
                Some(ValueContainer::Reference(reference.clone()))
            }
        }

        Scope::Apply { args, arg_count } => {
            // collect callee as active value if not set yet and we have args to collect
            if scope_container.active_value.is_none() {
                // directly apply if no args to collect
                if *arg_count == 0 {
                    context.pop_next_scope = true;
                    handle_apply(&value_container, args)?
                }
                // set callee as active value
                else {
                    Some(value_container)
                }
            } else {
                let callee = scope_container.active_value.as_ref().unwrap();
                // callee already exists, collect args
                args.push(value_container);

                // all args collected, apply function
                if args.len() == *arg_count as usize {
                    context.pop_next_scope = true;
                    handle_apply(callee, args)?
                } else {
                    Some(callee.clone())
                }
            }
        }

        Scope::AssignmentOperation { operator, address } => {
            let operator = *operator;
            let address = *address;
            let lhs = if let Ok(Some(val)) = context.get_slot_value(address) {
                val
            } else {
                return Err(ExecutionError::SlotNotInitialized(address));
            };
            let res =
                handle_assignment_operation(lhs, value_container, operator)?;
            context.set_slot_value(address, res.clone())?;
            Some(res)
        }

        Scope::UnaryOperation { operator } => {
            let operator = *operator;
            context.pop_next_scope = true;
            let result = handle_unary_operation(operator, value_container);
            if let Ok(val) = result {
                Some(val)
            } else {
                // handle error
                return Err(result.unwrap_err());
            }
        }

        Scope::BinaryOperation { operator } => {
            let active_value = &scope_container.active_value;
            match active_value {
                Some(active_value_container) => {
                    let res = handle_binary_operation(
                        active_value_container,
                        value_container,
                        *operator,
                    );
                    if let Ok(val) = res {
                        // set val as active value
                        context.pop_next_scope = true;
                        Some(val)
                    } else {
                        // handle error
                        return Err(res.unwrap_err());
                    }
                }
                None => Some(value_container),
            }
        }

        Scope::ComparisonOperation { operator } => {
            let active_value = &scope_container.active_value;
            match active_value {
                Some(active_value_container) => {
                    let res = handle_comparison_operation(
                        active_value_container,
                        value_container,
                        *operator,
                    );
                    if let Ok(val) = res {
                        // set val as active value
                        context.pop_next_scope = true;
                        Some(val)
                    } else {
                        // handle error
                        return Err(res.unwrap_err());
                    }
                }
                None => Some(value_container),
            }
        }

        Scope::Collection => {
            let active_value = &mut scope_container.active_value;
            match active_value {
                Some(active_value_container) => {
                    // handle active value collector
                    handle_collector(active_value_container, value_container);
                    None
                }
                None => {
                    unreachable!(
                        "Expected active value for collection scope, but got None"
                    );
                }
            }
        }

        _ => Some(value_container),
    };

    if let Some(result_value) = result_value {
        context.scope_stack.set_active_value_container(result_value);
    }

    Ok(())
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

fn handle_collector(collector: &mut ValueContainer, value: ValueContainer) {
    match collector {
        ValueContainer::Value(Value {
                                  inner: CoreValue::List(list),
                                  ..
                              }) => {
            // append value to list
            list.push(value);
        }
        ValueContainer::Value(Value {
                                  inner: CoreValue::Map(map),
                                  ..
                              }) => {
            // TODO #406: Implement map collector for optimized structural maps
            core::panic!("append {:?}", value);
        }
        _ => {
            unreachable!("Unsupported collector for collection scope");
        }
    }
}

fn handle_key_value_pair(
    active_container: &mut ValueContainer,
    key: &ValueContainer,
    value: ValueContainer,
) -> Result<(), ExecutionError> {
    // insert key value pair into active map
    match active_container {
        // Map
        ValueContainer::Value(Value {
                                  inner: CoreValue::Map(map),
                                  ..
                              }) => {
            // make sure key is a string
            map.try_set(key, value)
                .expect("Failed to set key-value pair in map");
        }
        _ => {
            unreachable!(
                "Expected active value that can collect key value pairs, but got: {}",
                active_container
            );
        }
    }

    Ok(())
}
