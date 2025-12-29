pub mod interrupts;
mod operations;
pub mod state;

use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{
    AssignmentOperator, BinaryOperator, ComparisonOperator, UnaryOperator,
};
use crate::global::protocol_structures::instructions::{
    ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data,
    FloatAsInt32Data, Instruction, IntegerData, RawPointerAddress,
    RegularInstruction, ShortTextData, SlotAddress, TextData, TypeInstruction,
};
use crate::dxb_parser::body::{DXBParserError, iterate_instructions};
use crate::dxb_parser::instruction_collector::{
    CollectedResults, CollectionResultsPopper, FullOrPartialResult,
    InstructionCollector, LastUnboundedResultCollector, ResultCollector,
    StatementResultCollectionStrategy,
};
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::execution_loop::interrupts::{
    ExecutionInterrupt, ExternalExecutionInterrupt, InterruptProvider,
    InterruptResult,
};
use crate::runtime::execution::execution_loop::operations::{
    handle_assignment_operation, handle_binary_operation,
    handle_comparison_operation, handle_unary_operation,
};
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::macros::{
    interrupt, interrupt_with_maybe_value, interrupt_with_value, yield_unwrap,
};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::stdlib::boxed::Box;
use crate::stdlib::rc::Rc;
use crate::stdlib::vec::Vec;
use crate::traits::apply::Apply;
use crate::types::definition::TypeDefinition;
use crate::utils::buffers::append_u32;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::list::List;
use crate::values::core_values::map::{Map, MapKey};
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use core::cell::RefCell;

#[derive(Debug)]
enum CollectedExecutionResult {
    /// contains an optional ValueContainer that is intercepted by the consumer of a value or passed as the final result at the end of execution
    Value(Option<ValueContainer>),
    /// contains a Type that is intercepted by a consumer of a type value
    Type(Type),
    /// contains a key-value pair that is intercepted by a map construction operation
    KeyValuePair((MapKey, Option<ValueContainer>)),
}

impl From<Option<ValueContainer>> for CollectedExecutionResult {
    fn from(value: Option<ValueContainer>) -> Self {
        CollectedExecutionResult::Value(value)
    }
}
impl From<ValueContainer> for CollectedExecutionResult {
    fn from(value: ValueContainer) -> Self {
        CollectedExecutionResult::Value(Some(value))
    }
}
impl From<Type> for CollectedExecutionResult {
    fn from(value: Type) -> Self {
        CollectedExecutionResult::Type(value)
    }
}
impl From<(MapKey, Option<ValueContainer>)> for CollectedExecutionResult {
    fn from(value: (MapKey, Option<ValueContainer>)) -> Self {
        CollectedExecutionResult::KeyValuePair(value)
    }
}

impl
    CollectionResultsPopper<
        CollectedExecutionResult,
        Option<ValueContainer>,
        MapKey,
        Type,
    > for CollectedResults<CollectedExecutionResult>
{
    fn try_extract_value_result(
        result: CollectedExecutionResult,
    ) -> Option<Option<ValueContainer>> {
        match result {
            CollectedExecutionResult::Value(val) => Some(val),
            _ => None,
        }
    }

    fn try_extract_type_result(
        result: CollectedExecutionResult,
    ) -> Option<Type> {
        match result {
            CollectedExecutionResult::Type(ty) => Some(ty),
            _ => None,
        }
    }

    fn try_extract_key_value_pair_result(
        result: CollectedExecutionResult,
    ) -> Option<(MapKey, Option<ValueContainer>)> {
        match result {
            CollectedExecutionResult::KeyValuePair((key, value)) => {
                Some((key, value))
            }
            _ => None,
        }
    }
}

