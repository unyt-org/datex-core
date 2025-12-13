pub mod state;
mod type_instruction_iteration;

use core::cell::RefCell;
use crate::stdlib::rc::Rc;
use log::info;
use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator, BitwiseUnaryOperator, ComparisonOperator, LogicalUnaryOperator, ReferenceUnaryOperator, UnaryOperator};
use crate::global::operators::binary::{ArithmeticOperator, BitwiseOperator, LogicalOperator};
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, RegularInstruction, IntegerData, RawFullPointerAddress, RawInternalPointerAddress, RawLocalPointerAddress, RawPointerAddress, ShortTextData, SlotAddress, TextData, TypeInstruction};
use crate::parser::body;
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::{ExecutionError, ExecutionInput, InvalidProgramError};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::execution_loop::type_instruction_iteration::get_type_from_instructions;
use crate::runtime::execution::macros::{handle_steps, intercept_steps, interrupt, interrupt_with_next_type_instruction, interrupt_with_result, next_iter, yield_unwrap};
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
use crate::values::core_values::map::Map;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

#[derive(Debug)]
pub enum ExecutionStep {
    InternalReturn(Option<ValueContainer>),
    InternalTypeReturn(Type),
    Return(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    GetInternalSlot(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Pause,
    NextTypeInstruction,
}

#[derive(Debug)]
pub enum InterruptProvider {
    Result(Option<ValueContainer>),
    NextTypeInstruction(TypeInstruction),
}


pub fn execute_loop(
    input: ExecutionInput,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        let dxb_body = input.dxb_body;
        let end_execution = input.end_execution;
        let context = input.context;
        let next_instructions_stack = &mut context.clone().borrow_mut().next_instructions_stack;

        let instruction_iterator = body::iterate_instructions(dxb_body, next_instructions_stack);

        for instruction in instruction_iterator {
            // TODO #100: use ? operator instead of yield_unwrap once supported in gen blocks
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
                let mut context_mut = context.borrow_mut();
                context_mut.pop_next_scope = false;
                if let Some(value) = result_value {
                    let res = handle_value(&mut context_mut, value);
                    drop(context_mut);
                    yield_unwrap!(res);
                } else {
                    drop(context_mut);
                }

                let mut context_mut = context.borrow_mut();

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
            let res = match context.borrow_mut().scope_stack.take_active_value()
            {
                None => ExecutionStep::Return(None),
                Some(val) => ExecutionStep::Return(Some(val)),
            };
            yield Ok(res);
        } else {
            yield Ok(ExecutionStep::Pause);
        }
    }
}

#[inline]
fn get_result_value_from_instruction(
    context: Rc<RefCell<RuntimeExecutionState>>,
    instruction: RegularInstruction,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        yield Ok(ExecutionStep::InternalReturn(match instruction {
            // boolean
            RegularInstruction::True => Some(true.into()),
            RegularInstruction::False => Some(false.into()),

            // integers
            RegularInstruction::Int8(integer) => Some(Integer::from(integer.0).into()),
            RegularInstruction::Int16(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::Int32(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::Int64(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::Int128(integer) => {
                Some(Integer::from(integer.0).into())
            }

            // unsigned integers
            RegularInstruction::UInt8(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::UInt16(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::UInt32(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::UInt64(integer) => {
                Some(Integer::from(integer.0).into())
            }
            RegularInstruction::UInt128(integer) => {
                Some(Integer::from(integer.0).into())
            }

            // big integers
            RegularInstruction::BigInteger(IntegerData(integer)) => {
                Some(integer.into())
            }

            // specific floats
            RegularInstruction::DecimalF32(Float32Data(f32)) => {
                Some(TypedDecimal::from(f32).into())
            }
            RegularInstruction::DecimalF64(Float64Data(f64)) => {
                Some(TypedDecimal::from(f64).into())
            }

            // default decimals (big decimals)
            RegularInstruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                Some(Decimal::from(i16 as f32).into())
            }
            RegularInstruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                Some(Decimal::from(i32 as f32).into())
            }
            RegularInstruction::Decimal(DecimalData(big_decimal)) => {
                Some(big_decimal.into())
            }

            // endpoint
            RegularInstruction::Endpoint(endpoint) => Some(endpoint.into()),

            // null
            RegularInstruction::Null => Some(Value::null().into()),

            // text
            RegularInstruction::ShortText(ShortTextData(text)) => Some(text.into()),
            RegularInstruction::Text(TextData(text)) => Some(text.into()),

            // binary operations
            RegularInstruction::Add
            | RegularInstruction::Subtract
            | RegularInstruction::Multiply
            | RegularInstruction::Divide => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::BinaryOperation {
                        operator: BinaryOperator::from(instruction),
                    },
                );
                None
            }

            // unary operations
            RegularInstruction::UnaryPlus => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Plus,
                        ),
                    },
                );
                None
            }
            RegularInstruction::UnaryMinus => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Minus,
                        ),
                    },
                );
                None
            }
            RegularInstruction::BitwiseNot => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Bitwise(
                            BitwiseUnaryOperator::Negation,
                        ),
                    },
                );
                None
            }

            // equality operations
            RegularInstruction::Is
            | RegularInstruction::Matches
            | RegularInstruction::StructuralEqual
            | RegularInstruction::Equal
            | RegularInstruction::NotStructuralEqual
            | RegularInstruction::NotEqual => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::ComparisonOperation {
                        operator: ComparisonOperator::from(instruction),
                    },
                );
                None
            }

            RegularInstruction::ExecutionBlock(block) => {
                // build dxb

                let mut buffer = Vec::with_capacity(256);
                for (addr, local_slot) in
                    block.injected_slots.into_iter().enumerate()
                {
                    buffer.push(InstructionCode::ALLOCATE_SLOT as u8);
                    append_u32(&mut buffer, addr as u32);

                    if let Some(vc) = yield_unwrap!(
                        context.borrow().get_slot_value(local_slot).map_err(
                            |_| ExecutionError::SlotNotAllocated(local_slot),
                        )
                    ) {
                        buffer.extend_from_slice(&compile_value_container(&vc));
                    } else {
                        return yield Err(ExecutionError::SlotNotInitialized(
                            local_slot,
                        ));
                    }
                }
                buffer.extend_from_slice(&block.body);

                let maybe_receivers =
                    context.borrow_mut().scope_stack.take_active_value();

                if let Some(receivers) = maybe_receivers {
                    interrupt_with_result!(
                        interrupt_provider,
                        ExecutionStep::RemoteExecution(receivers, buffer)
                    )
                } else {
                    // should not happen, receivers must be set
                    yield Err(ExecutionError::InvalidProgram(
                        InvalidProgramError::MissingRemoteExecutionReceiver,
                    ));
                    None
                }
            }

            RegularInstruction::CloseAndStore => {
                let _ = context.borrow_mut().scope_stack.take_active_value();
                None
            }

            RegularInstruction::Apply(ApplyData { arg_count }) => {
                context.borrow_mut().scope_stack.create_scope(Scope::Apply {
                    arg_count,
                    args: vec![],
                });
                None
            }

            RegularInstruction::ScopeStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::Default);
                None
            }

            RegularInstruction::ListStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        List::default().into(),
                    );
                None
            }

            RegularInstruction::MapStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        Map::default().into(),
                    );
                None
            }

            RegularInstruction::KeyValueShortText(ShortTextData(key)) => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::KeyValuePair,
                        key.into(),
                    );
                None
            }

            RegularInstruction::KeyValueDynamic => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::KeyValuePair);
                None
            }

            RegularInstruction::ScopeEnd => {
                // pop scope and return value
                yield_unwrap!(context.borrow_mut().scope_stack.pop())
            }

            // slots
            RegularInstruction::AllocateSlot(SlotAddress(address)) => {
                let mut context = context.borrow_mut();
                context.allocate_slot(address, None);
                context
                    .scope_stack
                    .create_scope(Scope::SlotAssignment { address });
                None
            }
            RegularInstruction::GetSlot(SlotAddress(address)) => {
                // if address is >= 0xffffff00, resolve internal slot
                if address >= 0xffffff00 {
                    interrupt_with_result!(
                        interrupt_provider,
                        ExecutionStep::GetInternalSlot(address)
                    )
                }
                // else handle normal slot
                else {
                    let res = context.borrow_mut().get_slot_value(address);
                    // get value from slot
                    let slot_value = yield_unwrap!(res);
                    if slot_value.is_none() {
                        return yield Err(ExecutionError::SlotNotInitialized(
                            address,
                        ));
                    }
                    slot_value
                }
            }
            RegularInstruction::SetSlot(SlotAddress(address)) => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::SlotAssignment { address });
                None
            }

            RegularInstruction::AssignToReference(operator) => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::AssignToReference {
                        reference: None,
                        operator,
                    },
                );
                None
            }

            RegularInstruction::Deref => {
                context.borrow_mut().scope_stack.create_scope(Scope::Deref);
                None
            }

            RegularInstruction::GetRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolvePointer(address)
                )
            }

            RegularInstruction::GetLocalRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolveLocalPointer(address)
                )
            }

            RegularInstruction::GetInternalRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolveInternalPointer(address)
                )
            }

            RegularInstruction::AddAssign(SlotAddress(address)) => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::AssignmentOperation {
                        address,
                        operator: AssignmentOperator::AddAssign,
                    },
                );
                None
            }

            RegularInstruction::SubtractAssign(SlotAddress(address)) => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::AssignmentOperation {
                        address,
                        operator: AssignmentOperator::SubtractAssign,
                    },
                );
                None
            }

            // refs
            RegularInstruction::CreateRef => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Reference(
                            ReferenceUnaryOperator::CreateRef,
                        ),
                    },
                );
                None
            }

            RegularInstruction::CreateRefMut => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Reference(
                            ReferenceUnaryOperator::CreateRefMut,
                        ),
                    },
                );
                None
            }

            // remote execution
            RegularInstruction::RemoteExecution => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::RemoteExecution);
                None
            }

            RegularInstruction::DropSlot(SlotAddress(address)) => {
                // remove slot from slots
                let res = context.borrow_mut().drop_slot(address);
                yield_unwrap!(res);
                None
            }

            // TODO
            // RegularInstruction::TypeInstructions(instructions) => {
            //     for output in
            //         get_type_from_instructions(interrupt_provider, instructions)
            //     {
            //         // TODO #403: handle type here
            //         yield output;
            //     }
            //     return;
            // }
            //
            // // type(...)
            // RegularInstruction::TypeExpression(instructions) => {
            //     for output in
            //         get_type_from_instructions(interrupt_provider, instructions)
            //     {
            //         yield output;
            //     }
            //     return;
            // }

            i => {
                return yield Err(ExecutionError::NotImplemented(
                    format!("Instruction {i}").to_string(),
                ));
            }
        }))
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

