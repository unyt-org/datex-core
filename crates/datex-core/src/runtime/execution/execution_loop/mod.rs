//! This module contains the implementation of the execution loop that drives the execution of the compiled DATEX bytecode (DXB).
//! It handles the execution of instructions, manages the runtime state, and processes interrupts that can occur during execution.
mod implementation;
use implementation::*;
mod execution_result_popper;
mod internal_slots;
pub mod interrupts;
mod runtime_value;
pub mod state;
use crate::{
    core_compiler::{
        core_compilation_context::{CompileInput, DXBWithSharedValues},
        injected_values::compile_injected_values,
    },
    dxb_parser::{
        body::{DXBParserError, SeekRequest, iterate_instructions_with_seek},
        instruction_collector::{
            CollectionResultsPopper, FullOrPartialResult, InstructionCollector,
            LastUnboundedResultCollector, ResultCollector,
            StatementResultCollectionStrategy,
        },
    },
    global::operators::{BinaryOperator, ComparisonOperator, UnaryOperator},
    instruction::{
        Instruction, NestedInstructionResolutionStrategy,
        instruction_data::{
            ApplyData, Float32Data, Float64Data, FloatAsInt16Data,
            FloatAsInt32Data, InstantData, IntersectionData, JumpData,
            ListSliceCollectionData, MapData, ShortStatementsData,
            ShortTextData, StatementsData, TaggedTypeData, TaggedValue,
            TextData, UnboundedStatementsData, UnionData,
        },
        regular_instruction::RegularInstruction,
        type_instruction::TypeInstruction,
    },
    libs::core::type_id::CoreLibBaseTypeId,
    prelude::*,
    runtime::{
        Runtime,
        cache::shared_values_cache::{
            CacheValueRetrievalError, ValueNotFoundInCacheError,
        },
        execution::{
            ExecutionError, InvalidProgramError,
            execution_loop::{
                internal_slots::get_root_property,
                interrupts::{
                    ExecutionInterrupt, ExternalExecutionInterrupt,
                    InterruptProvider, InterruptResult,
                },
                runtime_value::RuntimeValue,
                state::RuntimeExecutionState,
            },
            macros::{
                interrupt, interrupt_with_maybe_value, interrupt_with_value,
            },
        },
    },
    shared_values::{
        PointerAddress, ReferenceMutability, ReferencedSharedContainer,
        RemotePointerAddress, SharedContainer, SharedContainerMutability,
        SharedContainerOwnership,
        base_shared_value_container::BaseSharedValueContainer,
    },
    types::{
        r#type::Type,
        type_definition::{
            TypeDefinition,
            collection::{
                CollectionTypeDefinition,
                type_definition::{
                    list::ListCollectionTypeDefinition,
                    list_slice::ListSliceCollectionTypeDefinition,
                    map::MapCollectionTypeDefinition,
                },
            },
            impl_type::ImplMarkers,
            intersection::IntersectionTypeDefinition,
            map::MapTypeDefinition,
            range::RangeTypeDefinition,
            tagged_type::TaggedTypeDefinition,
            union::UnionTypeDefinition,
        },
        type_definition_with_metadata::TypeDefinitionWithMetadata,
    },
    value_updates::{
        update_data::{AppendEntryUpdateData, DeleteEntryUpdateData},
        update_handler::UpdateHandler,
    },
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        core_values::{
            boolean::Boolean,
            callable::{Callable, CallableBody, CoreStub},
            decimal::{Decimal, typed_decimal::TypedDecimal},
            endpoint::Endpoint,
            integer::{Integer, typed_integer::TypedInteger},
            list::List,
            map::Map,
        },
        value::Value,
        value_container::{
            ValueContainer, error::ValueError,
        },
    },
};
use alloc::rc::Rc;
use core::{cell::RefCell, ops::DerefMut};
mod collected_execution_result;
use crate::{
    instruction::instruction_data::{
        CallMethodData, CallableSignatureData, ImplMarkersData,
    },
    runtime::execution::macros::interrupt_with_borrowed_args_and_maybe_result,
    traits::apply::{ApplyArgument, into_apply_arguments_with_stack_indices},
    types::{
        entity_type::EntityType,
        type_definition::callable::CallableTypeDefinition,
    },
    value_updates::update_data::{
        DecrementUpdateData, IncrementUpdateData, ListSpliceUpdateData,
    },
    values::{
        core_values::callable::DatexBytecodeCallable,
        value::value_classification::{ValueClassification, ValueTag},
        value_container::borrowed_value_container::BorrowedValueContainer,
    },
};
use collected_execution_result::CollectedExecutionResult;

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
                    ExecutionInterrupt::TakeActiveValue => {
                        interrupt_provider.provide_result(
                            InterruptResult::ResolvedValue(active_value.take()),
                        );
                    }
                },
                Err(err) => {
                    match err {
                        ExecutionError::DXBParserError(
                            DXBParserError::ExpectingMoreInstructions(_),
                        ) => {
                            yield Err(
                                ExecutionError::IntermediateResultWithState(
                                    Box::new(active_value.take()),
                                    Box::new(None),
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

pub gen fn inner_execution_loop(
    dxb_body: Rc<RefCell<Vec<u8>>>,
    interrupt_provider: InterruptProvider,
    mut state: RuntimeExecutionState,
) -> Result<ExecutionInterrupt, ExecutionError> {
    let mut collector =
        InstructionCollector::<CollectedExecutionResult>::default();

    let seek_request: SeekRequest = Rc::new(RefCell::new(None));

    for instruction_result in iterate_instructions_with_seek(
        dxb_body,
        NestedInstructionResolutionStrategy::None,
        Some(seek_request.clone()),
    ) {
        let instruction = match instruction_result {
            Ok(instruction) => instruction,
            Err(DXBParserError::ExpectingMoreInstructions(stack)) => {
                yield Err(
                    DXBParserError::ExpectingMoreInstructions(stack).into()
                );
                // assume that when continuing after this yield, more instructions will have been loaded
                // so we run the loop again to try to get the next instruction
                continue;
            }
            Err(err) => {
                return yield Err(err.into());
            }
        };

        let result: Option<CollectedExecutionResult> = match instruction
            .instruction
        {
            // handle regular instructions
            Instruction::Regular(regular_instruction) => {
                let regular_instruction = collector
                    .default_regular_instruction_collection(
                        regular_instruction,
                        StatementResultCollectionStrategy::Last,
                        state.stack.current_index(),
                    );

                let expr: Option<Option<RuntimeValue>> = if let Some(
                    regular_instruction,
                ) =
                    regular_instruction
                {
                    if let RegularInstruction::Jump(data) = regular_instruction
                    {
                        // Jump has no result, so we need to place it here and not overwrite real result with None
                        seek_request.borrow_mut().replace(data.offset);
                        None
                    } else {
                        let regular_result: Result<
                            Option<RuntimeValue>,
                            ExecutionError,
                        > = try {
                            match regular_instruction {
                            // Handled above
                            RegularInstruction::Jump(_) => unreachable!(),
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
                            RegularInstruction::BigInteger(integer) => {
                                Some(ValueContainer::from(TypedInteger::IBig(integer)).into())
                            }

                            // default integer
                            RegularInstruction::Integer(integer) => {
                                Some(ValueContainer::from(integer).into())
                            }

                            // specific floats
                            RegularInstruction::DecimalF32(Float32Data(f32)) => {
                                Some(ValueContainer::from(TypedDecimal::from(f32)).into())
                            }
                            RegularInstruction::DecimalF64(Float64Data(f64)) => {
                                Some(ValueContainer::from(TypedDecimal::from(f64)).into())
                            }
                            // big decimal
                            RegularInstruction::BigDecimal(big_decimal) => {
                                Some(ValueContainer::from(TypedDecimal::Decimal(big_decimal)).into())
                            }

                            // default decimals
                            RegularInstruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                                Some(ValueContainer::from(Decimal::from(i16 as f32)).into())
                            }
                            RegularInstruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                                Some(ValueContainer::from(Decimal::from(i32 as f32)).into())
                            }
                            RegularInstruction::Decimal(big_decimal) => {
                                Some(ValueContainer::from(big_decimal).into())
                            }

                            // endpoint
                            RegularInstruction::Endpoint(endpoint) => Some(ValueContainer::from(endpoint).into()),

                            // instant (datetime), stored as i128, convert to Integer
                            RegularInstruction::Instant(InstantData(timestamp)) => {
                                Some(ValueContainer::from(Integer::new(timestamp)).into())
                            }

                            // null
                            RegularInstruction::Null => Some(ValueContainer::from(Value::null()).into()),

                            RegularInstruction::Uninitialized => Some(ValueContainer::from(Value::uninitialized()).into()),

                            // text
                            RegularInstruction::ShortText(ShortTextData(text)) => {
                                Some(ValueContainer::from(text).into())
                            }
                            RegularInstruction::Text(TextData(text)) => Some(ValueContainer::from(text).into()),

                            RegularInstruction::RequestRemoteSharedRef(address) => Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::GetReferenceToRemotePointer(address, ReferenceMutability::Immutable)
                                    )
                                ).into()),

                            RegularInstruction::RequestRemoteSharedRefMut(address) => Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::GetReferenceToRemotePointer(address, ReferenceMutability::Mutable)
                                    )
                                ).into()),

                            RegularInstruction::GetLocalSharedRef(address) => {
                                let val = interrupt_with_maybe_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::GetReferenceToLocalPointer(
                                            address
                                        )
                                    )
                                );
                                if let Some(val) = val {
                                    Some(val.into())
                                } else {
                                    return yield Err(ExecutionError::ReferenceNotFound);
                                }
                            }

                            RegularInstruction::GetCoreLibValue(id) => {
                                Some(interrupt_with_value!(
                                    interrupt_provider,
                                    ExecutionInterrupt::External(
                                        ExternalExecutionInterrupt::GetCoreLibValue(
                                            id.try_into().map_err(|_| ExecutionError::invalid_program(InvalidProgramError::InvalidCoreLibId(id)))?
                                        )
                                    )
                                ).into())
                            }

                            RegularInstruction::GetRootProperty(stack_index) => {
                                Some(RuntimeValue::ValueContainer(get_root_property(
                                    &state,
                                    stack_index,
                                )?))
                            }

                            RegularInstruction::BorrowStackValue(index) => {
                                Some(RuntimeValue::StackValue(index))
                            }

                            RegularInstruction::GetStackValueSharedRef(index) => {
                                let value = state.stack.get_stack_value(index)?;
                                match value {
                                    ValueContainer::Shared(container) => Some(RuntimeValue::ValueContainer(
                                        ValueContainer::Shared(SharedContainer::Referenced(container.derive_immutable_reference()))
                                    )),
                                    _ => return yield Err(ExecutionError::ExpectedSharedValue)
                                }
                            }
                            RegularInstruction::GetStackValueSharedRefMut(index) => {
                                let value = state.stack.get_stack_value(index)?;
                                match value {
                                    ValueContainer::Shared(container) => Some(RuntimeValue::ValueContainer(
                                        ValueContainer::Shared(SharedContainer::Referenced(
                                            container
                                                .try_derive_mutable_reference()
                                                .map_err(|_| ExecutionError::MutableReferenceToNonMutableValue)?
                                        ))
                                    )),
                                    _ => return yield Err(ExecutionError::ExpectedSharedValue)
                                }
                            }

                            RegularInstruction::CloneStackValue(index) => {
                                let value = state.stack.get_stack_value(index)?;
                                Some(RuntimeValue::ValueContainer(
                                    value.get_cloned()
                                ))
                            }

                            RegularInstruction::TakeStackValue(index) => {
                                Some(RuntimeValue::ValueContainer(
                                    state.stack.take_stack_value(index)?
                                ))
                            }

                            RegularInstruction::SharedRef(shared_ref) => {
                                let address = state.normalize_pointer_address(&shared_ref.address);
                                // shared ref without value, assumes value already known, otherwise request (todo)
                                let container = resolve_cache_value(
                                    &mut state,
                                    &address,
                                    SharedContainerOwnership::Referenced(shared_ref.ref_mutability),
                                )?;
                                Some(RuntimeValue::ValueContainer(ValueContainer::Shared(container)))
                            }

                            RegularInstruction::TaggedValue(TaggedValue { is_empty: true, tag: ShortTextData(tag) }) => {
                                Some(RuntimeValue::ValueContainer(
                                    ValueContainer::Local(Value::core_with_classification(CoreValue::Null, ValueClassification::new_with_tag(ValueTag {
                                        tag,
                                        is_empty: true,
                                    })))
                                ))
                            }

                            // NOTE: make sure that get_next_expected_instructions does not return None for these instructions!
                            RegularInstruction::Statements(_) |
                            RegularInstruction::ShortStatements(_) |
                            RegularInstruction::UnboundedStatements |
                            RegularInstruction::UnboundedStatementsEnd(_) |
                            RegularInstruction::List(_) |
                            RegularInstruction::Range |
                            RegularInstruction::ShortList(_) |
                            RegularInstruction::Map(_) |
                            RegularInstruction::ShortMap(_) |
                            RegularInstruction::TaggedValue(TaggedValue { is_empty: false, .. }) |
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
                            RegularInstruction::CallMethod(_) |
                            RegularInstruction::GetEntryText(_) |
                            RegularInstruction::GetEntryIndex(_) |
                            RegularInstruction::GetEntryDynamic |
                            RegularInstruction::TakeEntryText(_) |
                            RegularInstruction::TakeEntryIndex(_) |
                            RegularInstruction::TakeEntryDynamic |
                            RegularInstruction::SetEntryText(_) |
                            RegularInstruction::SetEntryIndex(_) |
                            RegularInstruction::SetEntryDynamic |
                            RegularInstruction::Is |
                            RegularInstruction::JumpIfFalse(_) |
                            RegularInstruction::JumpWithValue(_) |
                            RegularInstruction::Matches |
                            RegularInstruction::StructuralEqual |
                            RegularInstruction::Equal |
                            RegularInstruction::NotStructuralEqual |
                            RegularInstruction::NotEqual |
                            RegularInstruction::DeriveSharedReference |
                            RegularInstruction::DeriveSharedReferenceMut |
                            RegularInstruction::CreateShared |
                            RegularInstruction::CreateSharedMut |
                            RegularInstruction::PushToStack |
                            RegularInstruction::PushListToStack |
                            RegularInstruction::SetStackValue(_) |
                            RegularInstruction::Splice(_) |
                            RegularInstruction::SpliceDynamic |
                            RegularInstruction::AppendEntry |
                            RegularInstruction::Clear |
                            RegularInstruction::Increment |
                            RegularInstruction::Decrement |
                            RegularInstruction::SetSharedContainerValue |
                            RegularInstruction::Unbox |
                            RegularInstruction::EntityValue(_) |
                            RegularInstruction::RemoteExecution(_) |
                            RegularInstruction::MoveWithValue(_) |
                            RegularInstruction::SharedRefWithValue(_) |
                            RegularInstruction::CallableDeclaration(_) |
                            RegularInstruction::Callable(_) |
                            RegularInstruction::BoxedValue |
                            RegularInstruction::TypeExpression => {
                                // Note: If we reach this unreachable statement, we most probably forget to add a match
                                // arm to [RegularInstruction::get_next_expected_instructions](datex_core::global::regular_instructions)
                                unreachable!()
                            },
                            #[cfg(feature = "disassembler")]
                            RegularInstruction::_RemoteExecutionDebugFlat(_) |
                            RegularInstruction::_RemoteExecutionDebugTree(_) |
                            RegularInstruction::_CallableDeclarationDebugFlat(_) |
                            RegularInstruction::_CallableDeclarationDebugTree(_) |
                            RegularInstruction::_CallableDebugFlat(_) |
                            RegularInstruction::_CallableDebugTree(_)
                            => unreachable!(),
                        }
                        };
                        Some(match regular_result {
                            Ok(value) => value,
                            Err(error) => return yield Err(error),
                        })
                    }
                } else {
                    None
                };

                expr.map(CollectedExecutionResult::from)
            }
            Instruction::Type(type_instruction) => {
                let type_instruction = collector
                    .default_type_instruction_collection(type_instruction);

                if let Some(type_instruction) = type_instruction {
                    Some(match type_instruction {
                        TypeInstruction::CoreType(core_lib_type_id) => {
                            CollectedExecutionResult::type_definition(
                                TypeDefinition::CoreType(core_lib_type_id),
                            )
                        }
                        TypeInstruction::Literal(literal) => {
                            CollectedExecutionResult::type_definition(
                                literal.into(),
                            )
                        }
                        TypeInstruction::TaggedType(TaggedTypeData {
                            tag,
                            has_type: false,
                        }) => CollectedExecutionResult::type_definition(
                            TypeDefinition::TaggedType(
                                // TODO pointer addr
                                TaggedTypeDefinition::new_without_type(tag.0),
                            ),
                        ),

                        TypeInstruction::SharedTypeReference(type_ref) => {
                            let val = interrupt_with_maybe_value!(
                                interrupt_provider,
                                match type_ref.address {
                                    PointerAddress::SelfOwned(address) => {
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::GetReferenceToLocalPointer(
                                                address,
                                            ),
                                        )
                                    }
                                    PointerAddress::Remote(address) => {
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::GetReferenceToRemotePointer(
                                                address,
                                                ReferenceMutability::Immutable,
                                            ),
                                        )
                                    }
                                }
                            );

                            match val {
                                // simple Type value
                                Some(ValueContainer::Local(Value::Core(
                                    CoreValueWithClassification {
                                        inner: CoreValue::Type(_ty),
                                        ..
                                    },
                                ))) => todo!(),
                                // FIXME:
                                // // Type Reference
                                // Some(ValueContainer::Shared(SharedContainer {
                                //     value: SharedContainerInner::Type(type_ref),
                                //     .. })) => Type::new(
                                //     StructuralTypeDefinition::Shared(
                                //         type_ref,
                                //     ),
                                //     metadata,
                                // ),
                                _ => {
                                    return yield Err(
                                        ExecutionError::ExpectedTypeValue,
                                    );
                                }
                            }
                        }

                        TypeInstruction::ImplMarkers(ImplMarkersData {
                            impls,
                            ..
                        }) => CollectedExecutionResult::type_definition(
                            TypeDefinition::ImplMarkers(ImplMarkers::new(
                                impls,
                            )),
                        ),

                        // NOTE: make sure that get_next_expected_instructions does not return None for these instructions!
                        TypeInstruction::List(_)
                        | TypeInstruction::Intersection(_)
                        | TypeInstruction::ListSliceCollection(_)
                        | TypeInstruction::MapCollection
                        | TypeInstruction::ListCollection
                        | TypeInstruction::Boxed
                        | TypeInstruction::Union(_)
                        | TypeInstruction::Callable(_)
                        | TypeInstruction::Map(_)
                        | TypeInstruction::DefinitionWithMetadata(_)
                        | TypeInstruction::Range
                        | TypeInstruction::TaggedType(TaggedTypeData {
                            has_type: true,
                            ..
                        }) => {
                            panic!(
                                "Unexpected type instruction: {:?}",
                                type_instruction
                            );
                        }
                    })
                } else {
                    None
                }
            }
        };

        if let Some(result) = result {
            collector.push_result(result);
        }

        // handle collecting nested expressions
        while let Some(result) = collector.try_pop_collected() {
            let expr_result: Result<CollectedExecutionResult, ExecutionError> = try {
                match result {
                    FullOrPartialResult::Full {
                        instruction,
                        results: mut collected_results,
                    } => {
                        match instruction {
                            Instruction::Regular(
                                regular_instruction,
                            ) => match regular_instruction {
                                RegularInstruction::List(_)
                                | RegularInstruction::ShortList(_) => {
                                    let elements = collected_results.try_collect_value_containers(&mut state)?;
                                    RuntimeValue::ValueContainer(
                                        ValueContainer::from(List::new(
                                            elements,
                                        )),
                                    )
                                        .into()
                                }
                                RegularInstruction::Map(_)
                                | RegularInstruction::ShortMap(_) => {
                                    let entries = collected_results.try_collect_key_value_pair()?;
                                    RuntimeValue::ValueContainer(
                                        ValueContainer::from(Map::from(
                                            entries,
                                        )),
                                    )
                                        .into()
                                }

                                RegularInstruction::KeyValueDynamic => {
                                    let value = collected_results.try_pop_value_container(&mut state)?;
                                    let key = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    CollectedExecutionResult::key_value_pair(
                                        key,
                                        value,
                                    )
                                }

                                RegularInstruction::KeyValueShortText(
                                    short_text_data,
                                ) => {
                                    let value = collected_results.try_pop_value_container(&mut state)?;
                                    let key = ValueContainer::from(short_text_data.0);
                                    CollectedExecutionResult::key_value_pair(
                                        key, value,
                                    )
                                }

                                RegularInstruction::TaggedValue(TaggedValue {
                                    tag: ShortTextData(tag),
                                    is_empty
                                }) => {
                                    assert!(!is_empty);
                                    let value_container = collected_results.try_pop_value_container(&mut state)?;
                                    create_tagged_value_container(
                                        value_container,
                                        tag,
                                    )?.into()
                                }

                                RegularInstruction::Add
                                | RegularInstruction::Subtract
                                | RegularInstruction::Multiply
                                | RegularInstruction::Range
                                | RegularInstruction::Divide => {
                                    let right = collected_results
                                        .try_pop_runtime_value()?;
                                    let left = collected_results
                                        .try_pop_runtime_value()?;

                                    let res = handle_binary_operation(
                                        BinaryOperator::from(
                                            regular_instruction,
                                        ),
                                        left.as_value_container(&state.stack)?,
                                        right.as_value_container(&state.stack)?,
                                    )?;
                                    res.into()
                                }

                                RegularInstruction::Is
                                | RegularInstruction::StructuralEqual
                                | RegularInstruction::Equal
                                | RegularInstruction::NotStructuralEqual
                                | RegularInstruction::NotEqual => {
                                    let right = collected_results
                                        .try_pop_runtime_value()?;
                                    let left = collected_results
                                        .try_pop_runtime_value()?;

                                    let res = handle_comparison_operation(
                                        ComparisonOperator::from(
                                            regular_instruction,
                                        ),
                                        left.as_value_container(&state.stack)?,
                                        right.as_value_container(&state.stack)?,
                                    )?;
                                    res.into()
                                }

                                RegularInstruction::CallableDeclaration(callable_declaration) => {
                                    let types = collected_results.pop_types(callable_declaration.signature.total_type_count());
                                    let signature = resolve_callable_type_definition(types, &callable_declaration.signature);
                                    let injected_values = state.stack.resolve_injected_values(&callable_declaration.body.injected_values)?;

                                    ValueContainer::from(Callable {
                                        name: if callable_declaration.signature.name.0.is_empty() {
                                            None
                                        } else {
                                            Some(callable_declaration.signature.name.0)
                                        },
                                        signature,
                                        body: CallableBody::DatexBytecode(DatexBytecodeCallable {
                                            requires_async: callable_declaration.signature.requires_async,
                                            injected_values,
                                            body: callable_declaration.body.body,
                                        }),
                                        creator: state.caller_metadata.endpoint.clone(),
                                    })
                                        .into()
                                }
                                RegularInstruction::Callable(callable_declaration) => {

                                    // indicates a native callable declaration which cannot be transferred
                                    let has_native_impl = callable_declaration.body.length == 0;

                                    let injected_values = collected_results
                                        .pop_values(callable_declaration.body.injected_value_count)
                                        .into_iter()
                                        .flatten()
                                        .map(|runtime_value| runtime_value.into_value_container(&mut state))
                                        .collect::<Result<Vec<_>, ExecutionError>>()?;

                                    let types = collected_results.pop_types(callable_declaration.signature.total_type_count());
                                    let signature = resolve_callable_type_definition(types, &callable_declaration.signature);

                                    ValueContainer::from(Callable {
                                        name: if callable_declaration.signature.name.0.is_empty() {
                                            None
                                        } else {
                                            Some(callable_declaration.signature.name.0)
                                        },
                                        signature,
                                        body: if has_native_impl {
                                                CallableBody::Hidden
                                            }
                                            else {
                                                CallableBody::DatexBytecode(DatexBytecodeCallable {
                                                    requires_async: callable_declaration.signature.requires_async,
                                                    injected_values,
                                                    body: callable_declaration.body.body,
                                                })
                                            },
                                        creator: state.caller_metadata.endpoint.clone(),
                                    })
                                        .into()
                                }

                                RegularInstruction::Matches => {
                                    let _target = collected_results
                                        .try_pop_runtime_value()?;
                                    let _type_pattern =
                                        collected_results.pop_type();

                                    todo!("#645 Undescribed by author.")
                                }

                                instruction @ (
                                    RegularInstruction::CreateShared |
                                    RegularInstruction::CreateSharedMut
                                ) => {
                                    let value = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    let mutability = match instruction {
                                        RegularInstruction::CreateShared => SharedContainerMutability::Immutable,
                                        RegularInstruction::CreateSharedMut => SharedContainerMutability::Mutable,
                                        _ => unreachable!(),
                                    };

                                    create_owned_shared_container(
                                        value,
                                        mutability,
                                        state.runtime.pointer_address_provider_mut().deref_mut(),
                                    ).into()
                                }

                                RegularInstruction::DeriveSharedReference => {
                                    let target = collected_results
                                        .try_pop_value_container(&mut state)?;

                                    derive_shared_reference(
                                        &target,
                                        ReferenceMutability::Immutable,
                                    )?.into()
                                }

                                RegularInstruction::DeriveSharedReferenceMut => {
                                    let target = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    derive_shared_reference(
                                        &target,
                                        ReferenceMutability::Mutable,
                                    )?.into()
                                }

                                RegularInstruction::UnaryMinus
                                | RegularInstruction::UnaryPlus
                                | RegularInstruction::BitwiseNot
                                | RegularInstruction::Unbox => {
                                    let target = collected_results
                                        .try_pop_runtime_value()?;
                                    let value_container = target.as_value_container(
                                        &state.stack
                                    )?.clone();
                                    let res = handle_unary_operation(
                                        UnaryOperator::from(
                                            regular_instruction,
                                        ),
                                        value_container, // TODO #646: is unary operation supposed to take ownership?
                                        state.runtime.shared_references_cache_refcell(),
                                    )?;
                                    RuntimeValue::ValueContainer(
                                        res
                                    ).into()
                                }
                                RegularInstruction::BoxedValue => {
                                    let expression = collected_results.try_pop_runtime_value()?;
                                    let value_container = expression.into_value_container(&mut state)?;
                                    ValueContainer::from(Value::boxed(value_container)).into()
                                }
                                RegularInstruction::EntityValue(address) => {
                                    let mut value_container = collected_results
                                        .try_pop_value_container(&mut state)?;

                                    let container = resolve_cache_value(
                                        &mut state,
                                        &address,
                                        SharedContainerOwnership::Owned,
                                    )?;
                                    // try to convert to EntityType
                                    let entity_type = EntityType::try_from(container)
                                        .map_err(|_| ExecutionError::invalid_program(InvalidProgramError::InvalidType))?;

                                    match &mut value_container {
                                        ValueContainer::Local(Value::Core(value)) => {
                                            value.classification = ValueClassification::new_with_entity(entity_type);
                                        }
                                        _ => return yield Err(ExecutionError::invalid_program(InvalidProgramError::InvalidType)),
                                    }
                                    RuntimeValue::ValueContainer(
                                        value_container,
                                    )
                                        .into()
                                }

                                // type(...)
                                RegularInstruction::TypeExpression => {
                                    let ty =
                                        collected_results.pop_type();
                                    RuntimeValue::ValueContainer(
                                        ValueContainer::Local(Value::new(CoreValue::Type(ty))), // TODO #648: type for type
                                    )
                                        .into()
                                }

                                RegularInstruction::SetSharedContainerValue => {
                                    let mut target = collected_results
                                        .try_pop_runtime_value()?;
                                    let new_value: ValueContainer = collected_results
                                        .try_pop_runtime_value()?
                                        .into_value_container(&mut state)?;
                                    let source_id = state.source_id_cloned();
                                    let target = target.as_value_container_mut(&mut state.stack)?;
                                    try_set_shared_container_value(
                                        target,
                                        new_value,
                                        source_id,
                                        state.runtime.shared_references_cache_refcell()
                                    )?;
                                    None.into()
                                }

                                RegularInstruction::Splice(
                                    splice,
                                ) => {
                                    let source_id = state.source_id_cloned();
                                    let mut target = collected_results.try_pop_runtime_value()?;
                                    let values = collected_results.try_collect_value_containers(&mut state)?;
                                    let target_value = target.as_value_container_mut(&mut state.stack)?;

                                    let res_values = target_value
                                        .try_list_splice(vec![], source_id, ListSpliceUpdateData::new(
                                            splice.start_index,
                                            splice.delete_count,
                                            values,
                                        ), state.runtime.shared_references_cache_refcell()).map_err(|e| e.into())?;

                                    // create new list from result values
                                    ValueContainer::from(List::new(res_values)).into()
                                }

                                RegularInstruction::SpliceDynamic => {
                                    let source_id = state.source_id_cloned();
                                    let mut target = collected_results.try_pop_runtime_value()?;
                                    let values = collected_results.try_pop_value_container(&mut state)?;
                                    let delete_count = collected_results.try_pop_value_container(&mut state)?;
                                    let start_index = collected_results.try_pop_value_container(&mut state)?;

                                    // values must be a list
                                    let values: List = match values.try_into_value() {
                                        Ok(list) => list,
                                        Err(_) => return yield Err(ExecutionError::invalid_program(InvalidProgramError::ExpectedList))
                                    };
                                    // delete count must be integer
                                    let delete_count: u32 = match delete_count.clone().try_into_value::<Integer>() {
                                        Ok(int) => int.as_wrapped_u32(),
                                        Err(_) => match delete_count.try_into_value::<TypedInteger>() {
                                            Ok(int) => int.as_usize().unwrap() as u32,
                                            Err(_) => return yield Err(ExecutionError::invalid_program(InvalidProgramError::ExpectedList)),
                                        }
                                    };

                                    // start_index count must be integer
                                    let start_index: u32 = match start_index.clone().try_into_value::<Integer>() {
                                        Ok(int) => int.as_wrapped_u32(),
                                        Err(_) => match start_index.try_into_value::<TypedInteger>() {
                                            Ok(int) => int.as_usize().unwrap() as u32,
                                            Err(_) => return yield Err(ExecutionError::invalid_program(InvalidProgramError::ExpectedList)),
                                        }
                                    };

                                    let target_value = target.as_value_container_mut(&mut state.stack)?;

                                    let res_values = target_value
                                        .try_list_splice(vec![], source_id, ListSpliceUpdateData::new(
                                            start_index,
                                            delete_count,
                                            values.into_vec(),
                                        ), state.runtime.shared_references_cache_refcell()).map_err(|e| e.into())?;

                                    // create new list from result values
                                    ValueContainer::from(List::new(res_values)).into()
                                }

                                RegularInstruction::Clear => {
                                    let source_id = state.source_id_cloned();
                                    let mut target = collected_results.try_pop_runtime_value()?;
                                    let target_value = target.as_value_container_mut(&mut state.stack)?;

                                    // TODO: res?
                                    let _res = target_value
                                        .try_clear(vec![], source_id, state.runtime.shared_references_cache_refcell())
                                        .map_err(|e| e.into())?;

                                    None.into()
                                }
                                RegularInstruction::AppendEntry => {
                                    let source_id = state.source_id_cloned();
                                    let mut target = collected_results.try_pop_runtime_value()?;
                                    let value = collected_results.try_pop_value_container(&mut state)?;
                                    let target_value = target.as_value_container_mut(&mut state.stack)?;

                                    target_value
                                        .try_append_entry(vec![], source_id, AppendEntryUpdateData::new(value), state.runtime.shared_references_cache_refcell())
                                        .map_err(|e| e.into())?;

                                    None.into()
                                }

                                RegularInstruction::Increment => {
                                    let source_id = state.source_id_cloned();
                                    let mut target = collected_results.try_pop_runtime_value()?;
                                    let value = collected_results.try_pop_value_container(&mut state)?;
                                    let target_value = target.as_value_container_mut(&mut state.stack)?;


                                    target_value
                                        .try_increment(vec![], source_id, IncrementUpdateData::new(value), state.runtime.shared_references_cache_refcell())
                                        .map_err(|e| e.into())?;

                                    None.into()
                                }

                                RegularInstruction::Decrement => {
                                    let source_id = state.source_id_cloned();
                                    let mut target = collected_results.try_pop_runtime_value()?;
                                    let value = collected_results.try_pop_value_container(&mut state)?;
                                    let target_value = target.as_value_container_mut(&mut state.stack)?;

                                    target_value
                                        .try_decrement(vec![], source_id, DecrementUpdateData::new(value), state.runtime.shared_references_cache_refcell())
                                        .map_err(|e| e.into())?;

                                    None.into()
                                }

                                RegularInstruction::SetStackValue(index) => {
                                    let value = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    state
                                        .stack
                                        .set_stack_value(index, value)?;
                                    None.into()
                                }

                                RegularInstruction::PushToStack => {
                                    let value = collected_results
                                        .try_pop_value_container(&mut state)?;

                                    state
                                        .stack
                                        .push(value);

                                    None.into()
                                }

                                RegularInstruction::PushListToStack => {
                                    let value = collected_results
                                        .try_pop_value_container(&mut state)?;

                                    // value must be a list value
                                    // push all entries onto the stack
                                    match value {
                                        ValueContainer::Local(Value::Core(CoreValueWithClassification{ inner: CoreValue::List(list), .. })) => {
                                            for value in list {
                                                state.stack
                                                    .push(value);
                                            }
                                        }
                                        _ => {
                                            return yield Err(ExecutionError::invalid_program(InvalidProgramError::ExpectedList))
                                        }
                                    }

                                    None.into()
                                }

                                RegularInstruction::GetEntryText(
                                    property_data,
                                ) => {
                                    let mut target = collected_results
                                        .try_pop_runtime_value()?;
                                    let property_name = property_data.0;
                                    let target = target.as_value_container_mut(
                                        &mut state.stack
                                    )?;

                                    let res = if let Some(endpoint) = target.try_as::<Endpoint>() {
                                        interrupt_with_value!(
                                            interrupt_provider,
                                            ExecutionInterrupt::External(
                                                ExternalExecutionInterrupt::GetEndpointProperty{
                                                        endpoint: endpoint.clone(),
                                                        property_name,
                                                    }
                                            )
                                        )
                                    } else {
                                        let collapsed_value = target.collapsed_value();
                                        collapsed_value.borrow().try_get_property(
                                            &ValueContainer::from(property_name),
                                        )
                                            .map(BorrowedValueContainer::try_clone_to_value_container)  // FIXME: no clone?
                                            .map_err(ExecutionError::access_error)?
                                            .map_err(|_| ExecutionError::UnclonableValue)?
                                    };

                                    res.into()
                                }

                                RegularInstruction::GetEntryIndex(
                                    property_data,
                                ) => {
                                    let target = collected_results
                                        .try_pop_runtime_value()?;
                                    let property_index = property_data.0;

                                    let value_container = target.as_value_container(&state.stack)?;
                                    let collapsed_value = value_container.collapsed_value();
                                    let res = collapsed_value.borrow()
                                        .try_get_property(
                                            &ValueContainer::from(property_index)
                                        )
                                        .map(BorrowedValueContainer::try_clone_to_value_container) // FIXME: no clone?
                                        .map_err(ExecutionError::access_error)?
                                        .map_err(|_| ExecutionError::UnclonableValue)?;
                                    res.into()
                                }

                                RegularInstruction::GetEntryDynamic => {
                                    let key = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    let target = collected_results
                                        .try_pop_runtime_value()?;

                                    let value_container = target.as_value_container(&state.stack)?;
                                    let collapsed_value = value_container.collapsed_value();
                                    let res = collapsed_value.borrow()
                                        .try_get_property(&key)
                                        .map(BorrowedValueContainer::try_clone_to_value_container)  // FIXME: no clone?
                                        .map_err(ExecutionError::access_error)?
                                        .map_err(|_| ExecutionError::UnclonableValue)?;

                                    res.into()
                                }

                                RegularInstruction::TakeEntryIndex(
                                    property_data,
                                ) => {
                                    let mut target = collected_results
                                        .try_pop_runtime_value()?;
                                    let property_index = property_data.0;

                                    let source_id = state.source_id_cloned();
                                    let value_container = target.as_value_container_mut(&mut state.stack)?;
                                    let res = value_container.try_delete_entry(
                                        vec![], // FIXME path
                                        source_id,
                                        DeleteEntryUpdateData { key: ValueContainer::local(property_index as i64) },
                                        state.runtime.shared_references_cache_refcell()
                                    ).map_err(ExecutionError::update_error)?;
                                    ValueContainer::new_from_option(res)
                                        .into()
                                }

                                RegularInstruction::SetEntryText(
                                    property_data,
                                ) => {
                                    let mut target_runtime_value = collected_results
                                        .try_pop_runtime_value()?;
                                    let value_runtime_value = collected_results
                                        .try_pop_runtime_value()?;
                                    let source_id = state.source_id_cloned();
                                    let value = value_runtime_value.into_value_container(&mut state)?;
                                    let target = target_runtime_value.as_value_container_mut(&mut state.stack)?;

                                    let res = if let Some(endpoint) = target.try_as::<Endpoint>() {
                                        interrupt_with_maybe_value!(
                                            interrupt_provider,
                                            ExecutionInterrupt::External(
                                                ExternalExecutionInterrupt::SetEndpointProperty
                                                    {
                                                        endpoint: endpoint.clone(),
                                                        property_name: property_data.0,
                                                        value,
                                                    }
                                            )
                                        )
                                    } else {
                                        try_set_property(
                                            target,
                                            ValueContainer::local(
                                                property_data.0,
                                            ),
                                            value,
                                            vec![], // FIXME path
                                            source_id,
                                            state.runtime.shared_references_cache_refcell(),
                                        )?
                                    };
                                    ValueContainer::new_from_option(res).into()
                                }

                                RegularInstruction::SetEntryIndex(
                                    property_data,
                                ) => {
                                    let mut target = collected_results
                                        .try_pop_runtime_value()?;
                                    let value = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    let source_id = state.source_id_cloned();
                                    let value_container = target.as_value_container_mut(&mut state.stack)?;

                                    let _res = try_set_property(
                                        value_container,
                                        ValueContainer::local(
                                            property_data.0 as i64,
                                        ),
                                        value,
                                        vec![], // FIXME path
                                        source_id,
                                    state.runtime.shared_references_cache_refcell())?;
                                    None.into()
                                }

                                RegularInstruction::SetEntryDynamic => {
                                    let mut target = collected_results
                                        .try_pop_runtime_value()?;
                                    let value = collected_results
                                        .try_pop_value_container(&mut state)?;
                                    let key = collected_results
                                        .try_pop_value_container(&mut state)?;

                                    let source_id = state.source_id_cloned();

                                    let value_container = target.as_value_container_mut(&mut state.stack)?;

                                    let _res = try_set_property(
                                        value_container,
                                        key,
                                        value,
                                        vec![], // FIXME path
                                        source_id,
                                        state.runtime.shared_references_cache_refcell()
                                    )?;
                                    None.into()
                                }

                                RegularInstruction::MoveWithValue(move_with_value) => {
                                    let address = state.normalize_pointer_address(&PointerAddress::SelfOwned(move_with_value.previous_address));

                                    // for local addresses, if first value is in cache, assume all values are in cache and resolve
                                    if state.caller_metadata.endpoint.is_local_or_equals_endpoint(state.runtime.endpoint()) &&
                                        state
                                            .shared_value_cache
                                            .has_address_with_ownership(&address, SharedContainerOwnership::Owned) {
                                        let container = resolve_cache_value(
                                            &mut state,
                                            &address,
                                            SharedContainerOwnership::Owned,
                                        )?;
                                        Some(RuntimeValue::ValueContainer(ValueContainer::Shared(container))).into()
                                    }
                                    // otherwise, perform move
                                    else {
                                        let value = collected_results.try_pop_runtime_value()?.into_value_container(&mut state)?;
                                        let container = SharedContainer::new_owned_with_inferred_allowed_type(
                                            value,
                                            move_with_value.mutability,
                                            state.runtime.pointer_address_provider_mut().deref_mut(),
                                        );
                                        // TODO: confirm move


                                        Some(RuntimeValue::ValueContainer(ValueContainer::Shared(container))).into()
                                    }
                                }

                                RegularInstruction::RemoteExecution(
                                    exec_block_data,
                                ) => {
                                    let receivers = collected_results
                                        .try_pop_value_container(&mut state)?;

                                    // ensure receiver is single endpoint
                                    let receivers_list: Vec<Endpoint> = match receivers {
                                        ValueContainer::Local(Value::Core(CoreValueWithClassification { inner: CoreValue::Endpoint(endpoint), .. })) => vec![endpoint],
                                        // TODO: support advanced receivers
                                        _ => return yield Err(ExecutionError::value_error(ValueError::InvalidOperation))
                                    };

                                    let injected_values = state.stack.resolve_injected_values(&exec_block_data.injected_values)?;

                                    // build dxb
                                    let DXBWithSharedValues {
                                        dxb: buffer,
                                        shared_values: shared_containers,
                                    } = {
                                        let lookup = state.runtime.pointer_availability_lookup();
                                        let compile_input = CompileInput::new(
                                            &lookup,
                                            &receivers_list,
                                        );

                                        compile_injected_values(
                                            exec_block_data,
                                            injected_values,
                                            compile_input,
                                        ).map_err(|_e| ExecutionError::invalid_program(InvalidProgramError::ExpectedValue))?
                                    };

                                    interrupt_with_maybe_value!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::RemoteExecution {
                                                input: DXBWithSharedValues {
                                                    dxb: buffer,
                                                    shared_values: shared_containers,
                                                },
                                                receivers: receivers_list,
                                            }
                                        )
                                    )
                                        .map(RuntimeValue::ValueContainer)
                                        .into()
                                }

                                RegularInstruction::Apply(ApplyData { .. }) => {
                                    let mut args = collected_results.try_collect_runtime_values()?;
                                    // last argument is the callee
                                    let callee = args.remove(args.len() - 1);

                                    // special handling for panic function - abort execution
                                    if let RuntimeValue::ValueContainer(ValueContainer::Local(Value::Core(CoreValueWithClassification { inner: CoreValue::Callable(Callable { body: CallableBody::CoreStub(CoreStub::Panic), .. }), .. }))) = &callee
                                    {
                                        // assert for now that single string arg
                                        let error = args
                                            .remove(0)
                                            .into_value_container(&mut state)?
                                            .try_into_value::<String>()
                                            .map_err(|_| ExecutionError::InvalidProgram(Box::new(InvalidProgramError::InvalidType)))?;
                                        return yield Err(ExecutionError::Unspecified(error));
                                    }

                                    let (callee, _callee_stack_index) = callee.into_value_container_with_previous_stack_index(&mut state)?;
                                    let (args, borrowed_args_stack_indices) = state.stack.take_runtime_values_with_stack_indices(args)?;

                                    let (result, borrowed_args) = interrupt_with_borrowed_args_and_maybe_result!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::Apply(
                                                callee,
                                                into_apply_arguments_with_stack_indices(
                                                    args,
                                                    &borrowed_args_stack_indices,
                                                )
                                            )
                                        )
                                    );

                                    // put the borrowed args back on the stack
                                    state.stack.restore_stack_values(borrowed_args, borrowed_args_stack_indices)?;


                                    result.map(|val| {
                                            RuntimeValue::ValueContainer(val)
                                        })
                                        .into()
                                }
                                RegularInstruction::JumpIfFalse(JumpData { offset }) => {
                                    let condition_value = collected_results.try_pop_runtime_value()?;
                                    let condition_value = condition_value.as_value_container(&state.stack)?;
                                    // We dont accept anything expect for explicit Boolean(...)
                                    if condition_value.try_as::<Boolean>().ok_or_else(|| ExecutionError::InvalidTypeCast)?.is_false() {
                                        seek_request.borrow_mut().replace(offset);
                                    }
                                    CollectedExecutionResult::value(None)
                                }

                                RegularInstruction::CallMethod(CallMethodData {method_name, ..}) => {
                                    let method_name = method_name.0;

                                    let callee = collected_results.try_pop_runtime_value()?;
                                    let args = collected_results.try_collect_runtime_values()?;

                                    let (callee, callee_stack_index) = callee.into_value_container_with_previous_stack_index(&mut state)?;
                                    let (args, borrowed_args_stack_indices) = state.stack.take_runtime_values_with_stack_indices(args)?;

                                    let (val, borrowed_args) = interrupt_with_borrowed_args_and_maybe_result!(
                                        interrupt_provider,
                                        ExecutionInterrupt::External(
                                            ExternalExecutionInterrupt::CallMethod(
                                                ApplyArgument {
                                                    value: callee,
                                                    passed_as_ref: callee_stack_index.is_some(),
                                                },
                                                method_name,
                                                into_apply_arguments_with_stack_indices(
                                                    args,
                                                    &borrowed_args_stack_indices,
                                                )
                                            )
                                        )
                                    );

                                    let mut borrowed_args_iter = borrowed_args.into_iter();

                                    // put the callee back on the stack if it was borrowed
                                    if let Some(callee_stack_index) = callee_stack_index {
                                        let borrowed_callee = borrowed_args_iter.next().unwrap();
                                        state.stack.set_stack_value(callee_stack_index, borrowed_callee)?;
                                    }

                                    let borrowed_args = borrowed_args_iter.collect::<Vec<_>>();

                                    // put the borrowed args back on the stack
                                    state.stack.restore_stack_values(borrowed_args, borrowed_args_stack_indices)?;


                                    val.map(|val| {
                                        RuntimeValue::ValueContainer(val)
                                    })
                                        .into()
                                }

                                RegularInstruction::UnboundedStatementsEnd(
                                    UnboundedStatementsData { terminated },
                                ) => {
                                    let result = collector.try_pop_unbounded()
                                        .ok_or(ExecutionError::dxb_parser_error(DXBParserError::NotInUnboundedRegularScopeError))?;
                                    if let FullOrPartialResult::Partial {
                                        result: collected_result,
                                        previous_stack_index,
                                        ..
                                    } = result
                                    {
                                        // reset stack index
                                        state.stack.truncate(previous_stack_index);
                                        if terminated {
                                            CollectedExecutionResult::value(
                                                None,
                                            )
                                        } else {
                                            match collected_result {
                                                Some(CollectedExecutionResult::Value(val)) => (*val).into(),
                                                None => {
                                                    // if no last result, it might have been moved to the active value, try to get back
                                                    let active_value = interrupt_with_maybe_value!(interrupt_provider, ExecutionInterrupt::TakeActiveValue);
                                                    CollectedExecutionResult::value(active_value.map(RuntimeValue::ValueContainer))
                                                }
                                                _ => unreachable!(),
                                            }
                                        }
                                    } else {
                                        unreachable!()
                                    }
                                }

                                RegularInstruction::SharedRefWithValue(shared_ref) => {
                                    let address = state.normalize_pointer_address(&PointerAddress::SelfOwned(shared_ref.address.clone()));

                                    let value = collected_results
                                        .try_pop_runtime_value()?
                                        .into_value_container(&mut state)?;

                                    // if caller endpoint is local endpoint, this is a local pointer
                                    let referenced_container = if state.caller_metadata.endpoint.is_local_or_equals_endpoint(state.runtime.endpoint()) {
                                        let cache_result = resolve_cache_value(
                                            &mut state,
                                            &address,
                                            SharedContainerOwnership::Referenced(shared_ref.ref_mutability),
                                        );
                                        match cache_result {
                                            // TODO: update new value or compare hashes to make sure we have the latest value here
                                            Ok(container) => match container {
                                                SharedContainer::Referenced(referenced_container) => referenced_container,
                                                SharedContainer::Owned(_) => {
                                                    unreachable!()
                                                }
                                            },
                                            Err(_) => {
                                                create_new_reference_from_value(
                                                    &address,
                                                    &state.runtime,
                                                    value,
                                                    shared_ref.container_mutability,
                                                    shared_ref.ref_mutability,
                                                )?
                                            }
                                        }
                                    }
                                    // else, get remote pointer from address
                                    else {
                                        let pointer_address = PointerAddress::Remote(RemotePointerAddress::for_endpoint(&state.caller_metadata.endpoint, &shared_ref.address));
                                        create_new_reference_from_value(
                                            &pointer_address,
                                            &state.runtime,
                                            value,
                                            shared_ref.container_mutability,
                                            shared_ref.ref_mutability,
                                        )?
                                    };

                                    let container = SharedContainer::Referenced(referenced_container);
                                    CollectedExecutionResult::value(Some(ValueContainer::Shared(container).into()))
                                }

                                e => {
                                    todo!(
                                        "Unhandled collected regular instruction: {:?}",
                                        e
                                    );
                                }
                            },

                            Instruction::Type(type_instruction) => {
                                match type_instruction {
                                    TypeInstruction::TaggedType(tagged_type_data) => {
                                        if tagged_type_data.has_type {
                                            let def = collected_results.pop_type();
                                            TypeDefinition::TaggedType(TaggedTypeDefinition::new(
                                                def,
                                                tagged_type_data.tag.0
                                            )).into()
                                        } else {
                                            TypeDefinition::TaggedType(TaggedTypeDefinition::new_without_type(
                                                tagged_type_data.tag.0
                                            )).into()
                                        }
                                    }
                                    TypeInstruction::Range => {
                                        let type_end =
                                            collected_results.pop_type();
                                        let type_start =
                                            collected_results.pop_type();
                                        let x = Type::Definition(
                                            TypeDefinition::Range(RangeTypeDefinition {
                                                start: Box::new(type_start),
                                                end: Box::new(type_end),
                                            }).into(),
                                        );
                                        x.into()
                                    }
                                    TypeInstruction::DefinitionWithMetadata(metadata) => {
                                        let definition = collected_results.pop_type_definition();
                                        Type::Definition(TypeDefinitionWithMetadata::new(definition, metadata)).into()
                                    }
                                    TypeInstruction::Map(MapData { element_count}) => {
                                        let mut map_entries = Vec::with_capacity(element_count as usize);
                                        for _ in 0..element_count {
                                            let value_type = collected_results.pop_type();
                                            let key_type = collected_results.pop_type();
                                            map_entries.push((key_type, value_type));
                                        }
                                        TypeDefinition::Map(MapTypeDefinition(map_entries.into_iter().rev().collect())).into()
                                    }
                                    TypeInstruction::Union(UnionData {element_count}) => {
                                        let mut union_entries = Vec::with_capacity(element_count as usize);
                                        for _ in 0..element_count {
                                            let ty = collected_results.pop_type();
                                            union_entries.push(ty);
                                        }
                                        TypeDefinition::Union(UnionTypeDefinition(union_entries.into_iter().rev().collect())).into()
                                    }
                                    TypeInstruction::Intersection(IntersectionData {element_count}) => {
                                        let mut intersection_entries = Vec::with_capacity(element_count as usize);
                                        for _ in 0..element_count {
                                            let ty = collected_results.pop_type();
                                            intersection_entries.push(ty);
                                        }
                                        TypeDefinition::Intersection(IntersectionTypeDefinition(intersection_entries.into_iter().rev().collect())).into()
                                    }
                                    TypeInstruction::ListSliceCollection(ListSliceCollectionData { element_count }) => {
                                        let ty = collected_results.pop_type();
                                        TypeDefinition::Collection(CollectionTypeDefinition::ListSlice(
                                            ListSliceCollectionTypeDefinition::new(ty, element_count as usize)
                                        )).into()
                                    }
                                    TypeInstruction::MapCollection => {
                                        let value_type = collected_results.pop_type();
                                        let key_type = collected_results.pop_type();
                                        TypeDefinition::Collection(CollectionTypeDefinition::Map(
                                            MapCollectionTypeDefinition::new(key_type, value_type)
                                        )).into()
                                    }
                                    TypeInstruction::ListCollection => {
                                        let ty = collected_results.pop_type();
                                        TypeDefinition::Collection(CollectionTypeDefinition::List(
                                            ListCollectionTypeDefinition::new(ty)
                                        )).into()
                                    }
                                    TypeInstruction::Boxed => {
                                        let ty = collected_results.pop_type();
                                        TypeDefinition::Box(Box::new(ty)).into()
                                    }
                                    TypeInstruction::Callable(signature_data) => {
                                        let callable_type = resolve_callable_type_definition(
                                            collected_results.pop_types(signature_data.total_type_count()),
                                            &signature_data,
                                        );
                                        TypeDefinition::Callable(callable_type).into()
                                    }
                                    e => todo!(
                                        "Unhandled collected type instruction: {:?}",
                                        e
                                    ),
                                }
                            }
                        }
                    }
                    FullOrPartialResult::Partial {
                        instruction,
                        result: collected_result,
                        previous_stack_index,
                    } => {
                        // reset stack index
                        state.stack.truncate(previous_stack_index);

                        match instruction {
                            Instruction::Regular(regular_instruction) => {
                                match regular_instruction {
                                    RegularInstruction::ShortStatements(
                                        ShortStatementsData {
                                            terminated, ..
                                        },
                                    )
                                    | RegularInstruction::Statements(
                                        StatementsData { terminated, .. },
                                    ) => {
                                        if terminated {
                                            CollectedExecutionResult::value(
                                                None,
                                            )
                                        } else {
                                            match collected_result {
                                                Some(
                                                    CollectedExecutionResult::Value(
                                                        val,
                                                    ),
                                                ) => (*val).into(),
                                                None => {
                                                    CollectedExecutionResult::value(
                                                        None,
                                                    )
                                                }
                                                _ => unreachable!(), // statements always resolve to values
                                            }
                                        }
                                    }
                                    _ => unreachable!(),
                                }
                            }

                            Instruction::Type(_data) => unreachable!(),
                        }
                    }
                }
            };

            let expr: CollectedExecutionResult = match expr_result {
                Ok(expr) => expr,
                Err(error) => return yield Err(error),
            };

            // info!("{} | {} >>> {:#?}", state.runtime_internal.endpoint,instruction_copy, expr);

            collector.push_result(expr);
        }

        // if in unbounded statements, propagate active value via interrupt
        if let Some(ResultCollector::LastUnbounded(
            LastUnboundedResultCollector { last_result, .. },
        )) = collector.last_mut()
            && let Some(CollectedExecutionResult::Value(mut last_result)) =
                last_result.take()
        {
            let active_value_result: Result<_, ExecutionError> = try {
                last_result
                    .take()
                    .map(|v| v.into_value_container(&mut state))
                    .transpose()?
            };

            let active_value = match active_value_result {
                Ok(value) => value,
                Err(error) => return yield Err(error),
            };

            interrupt!(
                interrupt_provider,
                ExecutionInterrupt::SetActiveValue(active_value)
            );

            // TODO: handle other CollectedExecutionResults
        }
    }

    if let Some(result) = collector.take_root_result() {
        let root_result: Result<_, ExecutionError> = try {
            match result {
                CollectedExecutionResult::Value(value) => value
                    .map(|v| v.into_value_container(&mut state))
                    .transpose()?,
                _ => unreachable!("Expected root result"),
            }
        };

        let root_result = match root_result {
            Ok(value) => value,
            Err(error) => return yield Err(error),
        };

        yield Ok(ExecutionInterrupt::External(
            ExternalExecutionInterrupt::Result(root_result),
        ));
    } else {
        panic!("Execution finished without root result");
    }
}

