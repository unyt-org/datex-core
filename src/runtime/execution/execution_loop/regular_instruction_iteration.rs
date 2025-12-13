use core::cell::RefCell;
use datex_core::global::protocol_structures::instructions::RegularInstruction;
use datex_core::values::value_container::ValueError;
use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator, BitwiseUnaryOperator, ComparisonOperator, ReferenceUnaryOperator, UnaryOperator};
use crate::global::protocol_structures::instructions::{ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data, IntegerData, ShortTextData, SlotAddress, TextData};
use crate::stdlib::rc::Rc;
use crate::runtime::execution::execution_loop::{ExecutionStep, InterruptProvider};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::runtime::execution::execution_loop::operations::{handle_assignment_operation, handle_binary_operation, handle_unary_operation};
use crate::runtime::execution::macros::{intercept_step, interrupt_with_result, yield_unwrap};
use crate::runtime::execution::stack::Scope;
use crate::utils::buffers::append_u32;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

/// Yield an interrupt and get the next regular instruction,
/// expecting the next input to be a NextRegularInstruction variant
macro_rules! interrupt_with_next_regular_instruction {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        let res = $input.take().unwrap();
        match res {
            InterruptProvider::NextRegularInstruction(value) => value,
            _ => unreachable!(),
        }
    }};
}

/// Drives the regular instruction iteration to get the next value
/// Returns the resolved value or None if the next instructions did not generate a value
macro_rules! get_next_maybe_value {
    ($interrupt_provider:expr) => {{
        let next = interrupt_with_next_regular_instruction!(
            $interrupt_provider,
            crate::runtime::execution::execution_loop::ExecutionStep::GetNextInstruction
        );
        let inner_iterator = next_regular_instruction_iteration($interrupt_provider, next);
        let maybe_value = intercept_step!(
            inner_iterator,
            Ok(crate::runtime::execution::execution_loop::ExecutionStep::InternalReturn(value)) => {
                value
            }
        );
        match maybe_value {
            Some(value) => value,
            _ => {
                return yield Err(ExecutionError::InvalidProgram(InvalidProgramError::ExpectedValue));
            }
        }
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
                return yield Err(ExecutionError::InvalidProgram(InvalidProgramError::ExpectedValue));
            }
        }
    }};
}

pub(crate) fn next_regular_instruction_iteration(
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    instruction: RegularInstruction,
) -> Box<impl Iterator<Item = Result<ExecutionStep, ExecutionError>>> {
    Box::new(gen move {
        yield Ok(ExecutionStep::ValueReturn(match instruction {
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
            | RegularInstruction::Divide
            | RegularInstruction::Is
            | RegularInstruction::Matches
            | RegularInstruction::StructuralEqual
            | RegularInstruction::Equal
            | RegularInstruction::NotStructuralEqual
            | RegularInstruction::NotEqual => {
                let lhs = get_next_value!(interrupt_provider);
                let rhs = get_next_value!(interrupt_provider);

                let res = handle_binary_operation(
                    BinaryOperator::from(instruction),
                    &lhs,
                    &rhs,
                );
                Ok(yield_unwrap!(res))
            }

            // unary operations
            RegularInstruction::UnaryPlus
            | RegularInstruction::UnaryMinus
            | RegularInstruction::BitwiseNot => {
                let value = get_next_value!(interrupt_provider);
                Ok(yield_unwrap!(handle_unary_operation(
                    UnaryOperator::from(instruction),
                    value,
                )))
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

                return yield Ok(ExecutionStep::InstructionBlockReturn(
                    local_slot,
                ));

                // let maybe_receivers =
                //     context.borrow_mut().scope_stack.take_active_value();
                //
                // if let Some(receivers) = maybe_receivers {
                //     interrupt_with_result!(
                //         interrupt_provider,
                //         ExecutionStep::RemoteExecution(receivers, buffer)
                //     )
                // } else {
                //     // should not happen, receivers must be set
                //     yield Err(ExecutionError::InvalidProgram(
                //         InvalidProgramError::MissingRemoteExecutionReceiver,
                //     ));
                //     None
                // }
            }

            RegularInstruction::Apply(ApplyData { arg_count }) => {
                let callee = get_next_value!(interrupt_provider);
                let mut args = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    let arg = get_next_value!(interrupt_provider);
                    args.push(arg);
                }
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::Apply(callee, args)
                )
            }

            RegularInstruction::Statements(statements_data) => {
                let mut last_value;
                for _ in 0..statements_data.statements_count {
                    last_value = get_next_maybe_value!(interrupt_provider);
                }
                match statements_data.terminated {
                    true => None,
                    false => last_value,
                }
            }

            RegularInstruction::List(list_data) => {
                let mut list = List::with_capacity(list_data.element_count);
                for _ in 0..list_data.element_count {
                    let element = get_next_value!(interrupt_provider);
                    list.push(element);
                }
                Some(list.into())
            }

            RegularInstruction::Map => {
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
                let reference = get_next_value!(interrupt_provider);
                let value_container = get_next_value!(interrupt_provider);

                // assignment value must be a reference
                if let Some(reference) = reference.maybe_reference() {
                    let lhs = reference.value_container();
                    let res = handle_assignment_operation(
                        reference,
                        value_container,
                        operator,
                    )?;
                    reference.set_value_container(res)?;
                    Some(lhs)
                } else {
                    return yield Err(ExecutionError::DerefOfNonReference);
                }
            }

            RegularInstruction::Deref => {
                let reference = get_next_value!(interrupt_provider);
                // dereferenced value must be a reference
                if let Some(reference) = reference.maybe_reference() {
                    let lhs = reference.value_container();
                    Some(lhs)
                } else {
                    return yield Err(ExecutionError::DerefOfNonReference);
                }
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

            _ => todo!("..."),
        }))
    })
}