fn handle_unary_reference_operation(
    operator: ReferenceUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    Ok(match operator {
        ReferenceUnaryOperator::CreateRef => {
            ValueContainer::Reference(Reference::from(value_container))
        }
        ReferenceUnaryOperator::CreateRefMut => {
            ValueContainer::Reference(Reference::try_mut_from(value_container)?)
        }
        ReferenceUnaryOperator::Deref => {
            if let ValueContainer::Reference(reference) = value_container {
                reference.value_container()
            } else {
                return Err(ExecutionError::DerefOfNonReference);
            }
        }
    })
}
fn handle_unary_logical_operation(
    operator: LogicalUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    unimplemented!(
        "Logical unary operations are not implemented yet: {operator:?}"
    )
}
fn handle_unary_arithmetic_operation(
    operator: ArithmeticUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        ArithmeticUnaryOperator::Minus => Ok((-value_container)?),
        ArithmeticUnaryOperator::Plus => Ok(value_container),
        _ => unimplemented!(
            "Arithmetic unary operations are not implemented yet: {operator:?}"
        ),
    }
}

fn handle_unary_operation(
    operator: UnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        UnaryOperator::Reference(reference) => {
            handle_unary_reference_operation(reference, value_container)
        }
        UnaryOperator::Logical(logical) => {
            handle_unary_logical_operation(logical, value_container)
        }
        UnaryOperator::Arithmetic(arithmetic) => {
            handle_unary_arithmetic_operation(arithmetic, value_container)
        }
        _ => {
            core::todo!("#102 Unary instruction not implemented: {operator:?}")
        }
    }
}