fn resolve_callable_type_definition(
    mut types: Vec<Type>,
    signature_data: &CallableSignatureData,
) -> CallableTypeDefinition {
    let yeet_type = if signature_data.has_yeet_type {
        Some(Box::new(types.pop().expect("Expected yeet type")))
    } else {
        None
    };
    let return_type = if signature_data.has_return_type {
        Some(Box::new(types.pop().expect("Expected return type")))
    } else {
        None
    };

    let rest_parameter = if signature_data.has_rest_parameter {
        Some((
            signature_data
                .rest_parameter_name
                .as_ref()
                .map(|name| name.0.clone()),
            Box::new(types.pop().expect("Expected rest parameter type")),
        ))
    } else {
        None
    };

    let parameters = signature_data
        .parameter_names
        .iter()
        .zip(types)
        .map(|(name, param_type)| (Some(name.0.clone()), param_type))
        .collect::<Vec<_>>();

    CallableTypeDefinition {
        kind: signature_data.kind,
        requires_async: signature_data.requires_async,
        parameters,
        rest_parameter,
        return_type,
        yeet_type,
    }
}

/// Creates a new reference with the given value or returns the existing reference from the cache.
/// Stores the new reference in the cache.
fn create_new_reference_from_value(
    pointer_address: &PointerAddress,
    runtime: &Runtime,
    value: ValueContainer,
    container_mutability: SharedContainerMutability,
    ref_mutability: ReferenceMutability,
) -> Result<ReferencedSharedContainer, ExecutionError> {
    let memory = &mut runtime.shared_references_cache_refcell().borrow_mut();

    if let Some(reference) = memory.get_reference(pointer_address) {
        return Ok(reference.clone());
    }
    match pointer_address {
        // if self owned was not already in memory, we can't resolve it
        PointerAddress::SelfOwned(_) => {
            Err(CacheValueRetrievalError::ValueNotFoundInCache(
                ValueNotFoundInCacheError(pointer_address.clone()),
            )
            .into())
        }
        PointerAddress::Remote(remote_address) => {
            let base = BaseSharedValueContainer::try_new(
                value,
                TypeDefinition::CoreType(CoreLibBaseTypeId::Any.into()),
                container_mutability,
            )?;

            // Note: safe because we checked if the address already exists in memory before
            let reference = unsafe {
                ReferencedSharedContainer::try_new_remote_from_base_container(
                    base,
                    remote_address.clone(),
                    ref_mutability,
                )
            }
            .map_err(|_err| ExecutionError::InvalidSharedValueType)?;

            // stores the reference in memory, so that we can handle updates from the owner endpoint,
            // assuming that we are subscribed to the reference until we unsubscribe
            memory.register_remote_shared_container(&reference);
            // Also set up observers to send any update back to the owner
            runtime.internal().sync_value_with_owner(
                &SharedContainer::Referenced(reference.clone()),
            )?;

            Ok(reference)
        }
    }
}

