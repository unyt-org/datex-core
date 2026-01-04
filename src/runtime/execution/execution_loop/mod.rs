pub mod interrupts;
mod operations;
mod runtime_value;
mod slots;
pub mod state;

use crate::core_compiler::value_compiler::compile_value_container;
use crate::dxb_parser::body::{DXBParserError, iterate_instructions};
use crate::dxb_parser::instruction_collector::{
    CollectedResults, CollectionResultsPopper, FullOrPartialResult,
    InstructionCollector, LastUnboundedResultCollector, ResultCollector,
    StatementResultCollectionStrategy,
};
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::{
    AssignmentOperator, BinaryOperator, ComparisonOperator, UnaryOperator,
};
use crate::global::protocol_structures::instructions::{
    ApplyData, DecimalData, Float32Data, Float64Data, FloatAsInt16Data,
    FloatAsInt32Data, Instruction, IntegerData, RawPointerAddress,
    RegularInstruction, ShortTextData, SlotAddress, TextData, TypeInstruction,
};
use crate::references::reference::{Reference, ReferenceMutability};
use crate::runtime::execution::execution_loop::interrupts::{
    ExecutionInterrupt, ExternalExecutionInterrupt, InterruptProvider,
    InterruptResult,
};
use crate::runtime::execution::execution_loop::operations::{
    handle_assignment_operation, handle_binary_operation,
    handle_comparison_operation, handle_unary_operation, set_property,
};
use crate::runtime::execution::execution_loop::runtime_value::RuntimeValue;
use crate::runtime::execution::execution_loop::slots::get_internal_slot_value;
use crate::runtime::execution::execution_loop::state::RuntimeExecutionState;
use crate::runtime::execution::macros::{
    interrupt, interrupt_with_maybe_value, interrupt_with_value, yield_unwrap,
};
use crate::runtime::execution::{ExecutionError, InvalidProgramError};
use crate::stdlib::boxed::Box;
use crate::stdlib::rc::Rc;
use crate::stdlib::vec::Vec;
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
use crate::values::value_container::{OwnedValueKey, ValueContainer};
use core::cell::RefCell;
use datex_core::runtime::execution::execution_loop::slots::get_slot_value;

#[derive(Debug)]
enum CollectedExecutionResult {
    /// contains an optional runtime value that is intercepted by the consumer of a value or passed as the final result at the end of execution
    Value(Option<RuntimeValue>),
    /// contains a Type that is intercepted by a consumer of a type value
    Type(Type),
    /// contains a key-value pair that is intercepted by a map construction operation
    KeyValuePair((MapKey, ValueContainer)),
}

impl From<Option<RuntimeValue>> for CollectedExecutionResult {
    fn from(value: Option<RuntimeValue>) -> Self {
        CollectedExecutionResult::Value(value)
    }
}
impl From<RuntimeValue> for CollectedExecutionResult {
    fn from(value: RuntimeValue) -> Self {
        CollectedExecutionResult::Value(Some(value))
    }
}
impl From<Type> for CollectedExecutionResult {
    fn from(value: Type) -> Self {
        CollectedExecutionResult::Type(value)
    }
}
impl From<(MapKey, ValueContainer)> for CollectedExecutionResult {
    fn from(value: (MapKey, ValueContainer)) -> Self {
        CollectedExecutionResult::KeyValuePair(value)
    }
}

impl
    CollectionResultsPopper<
        CollectedExecutionResult,
        Option<RuntimeValue>,
        MapKey,
        ValueContainer,
        Type,
    > for CollectedResults<CollectedExecutionResult>
{
    fn try_extract_value_result(
        result: CollectedExecutionResult,
    ) -> Option<Option<RuntimeValue>> {
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
    ) -> Option<(MapKey, ValueContainer)> {
        match result {
            CollectedExecutionResult::KeyValuePair((key, value)) => {
                Some((key, value))
            }
            _ => None,
        }
    }
}