fn handle_comparison_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: ComparisonOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        ComparisonOperator::StructuralEqual => {
            let val = active_value_container.structural_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Equal => {
            let val = active_value_container.value_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::NotStructuralEqual => {
            let val = !active_value_container.structural_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::NotEqual => {
            let val = !active_value_container.value_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Is => {
            // TODO #103 we should throw a runtime error when one of lhs or rhs is a value
            // instead of a ref. Identity checks using the is operator shall be only allowed
            // for references.
            // @benstre: or keep as always false ? - maybe a compiler check would be better
            let val = active_value_container.identical(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Matches => {
            // TODO #407: Fix matches, rhs will always be a type, so actual_type() call is wrong
            let v_type = value_container.actual_container_type(); // Type::try_from(value_container)?;
            let val = v_type.value_matches(active_value_container);
            Ok(ValueContainer::from(val))
        }
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

fn handle_assignment_operation(
    lhs: ValueContainer,
    rhs: ValueContainer,
    operator: AssignmentOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        AssignmentOperator::AddAssign => Ok((lhs + rhs)?),
        AssignmentOperator::SubtractAssign => Ok((lhs - rhs)?),
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

fn handle_arithmetic_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: ArithmeticOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        ArithmeticOperator::Add => {
            Ok((active_value_container + &value_container)?)
        }
        ArithmeticOperator::Subtract => {
            Ok((active_value_container - &value_container)?)
        }
        // ArithmeticOperator::Multiply => {
        //     Ok((active_value_container * &value_container)?)
        // }
        // ArithmeticOperator::Divide => {
        //     Ok((active_value_container / &value_container)?)
        // }
        _ => {
            core::todo!(
                "#408 Implement arithmetic operation for {:?}",
                operator
            );
        }
    }
}

fn handle_bitwise_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: BitwiseOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    {
        core::todo!("#409 Implement bitwise operation for {:?}", operator);
    }
}

fn handle_logical_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: LogicalOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    {
        core::todo!("#410 Implement logical operation for {:?}", operator);
    }
}

fn handle_binary_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: BinaryOperator,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        BinaryOperator::Arithmetic(arith_op) => handle_arithmetic_operation(
            active_value_container,
            value_container,
            arith_op,
        ),
        BinaryOperator::Bitwise(bitwise_op) => handle_bitwise_operation(
            active_value_container,
            value_container,
            bitwise_op,
        ),
        BinaryOperator::Logical(logical_op) => handle_logical_operation(
            active_value_container,
            value_container,
            logical_op,
        ),
    }
}