impl CollectedResults<CollectedExecutionResult> {
    fn collect_value_results_assert_existing(
        mut self,
    ) -> Result<Vec<ValueContainer>, ExecutionError> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(self.pop_value_result().ok_or(
                ExecutionError::InvalidProgram(
                    InvalidProgramError::ExpectedValue,
                ),
            )?);
        }
        expressions.reverse();
        Ok(expressions)
    }

    fn pop_value_result_assert_existing(
        &mut self,
    ) -> Result<ValueContainer, ExecutionError> {
        self.pop_value_result()
            .ok_or(ExecutionError::InvalidProgram(
                InvalidProgramError::ExpectedValue,
            ))
    }

    fn collect_key_value_pair_results_assert_existing(
        mut self,
    ) -> Result<Vec<(MapKey, ValueContainer)>, ExecutionError> {
        let count = self.len();
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let (key, value) = self.pop_key_value_pair_result();
            pairs.push((
                key,
                value.ok_or(ExecutionError::InvalidProgram(
                    InvalidProgramError::ExpectedValue,
                ))?,
            ));
        }
        pairs.reverse();
        Ok(pairs)
    }
}

/// Main execution loop that drives the execution of the DXB body
/// The interrupt_provider is used to provide results for synchronous or asynchronous I/O operations
pub fn execution_loop(
    state: RuntimeExecutionState,
    dxb_body: Rc<RefCell<Vec<u8>>>,
    interrupt_provider: InterruptProvider,
) -> impl Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>> {
    gen move {
        let mut slots = state.slots;
        let mut active_value: Option<ValueContainer> = None;

        for interrupt in
            inner_execution_loop(dxb_body, interrupt_provider.clone())
        {
            match interrupt {
                Ok(interrupt) => {
                    match interrupt {
                        ExecutionInterrupt::External(external_interrupt) => {
                            yield Ok(external_interrupt);
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
                                let val = yield_unwrap!(
                                    slots.get_slot_value(address)
                                );
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
                    }
                }
                Err(err) => {
                    match err {
                        ExecutionError::DXBParserError(
                            DXBParserError::ExpectingMoreInstructions,
                        ) => {
                            yield Err(
                                ExecutionError::IntermediateResultWithState(
                                    active_value.take(),
                                    None,
                                ),
                            );
                            // assume that when continuing after this yield, more instructions will have been loaded
                            // so we run the loop again to try to get the next instruction
                            continue;
                        }
                        _ => {
                            yield Err(err);
                        }
                    }
                }
            }
        }
    }
}

pub fn inner_execution_loop(
    dxb_body: Rc<RefCell<Vec<u8>>>,
    interrupt_provider: InterruptProvider,
) -> impl Iterator<Item = Result<ExecutionInterrupt, ExecutionError>> {
    gen move {
        let mut collector =
            InstructionCollector::<CollectedExecutionResult>::default();

        for instruction_result in iterate_instructions(dxb_body) {
            let instruction = match instruction_result {
                Ok(instruction) => instruction,
                Err(DXBParserError::ExpectingMoreInstructions) => {
                    yield Err(DXBParserError::ExpectingMoreInstructions.into());
                    // assume that when continuing after this yield, more instructions will have been loaded
                    // so we run the loop again to try to get the next instruction
                    continue;
                }
                Err(err) => {
                    return yield Err(err.into());
                }
            };

            let result = match instruction {
                // handle regular instructions
                Instruction::RegularInstruction(regular_instruction) => {
                    let regular_instruction = collector
                        .default_regular_instruction_collection(
                            regular_instruction,
                            StatementResultCollectionStrategy::Last,
                        );

                    let expr: Option<Option<ValueContainer>> = if let Some(
                        regular_instruction,
                    ) =
                        regular_instruction
                    {
                        Some(match regular_instruction {
                            // boolean
                            RegularInstruction::True => Some(true.into()),
                            RegularInstruction::False => Some(false.into()),

                            // integers
                            RegularInstruction::Int8(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::Int16(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::Int32(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::Int64(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::Int128(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }

                            // unsigned integers
                            RegularInstruction::UInt8(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::UInt16(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::UInt32(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::UInt64(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }
                            RegularInstruction::UInt128(integer) => {
                                Some(TypedInteger::from(integer.0).into())
                            }

                            // big integers
                            RegularInstruction::BigInteger(IntegerData(integer)) => {
                                Some(TypedInteger::Big(integer).into())
                            }

                            // default integer
                            RegularInstruction::Integer(IntegerData(i8)) => {
                                Some(i8.into())
                            }

                            // specific floats
                            RegularInstruction::DecimalF32(Float32Data(f32)) => {
                                Some(TypedDecimal::from(f32).into())
                            }
                            RegularInstruction::DecimalF64(Float64Data(f64)) => {
                                Some(TypedDecimal::from(f64).into())
                            }
                            // big decimal
                            RegularInstruction::BigDecimal(DecimalData(big_decimal)) => {
                                Some(TypedDecimal::Decimal(big_decimal).into())
                            }

                            // default decimals
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

                            RegularInstruction::GetRef(address) => Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::ResolvePointer(address)
                                    )
                                )),
                            RegularInstruction::GetLocalRef(address) => {
                                Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::ResolveLocalPointer(
                                            address
                                        )
                                    )
                                ))
                            }
                            RegularInstruction::GetInternalRef(address) => {
                                Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::ResolveInternalPointer(
                                            address
                                        )
                                    )
                                ))
                            }

                            RegularInstruction::GetSlot(SlotAddress(address)) => {
                                Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::GetSlotValue(address)
                                ))
                            }

                            RegularInstruction::DropSlot(SlotAddress(address)) => {
                                interrupt!(
                                    interrupt_provider,
                                    ExecutionInterrupt::DropSlot(address)
                                );
                                None
                            }

                            // NOTE: make sure that each possible match case is either implemented in the default collection or here
                            // If an instruction is implemented in the default collection, it should be marked as unreachable!() here
                            RegularInstruction::Statements(_) |
                            RegularInstruction::ShortStatements(_) |
                            RegularInstruction::UnboundedStatements |
                            RegularInstruction::UnboundedStatementsEnd(_) |
                            RegularInstruction::List(_) |
                            RegularInstruction::ShortList(_)  |
                            RegularInstruction::Map(_) |
                            RegularInstruction::ShortMap(_) |
                            RegularInstruction::KeyValueDynamic |
                            RegularInstruction::KeyValueShortText(_) |
                            RegularInstruction::Add |
                            RegularInstruction::Subtract |
                            RegularInstruction::Multiply |
                            RegularInstruction::Divide |
                            RegularInstruction::UnaryMinus |
                            RegularInstruction::UnaryPlus |
                            RegularInstruction::BitwiseNot |
                            RegularInstruction::Apply(_) |
                            RegularInstruction::Is |
                            RegularInstruction::Matches |
                            RegularInstruction::StructuralEqual |
                            RegularInstruction::Equal |
                            RegularInstruction::NotStructuralEqual |
                            RegularInstruction::NotEqual |
                            RegularInstruction::AddAssign(_) |
                            RegularInstruction::SubtractAssign(_) |
                            RegularInstruction::MultiplyAssign(_) |
                            RegularInstruction::DivideAssign(_) |
                            RegularInstruction::CreateRef |
                            RegularInstruction::CreateRefMut |
                            RegularInstruction::GetOrCreateRef(_) |
                            RegularInstruction::GetOrCreateRefMut(_) |
                            RegularInstruction::AllocateSlot(_) |
                            RegularInstruction::SetSlot(_) |
                            RegularInstruction::SetReferenceValue(_) |
                            RegularInstruction::Deref |
                            RegularInstruction::TypedValue |
                            RegularInstruction::RemoteExecution(_) |
                            RegularInstruction::TypeExpression => unreachable!()
                        })
                    } else {
                        None
                    };

                    expr.map(|expr| CollectedExecutionResult::from(expr))
                }
                Instruction::TypeInstruction(type_instruction) => {
                    let type_instruction = collector
                        .default_type_instruction_collection(type_instruction);

                    let type_expression: Option<Type> = if let Some(
                        type_instruction,
                    ) = type_instruction
                    {
                        Some(match type_instruction {
                            TypeInstruction::LiteralInteger(integer) => {
                                Type::structural(integer.0)
                            }
                            TypeInstruction::LiteralText(text_data) => {
                                Type::structural(text_data.0)
                            }

                            TypeInstruction::TypeReference(type_ref) => {
                                let metadata = type_ref.metadata;
                                let val = interrupt_with_maybe_value!(
                                    interrupt_provider,
                                    match type_ref.address {
                                        RawPointerAddress::Local(address) => {
                                            ExecutionInterrupt::External(
                                                ExternalExecutionInterrupt::ResolveLocalPointer(
                                                    address,
                                                ),
                                            )
                                        }
                                        RawPointerAddress::Internal(
                                            address,
                                        ) => {
                                            ExecutionInterrupt::External(ExternalExecutionInterrupt::ResolveInternalPointer(address))
                                        }
                                        RawPointerAddress::Full(address) => {
                                            ExecutionInterrupt::External(
                                                ExternalExecutionInterrupt::ResolvePointer(
                                                    address,
                                                ),
                                            )
                                        }
                                    }
                                );

                                match val {
                                    // simple Type value
                                    Some(ValueContainer::Value(Value {
                                        inner: CoreValue::Type(ty),
                                        ..
                                    })) => ty,
                                    // Type Reference
                                    Some(ValueContainer::Reference(
                                        Reference::TypeReference(type_ref),
                                    )) => Type::new(
                                        TypeDefinition::Reference(type_ref),
                                        metadata.mutability.into(),
                                    ),
                                    _ => {
                                        return yield Err(
                                            ExecutionError::ExpectedTypeValue,
                                        );
                                    }
                                }
                            }

                            // NOTE: make sure that each possible match case is either implemented in the default collection or here
                            // If an instruction is implemented in the default collection, it should be marked as unreachable!() here
                            TypeInstruction::List(_)
                            | TypeInstruction::ImplType(_) => unreachable!(),
                        })
                    } else {
                        None
                    };

                    type_expression
                        .map(|ty_expr| CollectedExecutionResult::from(ty_expr))
                }
            };

            if let Some(result) = result {
                collector.push_result(result);
            }

            // handle collecting nested expressions
            while let Some(result) = collector.try_pop_collected() {
                let expr: CollectedExecutionResult = match result {
                    FullOrPartialResult::Full(
                        instruction,
                        mut collected_results,
                    ) => {
                        match instruction {
                            Instruction::RegularInstruction(
                                regular_instruction,
                            ) => match regular_instruction {
                                RegularInstruction::List(_)
                                | RegularInstruction::ShortList(_) => {
                                    let elements = yield_unwrap!(collected_results.collect_value_results_assert_existing());
                                    ValueContainer::from(List::new(elements))
                                        .into()
                                }
                                RegularInstruction::Map(_)
                                | RegularInstruction::ShortMap(_) => {
                                    let entries = yield_unwrap!(collected_results.collect_key_value_pair_results_assert_existing());
                                    ValueContainer::from(Map::from(entries))
                                        .into()
                                }

                                RegularInstruction::KeyValueDynamic => {
                                    let value =
                                        collected_results.pop_value_result();
                                    let key = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    CollectedExecutionResult::KeyValuePair((
                                        MapKey::Value(key),
                                        value,
                                    ))
                                }

                                RegularInstruction::KeyValueShortText(
                                    short_text_data,
                                ) => {
                                    let value =
                                        collected_results.pop_value_result();
                                    let key = MapKey::Text(short_text_data.0);
                                    CollectedExecutionResult::KeyValuePair((
                                        key, value,
                                    ))
                                }

                                RegularInstruction::Add
                                | RegularInstruction::Subtract
                                | RegularInstruction::Multiply
                                | RegularInstruction::Divide => {
                                    let right = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    let left = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );

                                    let res = handle_binary_operation(
                                        BinaryOperator::from(
                                            regular_instruction,
                                        ),
                                        &left,
                                        &right,
                                    );
                                    yield_unwrap!(res).into()
                                }

                                RegularInstruction::Is
                                | RegularInstruction::StructuralEqual
                                | RegularInstruction::Equal
                                | RegularInstruction::NotStructuralEqual
                                | RegularInstruction::NotEqual => {
                                    let right = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    let left = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );

                                    let res = handle_comparison_operation(
                                        ComparisonOperator::from(
                                            regular_instruction,
                                        ),
                                        &left,
                                        &right,
                                    );
                                    yield_unwrap!(res).into()
                                }

                                RegularInstruction::Matches => {
                                    let val = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    let type_pattern =
                                        collected_results.pop_type_result();

                                    todo!()
                                }

                                RegularInstruction::UnaryMinus
                                | RegularInstruction::UnaryPlus
                                | RegularInstruction::BitwiseNot
                                | RegularInstruction::CreateRef
                                | RegularInstruction::CreateRefMut
                                | RegularInstruction::Deref => {
                                    let expr = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    yield_unwrap!(handle_unary_operation(
                                        UnaryOperator::from(
                                            regular_instruction
                                        ),
                                        expr,
                                    ))
                                    .into()
                                }

                                RegularInstruction::TypedValue => {
                                    let mut value_container = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    let ty =
                                        collected_results.pop_type_result();

                                    match &mut value_container {
                                        ValueContainer::Value(value) => {
                                            // FIXME: only using type definition here, refactor and/or add checks
                                            value.actual_type =
                                                Box::new(ty.type_definition);
                                        }
                                        _ => panic!(
                                            "Expected ValueContainer::Value for type casting"
                                        ),
                                    }
                                    value_container.into()
                                }

                                // type(...)
                                RegularInstruction::TypeExpression => {
                                    let ty =
                                        collected_results.pop_type_result();
                                    ValueContainer::Value(Value {
                                        inner: CoreValue::Type(ty),
                                        actual_type: Box::new(
                                            TypeDefinition::Unknown,
                                        ), // TODO: type for type
                                    })
                                    .into()
                                }

                                RegularInstruction::AddAssign(SlotAddress(
                                    address,
                                ))
                                | RegularInstruction::MultiplyAssign(
                                    SlotAddress(address),
                                )
                                | RegularInstruction::DivideAssign(
                                    SlotAddress(address),
                                )
                                | RegularInstruction::SubtractAssign(
                                    SlotAddress(address),
                                ) => {
                                    let slot_value = interrupt_with_value!(
                                        interrupt_provider,
                                        ExecutionInterrupt::GetSlotValue(
                                            address
                                        )
                                    );
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );

                                    let new_val = yield_unwrap!(
                                        handle_assignment_operation(
                                            AssignmentOperator::from(
                                                regular_instruction
                                            ),
                                            slot_value,
                                            value,
                                        )
                                    );
                                    // set slot value
                                    interrupt!(
                                        interrupt_provider,
                                        ExecutionInterrupt::SetSlotValue(
                                            address,
                                            new_val.clone()
                                        )
                                    );
                                    // return assigned value
                                    new_val.into()
                                }

                                RegularInstruction::SetReferenceValue(
                                    operator,
                                ) => {
                                    let value_container = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    let ref_value_container = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );

                                    // assignment value must be a reference
                                    if let Some(reference) =
                                        ref_value_container.maybe_reference()
                                    {
                                        let lhs = reference.value_container();
                                        let res = yield_unwrap!(
                                            handle_assignment_operation(
                                                operator,
                                                lhs.clone(),
                                                value_container,
                                            )
                                        );
                                        yield_unwrap!(
                                            reference.set_value_container(res)
                                        );
                                        ref_value_container.into()
                                    } else {
                                        return yield Err(
                                            ExecutionError::DerefOfNonReference,
                                        );
                                    }
                                }

                                RegularInstruction::SetSlot(SlotAddress(
                                    address,
                                )) => {
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    interrupt!(
                                        interrupt_provider,
                                        ExecutionInterrupt::SetSlotValue(
                                            address,
                                            value.clone()
                                        )
                                    );
                                    value.into()
                                }

                                RegularInstruction::AllocateSlot(
                                    SlotAddress(address),
                                ) => {
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );
                                    interrupt!(
                                        interrupt_provider,
                                        ExecutionInterrupt::AllocateSlot(
                                            address,
                                            value.clone()
                                        )
                                    );
                                    value.into()
                                }

                                RegularInstruction::RemoteExecution(
                                    exec_block_data,
                                ) => {
                                    // build dxb
                                    let mut buffer = Vec::with_capacity(256);
                                    for (addr, local_slot) in exec_block_data
                                        .injected_slots
                                        .into_iter()
                                        .enumerate()
                                    {
                                        buffer.push(
                                            InstructionCode::ALLOCATE_SLOT
                                                as u8,
                                        );
                                        append_u32(&mut buffer, addr as u32);

                                        let slot_value = interrupt_with_value!(
                                            interrupt_provider,
                                            ExecutionInterrupt::GetSlotValue(
                                                local_slot
                                            )
                                        );
                                        buffer.extend_from_slice(
                                            &compile_value_container(
                                                &slot_value,
                                            ),
                                        );
                                    }
                                    buffer.extend_from_slice(
                                        &exec_block_data.body,
                                    );

                                    let receivers = yield_unwrap!(
                                        collected_results
                                            .pop_value_result_assert_existing()
                                    );

                                    interrupt_with_maybe_value!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::RemoteExecution(
                                                receivers, buffer
                                            )
                                        )
                                    ).into()
                                }

                                RegularInstruction::Apply(ApplyData {
                                    ..
                                }) => {
                                    let mut args = yield_unwrap!(collected_results.collect_value_results_assert_existing());
                                    let callee = args.remove(0);
                                    interrupt_with_maybe_value!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::Apply(
                                                callee, args
                                            )
                                        )
                                    )
                                    .into()
                                }

                                RegularInstruction::UnboundedStatementsEnd(
                                    terminated,
                                ) => {
                                    let result = yield_unwrap!(collector.try_pop_unbounded().ok_or(DXBParserError::NotInUnboundedRegularScopeError));
                                    if let FullOrPartialResult::Partial(
                                        _,
                                        mut collected_result,
                                    ) = result
                                    {
                                        if terminated {
                                            CollectedExecutionResult::Value(
                                                None,
                                            )
                                        } else {
                                            match collected_result {
                                                Some(CollectedExecutionResult::Value(val)) => val.into(),
                                                None => CollectedExecutionResult::Value(None),
                                                _ => unreachable!(),
                                            }
                                        }
                                    } else {
                                        unreachable!()
                                    }
                                }

                                e => {
                                    todo!(
                                        "Unhandled collected regular instruction: {:?}",
                                        e
                                    );
                                }
                            },

                            Instruction::TypeInstruction(type_instruction) => {
                                match type_instruction {
                                    TypeInstruction::ImplType(
                                        impl_type_data,
                                    ) => {
                                        let mutability: Option<
                                            ReferenceMutability,
                                        > = impl_type_data
                                            .metadata
                                            .mutability
                                            .into();
                                        let base_type =
                                            collected_results.pop_type_result();
                                        Type::new(
                                            TypeDefinition::ImplType(
                                                Box::new(base_type),
                                                impl_type_data
                                                    .impls
                                                    .iter()
                                                    .map(PointerAddress::from)
                                                    .collect(),
                                            ),
                                            mutability.clone(),
                                        )
                                        .into()
                                    }
                                    _ => todo!(),
                                }
                            }
                        }
                    }
                    FullOrPartialResult::Partial(
                        instruction,
                        collected_result,
                    ) => match instruction {
                        Instruction::RegularInstruction(
                            regular_instruction,
                        ) => match regular_instruction {
                            RegularInstruction::Statements(statements_data) => {
                                if statements_data.terminated {
                                    CollectedExecutionResult::Value(None)
                                } else {
                                    match collected_result {
                                        Some(
                                            CollectedExecutionResult::Value(
                                                val,
                                            ),
                                        ) => val.into(),
                                        None => {
                                            CollectedExecutionResult::Value(
                                                None,
                                            )
                                        }
                                        _ => unreachable!(),
                                    }
                                }
                            }
                            _ => unreachable!(),
                        },

                        Instruction::TypeInstruction(data) => unreachable!(),
                    },
                };

                collector.push_result(expr);
            }

            // if in unbounded statements, propagate active value via interrupt
            if let Some(ResultCollector::LastUnbounded(
                LastUnboundedResultCollector {
                    last_result:
                        Some(CollectedExecutionResult::Value(last_result)),
                    ..
                },
            )) = collector.last()
            {
                interrupt!(
                    interrupt_provider,
                    ExecutionInterrupt::SetActiveValue(last_result.clone())
                );
            }
        }

        if let Some(result) = collector.take_root_result() {
            yield Ok(ExecutionInterrupt::External(
                ExternalExecutionInterrupt::Result(match result {
                    CollectedExecutionResult::Value(value) => value,
                    _ => unreachable!("Expected root result"),
                }),
            ));
        } else {
            panic!("Execution finished without root result");
        }
    }
}