impl CollectedResults<CollectedExecutionResult> {
    fn collect_value_container_results_assert_existing(
        mut self,
        state: &RuntimeExecutionState,
    ) -> Result<Vec<ValueContainer>, ExecutionError> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(
                self.pop_cloned_value_container_result_assert_existing(state)?,
            );
        }
        expressions.reverse();
        Ok(expressions)
    }

    /// Pops a runtime value result, returning an error if none exists
    fn pop_runtime_value_result_assert_existing(
        &mut self,
    ) -> Result<RuntimeValue, ExecutionError> {
        self.pop_value_result()
            .ok_or(ExecutionError::InvalidProgram(
                InvalidProgramError::ExpectedValue,
            ))
    }

    /// Pops a value container result, returning an error if none exists.
    /// If the value is a slot address, it is resolved to a cloned value container.
    /// Do not use this method if you want to work on the actual value without cloning it.
    fn pop_cloned_value_container_result_assert_existing(
        &mut self,
        state: &RuntimeExecutionState,
    ) -> Result<ValueContainer, ExecutionError> {
        self.pop_runtime_value_result_assert_existing()?
            .into_cloned_value_container(state)
    }

    fn collect_key_value_pair_results_assert_existing(
        mut self,
    ) -> Result<Vec<(MapKey, ValueContainer)>, ExecutionError> {
        let count = self.len();
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let (key, value) = self.pop_key_value_pair_result();
            pairs.push((key, value));
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
        let mut active_value: Option<ValueContainer> = None;

        for interrupt in
            inner_execution_loop(dxb_body, interrupt_provider.clone(), state)
        {
            match interrupt {
                Ok(interrupt) => match interrupt {
                    ExecutionInterrupt::External(external_interrupt) => {
                        yield Ok(external_interrupt);
                    }
                    ExecutionInterrupt::SetActiveValue(value) => {
                        active_value = value;
                    }
                },
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
    mut state: RuntimeExecutionState,
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

                    let expr: Option<Option<RuntimeValue>> = if let Some(
                        regular_instruction,
                    ) =
                        regular_instruction
                    {
                        Some(match regular_instruction {
                            // boolean
                            RegularInstruction::True => Some(ValueContainer::from(true).into()),
                            RegularInstruction::False => Some(ValueContainer::from(false).into()),

                            // integers
                            RegularInstruction::Int8(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::Int16(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::Int32(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::Int64(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::Int128(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }

                            // unsigned integers
                            RegularInstruction::UInt8(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::UInt16(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::UInt32(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::UInt64(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }
                            RegularInstruction::UInt128(integer) => {
                                Some(ValueContainer::from(TypedInteger::from(integer.0)).into())
                            }

                            // big integers
                            RegularInstruction::BigInteger(IntegerData(integer)) => {
                                Some(ValueContainer::from(TypedInteger::IBig(integer)).into())
                            }

                            // default integer
                            RegularInstruction::Integer(IntegerData(i8)) => {
                                Some(ValueContainer::from(i8).into())
                            }

                            // specific floats
                            RegularInstruction::DecimalF32(Float32Data(f32)) => {
                                Some(ValueContainer::from(TypedDecimal::from(f32)).into())
                            }
                            RegularInstruction::DecimalF64(Float64Data(f64)) => {
                                Some(ValueContainer::from(TypedDecimal::from(f64)).into())
                            }
                            // big decimal
                            RegularInstruction::BigDecimal(DecimalData(big_decimal)) => {
                                Some(ValueContainer::from(TypedDecimal::Decimal(big_decimal)).into())
                            }

                            // default decimals
                            RegularInstruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                                Some(ValueContainer::from(Decimal::from(i16 as f32)).into())
                            }
                            RegularInstruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                                Some(ValueContainer::from(Decimal::from(i32 as f32)).into())
                            }
                            RegularInstruction::Decimal(DecimalData(big_decimal)) => {
                                Some(ValueContainer::from(big_decimal).into())
                            }

                            // endpoint
                            RegularInstruction::Endpoint(endpoint) => Some(ValueContainer::from(endpoint).into()),

                            // null
                            RegularInstruction::Null => Some(ValueContainer::from(Value::null()).into()),

                            // text
                            RegularInstruction::ShortText(ShortTextData(text)) => {
                                Some(ValueContainer::from(text).into())
                            }
                            RegularInstruction::Text(TextData(text)) => Some(ValueContainer::from(text).into()),

                            RegularInstruction::GetRef(address) => Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::ResolvePointer(address)
                                    )
                                ).into()),
                            RegularInstruction::GetLocalRef(address) => {
                                Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::ResolveLocalPointer(
                                            address
                                        )
                                    )
                                ).into())
                            }
                            RegularInstruction::GetInternalRef(address) => {
                                Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::ResolveInternalPointer(
                                            address
                                        )
                                    )
                                ).into())
                            }

                            RegularInstruction::GetSlot(SlotAddress(address)) => {
                                Some(RuntimeValue::SlotAddress(address))
                            }

                            RegularInstruction::GetInternalSlot(SlotAddress(address)) => {
                                Some(RuntimeValue::ValueContainer(yield_unwrap!(
                                    get_internal_slot_value(
                                        &state,
                                        address,
                                    )
                                )))
                            }


                            RegularInstruction::DropSlot(SlotAddress(address)) => {
                                yield_unwrap!(state.slots.drop_slot(address));
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
                            RegularInstruction::GetPropertyText(_) |
                            RegularInstruction::GetPropertyIndex(_) |
                            RegularInstruction::GetPropertyDynamic |
                            RegularInstruction::SetPropertyText(_) |
                            RegularInstruction::SetPropertyIndex(_) |
                            RegularInstruction::SetPropertyDynamic |
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

                    expr.map(CollectedExecutionResult::from)
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
                                    let elements = yield_unwrap!(collected_results.collect_value_container_results_assert_existing(&state));
                                    RuntimeValue::ValueContainer(
                                        ValueContainer::from(List::new(
                                            elements,
                                        )),
                                    )
                                    .into()
                                }
                                RegularInstruction::Map(_)
                                | RegularInstruction::ShortMap(_) => {
                                    let entries = yield_unwrap!(collected_results.collect_key_value_pair_results_assert_existing());
                                    RuntimeValue::ValueContainer(
                                        ValueContainer::from(Map::from(
                                            entries,
                                        )),
                                    )
                                    .into()
                                }

                                RegularInstruction::KeyValueDynamic => {
                                    let value = yield_unwrap!(
                                        collected_results.pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let key = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    CollectedExecutionResult::KeyValuePair((
                                        MapKey::Value(key),
                                        value,
                                    ))
                                }

                                RegularInstruction::KeyValueShortText(
                                    short_text_data,
                                ) => {
                                    let value = yield_unwrap!(
                                        collected_results.pop_cloned_value_container_result_assert_existing(&state)
                                    );
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
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let left = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );

                                    let res = handle_binary_operation(
                                        BinaryOperator::from(
                                            regular_instruction,
                                        ),
                                        &left,
                                        &right,
                                    );
                                    RuntimeValue::ValueContainer(yield_unwrap!(
                                        res
                                    ))
                                    .into()
                                }

                                RegularInstruction::Is
                                | RegularInstruction::StructuralEqual
                                | RegularInstruction::Equal
                                | RegularInstruction::NotStructuralEqual
                                | RegularInstruction::NotEqual => {
                                    let right = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let left = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );

                                    let res = handle_comparison_operation(
                                        ComparisonOperator::from(
                                            regular_instruction,
                                        ),
                                        &left,
                                        &right,
                                    );
                                    RuntimeValue::ValueContainer(yield_unwrap!(
                                        res
                                    ))
                                    .into()
                                }

                                RegularInstruction::Matches => {
                                    let target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let type_pattern =
                                        collected_results.pop_type_result();

                                    todo!("#645 Undescribed by author.")
                                }

                                RegularInstruction::UnaryMinus
                                | RegularInstruction::UnaryPlus
                                | RegularInstruction::BitwiseNot
                                | RegularInstruction::CreateRef
                                | RegularInstruction::CreateRefMut
                                | RegularInstruction::Deref => {
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| {
                                            handle_unary_operation(
                                                UnaryOperator::from(
                                                    regular_instruction,
                                                ),
                                                target.clone(), // TODO #646: is unary operation supposed to take ownership?
                                            )
                                        },
                                    );
                                    RuntimeValue::ValueContainer(yield_unwrap!(
                                        yield_unwrap!(res)
                                    ))
                                    .into()
                                }

                                RegularInstruction::TypedValue => {
                                    let mut value_container = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let ty =
                                        collected_results.pop_type_result();

                                    match &mut value_container {
                                        ValueContainer::Value(value) => {
                                            // FIXME #647: only using type definition here, refactor and/or add checks
                                            value.actual_type =
                                                Box::new(ty.type_definition);
                                        }
                                        _ => panic!(
                                            "Expected ValueContainer::Value for type casting"
                                        ),
                                    }
                                    RuntimeValue::ValueContainer(
                                        value_container,
                                    )
                                    .into()
                                }

                                // type(...)
                                RegularInstruction::TypeExpression => {
                                    let ty =
                                        collected_results.pop_type_result();
                                    RuntimeValue::ValueContainer(
                                        ValueContainer::Value(Value {
                                            inner: CoreValue::Type(ty),
                                            actual_type: Box::new(
                                                TypeDefinition::Unknown,
                                            ), // TODO #648: type for type
                                        }),
                                    )
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
                                    let slot_value = yield_unwrap!(
                                        get_slot_value(&state, address)
                                    );
                                    let value = yield_unwrap!(
                                            collected_results
                                                .pop_cloned_value_container_result_assert_existing(&state)
                                        );

                                    let new_val = yield_unwrap!(
                                        handle_assignment_operation(
                                            AssignmentOperator::from(
                                                regular_instruction
                                            ),
                                            &slot_value,
                                            value,
                                        )
                                    );
                                    yield_unwrap!(
                                        state
                                            .slots
                                            .set_slot_value(address, new_val)
                                    );
                                    None.into()
                                }

                                RegularInstruction::SetReferenceValue(
                                    operator,
                                ) => {
                                    let value_container = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let ref_value_container = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );

                                    // assignment value must be a reference
                                    if let Some(reference) =
                                        ref_value_container.maybe_reference()
                                    {
                                        let lhs = reference.value_container();
                                        let res = yield_unwrap!(
                                            handle_assignment_operation(
                                                operator,
                                                &lhs,
                                                value_container,
                                            )
                                        );
                                        yield_unwrap!(
                                            reference.set_value_container(res)
                                        );
                                        RuntimeValue::ValueContainer(
                                            ref_value_container,
                                        )
                                        .into()
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
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    yield_unwrap!(
                                        state
                                            .slots
                                            .set_slot_value(address, value)
                                    );
                                    None.into()
                                }

                                RegularInstruction::AllocateSlot(
                                    SlotAddress(address),
                                ) => {
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    state
                                        .slots
                                        .allocate_slot(address, Some(value));

                                    None.into()
                                }

                                RegularInstruction::GetPropertyText(
                                    property_data,
                                ) => {
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let property_name = property_data.0;

                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| {
                                            target.try_get_property(
                                                &property_name,
                                            )
                                        },
                                    );
                                    RuntimeValue::ValueContainer(yield_unwrap!(
                                        yield_unwrap!(res)
                                    ))
                                    .into()
                                }

                                RegularInstruction::GetPropertyIndex(
                                    property_data,
                                ) => {
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let property_index = property_data.0;

                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| {
                                            target.try_get_property(
                                                property_index,
                                            )
                                        },
                                    );
                                    RuntimeValue::ValueContainer(yield_unwrap!(
                                        yield_unwrap!(res)
                                    ))
                                    .into()
                                }

                                RegularInstruction::GetPropertyDynamic => {
                                    let key = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );

                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| target.try_get_property(&key),
                                    );
                                    RuntimeValue::ValueContainer(yield_unwrap!(
                                        yield_unwrap!(res)
                                    ))
                                    .into()
                                }

                                RegularInstruction::SetPropertyText(
                                    property_data,
                                ) => {
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let runtime_internal =
                                        state.runtime_internal.clone();
                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| {
                                            set_property(
                                                &runtime_internal,
                                                target,
                                                OwnedValueKey::Text(
                                                    property_data.0,
                                                ),
                                                value,
                                            )
                                        },
                                    );
                                    yield_unwrap!(yield_unwrap!(res));
                                    None.into()
                                }

                                RegularInstruction::SetPropertyIndex(
                                    property_data,
                                ) => {
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );

                                    let runtime_internal =
                                        state.runtime_internal.clone();
                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| {
                                            set_property(
                                                &runtime_internal,
                                                target,
                                                OwnedValueKey::Index(
                                                    property_data.0 as i64,
                                                ),
                                                value,
                                            )
                                        },
                                    );
                                    yield_unwrap!(yield_unwrap!(res));
                                    None.into()
                                }

                                RegularInstruction::SetPropertyDynamic => {
                                    let mut target = yield_unwrap!(
                                        collected_results
                                            .pop_runtime_value_result_assert_existing()
                                    );
                                    let value = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );
                                    let key = yield_unwrap!(
                                        collected_results
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );

                                    let runtime_internal =
                                        state.runtime_internal.clone();
                                    let res = target.with_mut_value_container(
                                        &mut state,
                                        |target| {
                                            set_property(
                                                &runtime_internal,
                                                target,
                                                OwnedValueKey::Value(key),
                                                value,
                                            )
                                        },
                                    );
                                    yield_unwrap!(yield_unwrap!(res));
                                    None.into()
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

                                        let slot_value = yield_unwrap!(
                                            get_slot_value(&state, local_slot,)
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
                                            .pop_cloned_value_container_result_assert_existing(&state)
                                    );

                                    interrupt_with_maybe_value!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::RemoteExecution(
                                                receivers, buffer
                                            )
                                        )
                                    )
                                        .map(|val| RuntimeValue::ValueContainer(val))
                                        .into()
                                }

                                RegularInstruction::Apply(ApplyData {
                                    ..
                                }) => {
                                    let mut args = yield_unwrap!(collected_results.collect_value_container_results_assert_existing(&state));
                                    // last argument is the callee
                                    let callee = args.remove(args.len() - 1);
                                    interrupt_with_maybe_value!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::Apply(
                                                callee, args
                                            )
                                        )
                                    )
                                    .map(|val| {
                                        RuntimeValue::ValueContainer(val)
                                    })
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
                                    _ => todo!("#649 Undescribed by author."),
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
                let active_value = yield_unwrap!(
                    last_result
                        .clone()
                        .map(|v| v.into_cloned_value_container(&state))
                        .transpose()
                );

                interrupt!(
                    interrupt_provider,
                    ExecutionInterrupt::SetActiveValue(active_value)
                );
            }
        }

        if let Some(result) = collector.take_root_result() {
            yield Ok(ExecutionInterrupt::External(
                ExternalExecutionInterrupt::Result(match result {
                    CollectedExecutionResult::Value(value) => {
                        let value = yield_unwrap!(
                            value
                                .map(|v| v.into_cloned_value_container(&state))
                                .transpose()
                        );

                        value
                    }
                    _ => unreachable!("Expected root result"),
                }),
            ));
        } else {
            panic!("Execution finished without root result");
        }
    }
}
