use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{AssignmentOperator, BinaryOperator, ComparisonOperator, UnaryOperator};
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, IntegerData, ShortTextData, SlotAddress, TextData, UnboundedStatementsData};
use crate::runtime::execution::execution_loop::operations::{
    handle_assignment_operation, handle_binary_operation,
    handle_unary_operation,
};
use crate::runtime::execution::execution_loop::type_instruction_execution::get_next_type;
use crate::runtime::execution::execution_loop::{ExternalExecutionInterrupt, InterruptResult};
use crate::runtime::execution::macros::{intercept_maybe_step, intercept_step, interrupt, interrupt_with_maybe_value, yield_unwrap};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::stdlib::rc::Rc;
use crate::types::definition::TypeDefinition;
use crate::utils::buffers::append_u32;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::list::List;
use crate::values::core_values::map::{Map, OwnedMapKey};
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;
use log::info;
use datex_core::global::protocol_structures::instructions::RegularInstruction;
use datex_core::runtime::execution::execution_loop::operations::handle_comparison_operation;
use datex_core::runtime::execution::macros::interrupt_with_value;
use datex_core::values::core_value::CoreValue;
use crate::runtime::execution::execution_loop::interrupts::{ExecutionInterrupt, InterruptProvider};

/// Yield an interrupt and get the next regular instruction,
/// expecting the next input to be a NextRegularInstruction variant
macro_rules! interrupt_with_next_regular_instruction {
    ($input:expr) => {{
        use crate::runtime::execution::macros::interrupt;

        let res = interrupt!($input, ExecutionInterrupt::GetNextRegularInstruction).unwrap();
        match res {
            InterruptResult::NextRegularInstruction(value) => value,
            _ => unreachable!(), // must be ensured by execution loop
        }
    }};
}

/// Drives the regular instruction iteration to get the next value
/// Returns the resolved value or None if the next instructions did not generate a value
macro_rules! get_next_maybe_value {
    ($interrupt_provider:expr) => {{
        let next = interrupt_with_next_regular_instruction!($interrupt_provider);
        let mut inner_iterator = execute_regular_instruction($interrupt_provider, next);
        intercept_step!(
            inner_iterator,
            Ok(ExecutionInterrupt::ValueReturn(value)) => {
                value
            }
        )
    }};
}

/// Drives the regular instruction iteration to get the next value
/// Returns the resolved value or aborts with an ExecutionError if no value could be resolved (should not happen in valid program)
macro_rules! get_next_value {
    ($interrupt_provider:expr) => {{
        let maybe_value = get_next_maybe_value!($interrupt_provider);
        match maybe_value {
            Some(value) => value,
            _ => {
                return yield Err(ExecutionError::InvalidProgram(
                    InvalidProgramError::ExpectedValue,
                ));
            }
        }
    }};
}

/// Drives the regular instruction iteration to get the next key value pair
/// Returns the key value pair or aborts with an ExecutionError (should not happen in valid program)
macro_rules! get_next_key_value_pair {
    ($interrupt_provider:expr) => {{
        let next = interrupt_with_next_regular_instruction!($interrupt_provider);
        let mut inner_iterator = execute_regular_instruction($interrupt_provider, next);
        intercept_step!(
            inner_iterator,
            Ok(ExecutionInterrupt::KeyValuePairReturn(value)) => {
                value
            }
        )
    }};
}


enum ValueOrStatementsEnd {
    Value(Option<ValueContainer>),
    StatementsEnd(bool)
}

/// Drives the regular instruction iteration until the next statement or end of statements
macro_rules! get_next_value_or_statements_end {
    ($interrupt_provider:expr) => {{
        let next = interrupt_with_next_regular_instruction!($interrupt_provider);
        let mut inner_iterator = execute_regular_instruction($interrupt_provider, next);
        intercept_step!(
            inner_iterator,
            Ok(ExecutionInterrupt::ValueReturn(value)) => {
                ValueOrStatementsEnd::Value(value)
            },
            Ok(ExecutionInterrupt::StatementsEnd(terminated)) => {
                ValueOrStatementsEnd::StatementsEnd(terminated)
            }
        )
    }}
}