/// Tries to resolve a cache value by address from either the execution or runtime cache
fn resolve_cache_value(
    state: &mut RuntimeExecutionState,
    pointer_address: &PointerAddress,
    ownership: SharedContainerOwnership,
) -> Result<SharedContainer, ExecutionError> {
    // first try to get from execution cache
    if let Ok(val) =
        resolve_execution_cache_value(state, pointer_address, ownership)
    {
        Ok(val)
    }
    // else, try to get from runtime cache
    else {
        if let Some(reference) = state
            .runtime
            .shared_references_cache_refcell()
            .borrow()
            .get_reference(pointer_address)
        {
            Ok(SharedContainer::Referenced(reference))
        } else {
            Err(ExecutionError::cache_value_retrieval_error(
                CacheValueRetrievalError::ValueNotFoundInCache(
                    ValueNotFoundInCacheError(pointer_address.clone()),
                ),
            ))
        }
    }
}

fn resolve_execution_cache_value(
    state: &mut RuntimeExecutionState,
    pointer_address: &PointerAddress,
    ownership: SharedContainerOwnership,
) -> Result<SharedContainer, ExecutionError> {
    // try to find in execution context cache
    state
        .shared_value_cache
        .try_get_shared_container_with_ownership(pointer_address, ownership)
        .map_err(ExecutionError::cache_value_retrieval_error)
}