/// TODO: put interrupt provider together with state (active value, stack) and pass that around
/// instead of using interrupts for accessing the state
pub(crate) fn execute_regular_instruction(
    interrupt_provider: InterruptProvider,
    instruction: RegularInstruction,
) -> Box<impl Iterator<Item = Result<ExecutionInterrupt, ExecutionError>>> {
    Box::new(gen move {
        yield Ok(ExecutionInterrupt::ValueReturn(match instruction {
            // boolean
            RegularInstruction::True => Some(true.into()),
            RegularInstruction::False => Some(false.into()),

            // integers
            RegularInstruction::Int8(integer) => {
                Some(Integer::from(integer.0).into())
            }
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
            RegularInstruction::ShortText(ShortTextData(text)) => {
                Some(text.into())
            }
            RegularInstruction::Text(TextData(text)) => Some(text.into()),

            // binary operations
            RegularInstruction::Add
            | RegularInstruction::Subtract
            | RegularInstruction::Multiply
            | RegularInstruction::Divide => {
                let lhs = get_next_value!(interrupt_provider.clone());
                let rhs = get_next_value!(interrupt_provider);

                let res = handle_binary_operation(
                    BinaryOperator::from(instruction),
                    &lhs,
                    &rhs,
                );
                Some(yield_unwrap!(res))
            }

            // comparison operations
            RegularInstruction::Is
            | RegularInstruction::Matches
            | RegularInstruction::StructuralEqual
            | RegularInstruction::Equal
            | RegularInstruction::NotStructuralEqual
            | RegularInstruction::NotEqual => {
                let lhs = get_next_value!(interrupt_provider.clone());
                let rhs = get_next_value!(interrupt_provider);

                let res = handle_comparison_operation(
                    ComparisonOperator::from(instruction),
                    &lhs,
                    &rhs,
                );
                Some(yield_unwrap!(res))
            }


            // unary operations
            RegularInstruction::CreateRef
            | RegularInstruction::CreateRefMut
            | RegularInstruction::Deref
            | RegularInstruction::UnaryPlus
            | RegularInstruction::UnaryMinus
            | RegularInstruction::BitwiseNot => {
                let value = get_next_value!(interrupt_provider);
                Some(yield_unwrap!(handle_unary_operation(
                    UnaryOperator::from(instruction),
                    value,
                )))
            }

            RegularInstruction::RemoteExecution(exec_block_data) => {
                // build dxb
                let mut buffer = Vec::with_capacity(256);
                for (addr, local_slot) in
                    exec_block_data.injected_slots.into_iter().enumerate()
                {
                    buffer.push(InstructionCode::ALLOCATE_SLOT as u8);
                    append_u32(&mut buffer, addr as u32);

                    let slot_value = interrupt_with_value!(
                        interrupt_provider,
                        ExecutionInterrupt::GetSlotValue(local_slot)
                    );
                    buffer.extend_from_slice(&compile_value_container(
                        &slot_value,
                    ));

                    // if let Some(vc) = yield_unwrap!(
                    //     state.borrow().get_slot_value(local_slot).map_err(
                    //         |_| ExecutionError::SlotNotAllocated(local_slot),
                    //     )
                    // ) {
                    //     buffer.extend_from_slice(&compile_value_container(&vc));
                    // } else {
                    //     return yield Err(ExecutionError::SlotNotInitialized(
                    //         local_slot,
                    //     ));
                    // }
                }
                buffer.extend_from_slice(&exec_block_data.body);

                let receivers = get_next_value!(interrupt_provider.clone());

                interrupt_with_maybe_value!(
                    interrupt_provider,
                    ExecutionInterrupt::External(ExternalExecutionInterrupt::RemoteExecution(receivers, buffer))
                )
            }

            RegularInstruction::Apply(ApplyData { arg_count }) => {
                let callee = get_next_value!(interrupt_provider.clone());
                let mut args = Vec::with_capacity(arg_count as usize);
                for _ in 0..arg_count {
                    let arg = get_next_value!(interrupt_provider.clone());
                    args.push(arg);
                }
                interrupt_with_maybe_value!(
                    interrupt_provider,
                    ExecutionInterrupt::External(ExternalExecutionInterrupt::Apply(callee, args))
                )
            }

            RegularInstruction::Statements(statements_data)
            | RegularInstruction::ShortStatements(statements_data) => {
                let mut last_value = None;
                for _ in 0..statements_data.statements_count {
                    last_value =
                        get_next_maybe_value!(interrupt_provider.clone());
                }
                match statements_data.terminated {
                    true => None,
                    false => last_value,
                }
            }

            RegularInstruction::UnboundedStatements => {
                let mut last_value: Option<ValueContainer> = None;
                let mut terminated = false;
                loop {
                    match get_next_value_or_statements_end!(interrupt_provider.clone()) {
                        ValueOrStatementsEnd::Value(value) => {
                            // store as active value
                            interrupt!(
                                interrupt_provider,
                                ExecutionInterrupt::SetActiveValue(value.clone())
                            );
                            last_value = value;
                        }
                        ValueOrStatementsEnd::StatementsEnd(is_terminated) => {
                            terminated = is_terminated;
                            break;
                        }
                    }
                }
                match terminated {
                    true => None,
                    false => last_value,
                }
            }

            RegularInstruction::UnboundedStatementsEnd(terminated) => {
                return yield Ok(ExecutionInterrupt::StatementsEnd(terminated));
            }

            RegularInstruction::List(list_data)
            | RegularInstruction::ShortList(list_data)=> {
                let mut list = List::with_capacity(list_data.element_count);
                for _ in 0..list_data.element_count {
                    let element = get_next_value!(interrupt_provider.clone());
                    list.push(element);
                }
                Some(list.into())
            }

            RegularInstruction::Map(map_data)
            | RegularInstruction::ShortMap(map_data) => {
                let mut map = Map::default(); // TODO: optimize initial map construction (capacity, etc)
                for _ in 0..map_data.element_count {
                    let (key, value) =
                        get_next_key_value_pair!(interrupt_provider.clone());
                    map.set(&key, value);
                }
                Some(map.into())
            }

            RegularInstruction::KeyValueShortText(ShortTextData(key)) => {
                return yield Ok(ExecutionInterrupt::KeyValuePairReturn((
                    OwnedMapKey::Text(key),
                    get_next_value!(interrupt_provider),
                )));
            }

            RegularInstruction::KeyValueDynamic => {
                return yield Ok(ExecutionInterrupt::KeyValuePairReturn((
                    OwnedMapKey::Value(get_next_value!(
                        interrupt_provider.clone()
                    )),
                    get_next_value!(interrupt_provider),
                )));
            }

            // slots
            RegularInstruction::AllocateSlot(SlotAddress(address)) => {
                let value = get_next_value!(interrupt_provider.clone());
                interrupt!(
                    interrupt_provider,
                    ExecutionInterrupt::AllocateSlot(address, value.clone())
                );
                Some(value)
            }
            RegularInstruction::GetSlot(SlotAddress(address)) => {
                Some(interrupt_with_value!(
                    interrupt_provider,
                    ExecutionInterrupt::GetSlotValue(address)
                ))
            }
            RegularInstruction::SetSlot(SlotAddress(address)) => {
                let value = get_next_value!(interrupt_provider.clone());
                interrupt!(
                    interrupt_provider,
                    ExecutionInterrupt::SetSlotValue(address, value.clone())
                );
                Some(value)
            }

            RegularInstruction::AssignToReference(operator) => {
                let ref_value_container = get_next_value!(interrupt_provider.clone());
                let value_container = get_next_value!(interrupt_provider);

                // assignment value must be a reference
                if let Some(reference) = ref_value_container.maybe_reference() {
                    let lhs = reference.value_container();
                    let res = yield_unwrap!(handle_assignment_operation(
                        operator,
                        lhs.clone(),
                        value_container,
                    ));
                    yield_unwrap!(reference.set_value_container(res));
                    Some(ref_value_container)
                } else {
                    return yield Err(ExecutionError::DerefOfNonReference);
                }
            }

            RegularInstruction::GetRef(address) => {
                Some(interrupt_with_value!(
                    interrupt_provider,
                    ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolvePointer(address))
                ))
            }
            RegularInstruction::GetLocalRef(address) => {
                Some(interrupt_with_value!(
                    interrupt_provider,
                    ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolveLocalPointer(address))
                ))
            }
            RegularInstruction::GetInternalRef(address) => {
                Some(interrupt_with_value!(
                    interrupt_provider,
                    ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolveInternalPointer(address))
                ))
            }

            RegularInstruction::AddAssign(SlotAddress(address))
            | RegularInstruction::SubtractAssign(SlotAddress(address)) => {
                let slot_value = interrupt_with_value!(
                    interrupt_provider,
                    ExecutionInterrupt::GetSlotValue(address)
                );
                let value = get_next_value!(interrupt_provider.clone());

                let new_val = yield_unwrap!(handle_assignment_operation(
                    AssignmentOperator::from(instruction),
                    slot_value,
                    value,
                ));
                // set slot value
                interrupt!(
                    interrupt_provider,
                    ExecutionInterrupt::SetSlotValue(address, new_val.clone())
                );
                // return assigned value
                Some(new_val)
            }

            RegularInstruction::DropSlot(SlotAddress(address)) => {
                interrupt!(
                    interrupt_provider,
                    ExecutionInterrupt::DropSlot(address)
                );
                None
            }

            RegularInstruction::TypedValue => {
                let ty = get_next_type!(interrupt_provider.clone());
                let mut value_container = get_next_value!(interrupt_provider);
                match &mut value_container {
                    ValueContainer::Value(value) => {
                        // FIXME: only using type definition here, refactor and/or add checks
                        value.actual_type = Box::new(ty.type_definition);
                    }
                    _ => panic!(
                        "Expected ValueContainer::Value for type casting"
                    ),
                }
                Some(value_container)
            }

            // type(...)
            RegularInstruction::TypeExpression => {
                let ty = get_next_type!(interrupt_provider);
                Some(
                    Value {
                        inner: CoreValue::Type(ty),
                        actual_type: Box::new(TypeDefinition::Unknown), // TODO: type for type
                    }
                    .into(),
                )
            }

            i => {
                return yield Err(ExecutionError::NotImplemented(
                    format!("Instruction {i}").to_string(),
                ));
            }
        }))
    })
}
