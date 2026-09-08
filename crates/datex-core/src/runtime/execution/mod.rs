//! This module contains the implementation of the execution engine which is responsible for executing compiled DATEX bytecode (DXB) and handling interrupts that can occur during execution, such as calling functions, loading pointers, and performing pointer updates.
use crate::{
    core_compiler::InstructionInput,
    instruction::regular_instruction::RegularInstruction,
    libs::core::core_lib_id::CoreLibId,
    prelude::*,
    runtime::{
        Runtime,
        execution::{
            context::{ExecutionMode, RemoteExecutionContext},
            execution_loop::interrupts::{
                ExternalExecutionInterrupt, InterruptResult,
            },
        },
    },
    shared_values::{
        PointerAddress, ReferenceMutability, ReferencedSharedContainer,
        RemotePointerAddress, SelfOwnedPointerAddress, SharedContainer,
    },
    traits::apply::{Apply, ApplyArgument},
    types::{
        entities::entity_impls::EntityImplMethod, entity_type::EntityType,
    },
    values::{
        core_values::endpoint::Endpoint,
        value::{Value, value_classification::ValueClassification},
        value_container::ValueContainer,
    },
};
use core::{cell::Ref, ops::Deref, result::Result, unreachable};
pub use errors::*;
pub use execution_input::{ExecutionInput, ExecutionOptions};
pub use stack_dump::*;

pub mod context;
mod errors;
pub mod execution_input;
pub mod execution_loop;
pub mod macros;
mod stack_dump;

#[cfg(all(test, feature = "std"))]
mod test_remote_execution;

pub fn execute_dxb_sync(
    input: ExecutionInput,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let runtime = input.runtime.clone();
    let (interrupt_provider, execution_loop) = input.execution_loop();

    for output in execution_loop {
        match output? {
            ExternalExecutionInterrupt::SetEndpointProperty {
                endpoint,
                property_name,
                value,
            } => {
                if endpoint.is_local_or_equals_endpoint(runtime.endpoint()) {
                    interrupt_provider.provide_result(
                        InterruptResult::ResolvedValue(
                            runtime
                                .endpoint_properties_mut()
                                .insert(property_name, value),
                        ),
                    );
                } else {
                    return Err(ExecutionError::RequiresAsyncExecution);
                }
            }
            ExternalExecutionInterrupt::GetEndpointProperty {
                endpoint,
                property_name,
            } => {
                if endpoint.is_local_or_equals_endpoint(runtime.endpoint()) {
                    let value = runtime
                        .endpoint_properties()
                        .get(&property_name)
                        .cloned();
                    interrupt_provider.provide_result(
                        InterruptResult::ResolvedValue(Some(
                            ValueContainer::new_from_option(value),
                        )),
                    );
                } else {
                    return Err(ExecutionError::RequiresAsyncExecution);
                }
            }
            ExternalExecutionInterrupt::Result(result) => return Ok(result),
            ExternalExecutionInterrupt::GetReferenceToRemotePointer(
                address,
                mutability,
            ) => interrupt_provider.provide_result(
                InterruptResult::ResolvedValue(
                    get_remote_shared_container_reference(
                        &runtime, address, mutability,
                    )?
                    .map(|v| {
                        ValueContainer::Shared(SharedContainer::Referenced(v))
                    }),
                ),
            ),
            ExternalExecutionInterrupt::GetReferenceToLocalPointer(address) => {
                // TODO #401: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(
                        get_local_pointer_value(&runtime, address).map(|v| {
                            ValueContainer::Shared(SharedContainer::Referenced(
                                v,
                            ))
                        }),
                    ),
                );
            }
            ExternalExecutionInterrupt::GetCoreLibValue(id) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(Some(
                        get_core_lib_value_container(&runtime, id)?,
                    )),
                );
            }
            ExternalExecutionInterrupt::Apply(callee, args) => {
                let res = callee.try_apply_sync_checked(&runtime, args)?;
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValueAndBorrowedArgs(res),
                );
            }
            ExternalExecutionInterrupt::CallMethod(
                callee,
                method_name,
                args,
            ) => {
                let val = callee.value.collapsed_value();
                let entity_type = try_get_entity_type(val.borrow().deref())?;
                let method = try_get_method_data(&entity_type, method_name)?;

                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValueAndBorrowedArgs(
                        try_call_method_sync(
                            callee,
                            method.deref(),
                            args,
                            &runtime,
                        )?,
                    ),
                )
            }
            _ => return Err(ExecutionError::RequiresAsyncExecution),
        }
    }

    Err(ExecutionError::RequiresAsyncExecution)
}

pub async fn execute_dxb(
    input: ExecutionInput,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let runtime = input.runtime.clone();
    let _caller_metadata = input.caller_metadata.clone();
    let (interrupt_provider, execution_loop) = input.execution_loop();

    for output in execution_loop {
        match output? {
            ExternalExecutionInterrupt::SetEndpointProperty {
                endpoint,
                property_name,
                value,
            } => {
                if endpoint.is_local_or_equals_endpoint(runtime.endpoint()) {
                    interrupt_provider.provide_result(
                        InterruptResult::ResolvedValue(
                            runtime
                                .endpoint_properties_mut()
                                .insert(property_name, value),
                        ),
                    );
                } else {
                    interrupt_provider.provide_result(
                        InterruptResult::ResolvedValue(
                            set_remote_endpoint_property(
                                &runtime,
                                endpoint,
                                property_name,
                                value,
                            )
                            .await?,
                        ),
                    );
                }
            }
            ExternalExecutionInterrupt::GetEndpointProperty {
                endpoint,
                property_name,
            } => {
                if endpoint.is_local_or_equals_endpoint(runtime.endpoint()) {
                    let value = runtime
                        .endpoint_properties()
                        .get(&property_name)
                        .cloned();
                    interrupt_provider.provide_result(
                        InterruptResult::ResolvedValue(Some(
                            ValueContainer::new_from_option(value),
                        )),
                    );
                } else {
                    interrupt_provider.provide_result(
                        InterruptResult::ResolvedValue(
                            get_remote_endpoint_property(
                                &runtime,
                                endpoint,
                                property_name,
                            )
                            .await?,
                        ),
                    );
                }
            }
            ExternalExecutionInterrupt::Result(result) => return Ok(result),
            ExternalExecutionInterrupt::GetReferenceToRemotePointer(
                address,
                mutability,
            ) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(
                        get_remote_shared_container_reference(
                            &runtime, address, mutability,
                        )?
                        .map(|v| {
                            ValueContainer::Shared(SharedContainer::Referenced(
                                v,
                            ))
                        }),
                    ),
                );
            }
            ExternalExecutionInterrupt::GetReferenceToLocalPointer(address) => {
                // TODO #402: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(
                        get_local_pointer_value(&runtime, address).map(|v| {
                            ValueContainer::Shared(SharedContainer::Referenced(
                                v,
                            ))
                        }),
                    ),
                );
            }
            ExternalExecutionInterrupt::GetCoreLibValue(id) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(Some(
                        get_core_lib_value_container(&runtime, id)?,
                    )),
                );
            }
            ExternalExecutionInterrupt::RemoteExecution {
                input,
                receivers,
            } => {
                // assert that receivers is a single endpoint
                assert_eq!(receivers.len(), 1);

                let mut remote_execution_context = RemoteExecutionContext::new(
                    receivers,
                    ExecutionMode::Static,
                    runtime.clone(),
                );
                let res = runtime
                    .execute_remote(&mut remote_execution_context, input)
                    .await?;
                interrupt_provider
                    .provide_result(InterruptResult::ResolvedValue(res));
            }
            ExternalExecutionInterrupt::Apply(callee, args) => {
                let res =
                    callee.try_apply_async_checked(&runtime, args).await?;
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValueAndBorrowedArgs(res),
                );
            }
            ExternalExecutionInterrupt::CallMethod(
                callee,
                method_name,
                args,
            ) => {
                let val = callee.value.collapsed_value();
                let entity_type = try_get_entity_type(val.borrow().deref())?;
                let method = try_get_method_data(&entity_type, method_name)?;

                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValueAndBorrowedArgs(
                        try_call_method_async(
                            callee,
                            method.deref(),
                            args,
                            &runtime,
                        )
                        .await?,
                    ),
                )
            }
        }
    }

    unreachable!("Execution loop should always return a result");
}

fn try_get_entity_type(value: &Value) -> Result<EntityType, ExecutionError> {
    match value {
        Value {
            classification: ValueClassification::Entity(entity_type),
            ..
        } => Ok(entity_type.clone()),
        _ => Err(ExecutionError::ExpectedEntityValue),
    }
}

fn try_get_method_data(
    entity_type: &EntityType,
    method_name: String,
) -> Result<Ref<'_, EntityImplMethod>, ExecutionError> {
    Ref::filter_map(entity_type.entity_definition(), |def| {
        def.try_get_method(&method_name)
    })
    .map_err(|_| ExecutionError::MethodNotFound(method_name))
}

fn try_call_method_sync(
    callee: ApplyArgument,
    method: &EntityImplMethod,
    mut args: Vec<ApplyArgument>,
    runtime: &Runtime,
) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ExecutionError> {
    let owner_endpoint = callee.value.owner();

    // prepend callee to args
    args.insert(0, callee);

    // only local calls supported for sync execution
    if !method.call_on_owner
        || owner_endpoint.is_local_or_equals_endpoint(runtime.endpoint())
    {
        let res = method.callable.try_apply_sync_checked(runtime, args)?;
        Ok(res)
    } else {
        Err(ExecutionError::RequiresAsyncExecution)
    }
}

async fn try_call_method_async(
    callee: ApplyArgument,
    method: &EntityImplMethod,
    mut args: Vec<ApplyArgument>,
    runtime: &Runtime,
) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ExecutionError> {
    let owner_endpoint = callee.value.owner();

    // prepend callee to args
    args.insert(0, callee);

    // call locally
    if !method.call_on_owner
        || owner_endpoint.is_local_or_equals_endpoint(runtime.endpoint())
    {
        let res = method
            .callable
            .try_apply_async_checked(runtime, args)
            .await?;
        Ok(res)
    }
    // call on owner endpoint
    else {
        let mut instructions = vec![
            RegularInstruction::call_method(
                method.name().unwrap().clone(), // FIXME: don't use method name here
                args.len() as u8,
            )
            .into(),
        ];
        instructions.extend(
            args.into_iter()
                .map(|v| InstructionInput::ValueContainer(v.value)),
        );

        let res = runtime
            .execute_instructions_remote(vec![owner_endpoint], instructions)
            .await?;

        // FIXME: restore borrowed stack values across remote execution.
        // For now, local borrows are not supported cross endpoint
        Ok((res, vec![]))
    }
}

async fn set_remote_endpoint_property(
    runtime: &Runtime,
    endpoint: Endpoint,
    property_name: String,
    value: ValueContainer,
) -> Result<Option<ValueContainer>, ExecutionError> {
    runtime
        .execute_instructions_remote(
            vec![endpoint],
            vec![
                RegularInstruction::set_entry_text(property_name).into(),
                InstructionInput::ValueContainer(value),
                RegularInstruction::Endpoint(Endpoint::LOCAL).into(),
            ],
        )
        .await
}

async fn get_remote_endpoint_property(
    runtime: &Runtime,
    endpoint: Endpoint,
    property_name: String,
) -> Result<Option<ValueContainer>, ExecutionError> {
    runtime
        .execute_instructions_remote(
            vec![endpoint],
            vec![
                RegularInstruction::get_entry_text(property_name).into(),
                RegularInstruction::Endpoint(Endpoint::LOCAL).into(),
            ],
        )
        .await
}

fn get_remote_shared_container_reference(
    runtime: &Runtime,
    address: RemotePointerAddress,
    _mutability: ReferenceMutability,
) -> Result<Option<ReferencedSharedContainer>, ExecutionError> {
    let address_provider = runtime.pointer_address_provider_mut();
    let memory = runtime.shared_references_cache_refcell().borrow();
    let resolved_address = address_provider.normalize_address(address);
    // convert slot to InternalSlot enum
    // TODO #770: resolve from remote, handle mutability
    Ok(memory.get_reference(&resolved_address))
}

fn get_core_lib_value_container(
    runtime: &Runtime,
    id: CoreLibId,
) -> Result<ValueContainer, ExecutionError> {
    let value = runtime.core_library().value_or_type_by_id(id);
    Ok(ValueContainer::Local(value.clone()))
}

fn get_local_pointer_value(
    runtime: &Runtime,
    address: SelfOwnedPointerAddress,
) -> Option<ReferencedSharedContainer> {
    // convert slot to InternalSlot enum
    runtime
        .shared_references_cache_refcell()
        .borrow()
        .get_reference(&PointerAddress::SelfOwned(address))
}

#[cfg(test)]
#[cfg(feature = "compiler")]
mod tests {
    use super::*;
    use crate::{
        collections::HashMap,
        compiler::{CompileOptions, compile_script, scope::CompilationScope},
        core_compiler::{
            core_compilation_context::DXBWithSharedValues,
            value_compiler::compile_instruction,
        },
        instruction::Instruction,
        libs::core::type_id::CoreLibBaseTypeId,
        prelude::*,
        runtime::{
            Runtime, RuntimeConfig, RuntimeRunner,
            execution::{
                context::{ExecutionContext, LocalExecutionContext},
                execution_input::{ExecutionCallerMetadata, ExecutionOptions},
            },
        },
        shared_values::{
            OwnedSharedContainer, ReferencedSharedContainer, SharedContainer,
            SharedContainerInner, SharedContainerMutability,
            base_shared_value_container::BaseSharedValueContainer,
            traits::SharedContainerCommon,
        },
        traits::{
            structural_eq::{StructuralEq, assert_structural_eq},
            value_eq::{ValueEq, assert_value_eq},
        },
        types::{
            r#type::Type,
            type_definition::{
                TypeDefinition,
                callable::{CallableKind, CallableTypeDefinition},
                tagged_type::TaggedTypeDefinition,
            },
        },
        values::{
            core_value::CoreValue,
            core_values::{
                callable::{Callable, CallableBody, DatexBytecodeCallable},
                decimal::Decimal,
                endpoint::Endpoint,
                integer::{Integer, typed_integer::TypedInteger},
                list::{List, datex_list},
                map::Map,
            },
            value::{
                Value,
                value_classification::{ValueClassification, ValueTag},
            },
        },
    };
    use core::assert_matches;
    use indexmap::IndexMap;
    use log::{debug, info};

    fn execute_datex_script_debug(
        datex_script: &str,
    ) -> Option<ValueContainer> {
        let runtime = Runtime::stub();
        let (dxb, _) = compile_script(
            datex_script,
            CompileOptions::default(),
            runtime.clone(),
        )
        .unwrap();
        let context = ExecutionInput::new(
            DXBWithSharedValues::new(dxb, vec![]),
            ExecutionCallerMetadata::local_default(),
            ExecutionOptions { verbose: true },
            runtime,
        );
        execute_dxb_sync(context).unwrap()
    }

    fn execute_datex_script_debug_unbounded(
        datex_script_parts: impl Iterator<Item = &'static str>,
        runtime: Runtime,
    ) -> impl Iterator<Item = Result<Option<ValueContainer>, ExecutionError>>
    {
        gen move {
            let datex_script_parts = datex_script_parts.collect::<Vec<_>>();
            let mut execution_context =
                ExecutionContext::Local(LocalExecutionContext::new(
                    ExecutionMode::unbounded(),
                    runtime.clone(),
                    ExecutionCallerMetadata::local_default(),
                ));
            let mut compilation_scope =
                CompilationScope::new(ExecutionMode::unbounded());

            let len = datex_script_parts.len();
            for (index, script_part) in
                datex_script_parts.into_iter().enumerate()
            {
                // if last part, compile and return static value if possible
                if index == len - 1 {
                    compilation_scope.mark_as_last_execution();
                }

                let (dxb, new_compilation_scope) = compile_script(
                    script_part,
                    CompileOptions::new(
                        compilation_scope,
                        vec![Endpoint::LOCAL],
                    ),
                    runtime.clone(),
                )
                .unwrap();
                compilation_scope = new_compilation_scope;
                yield execution_context.execute_dxb_sync(
                    DXBWithSharedValues::new(dxb, vec![]),
                    None,
                );
            }
        }
    }

    fn assert_unbounded_input_matches_output(
        input: Vec<&'static str>,
        expected_output: Vec<Option<ValueContainer>>,
        runtime: Runtime,
    ) {
        let input = input.into_iter();
        let expected_output = expected_output.into_iter();
        for (result, expected) in execute_datex_script_debug_unbounded(
            input.into_iter(),
            runtime.clone(),
        )
        .zip(expected_output)
        {
            let result = result.unwrap();
            assert_eq!(result, expected);
        }
    }

    fn execute_datex_script_debug_with_error(
        datex_script: &str,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let runtime = Runtime::stub();
        let (dxb, _) = compile_script(
            datex_script,
            CompileOptions::default(),
            runtime.clone(),
        )
        .unwrap();
        let context = ExecutionInput::new(
            DXBWithSharedValues::new(dxb, vec![]),
            ExecutionCallerMetadata::local_default(),
            ExecutionOptions { verbose: true },
            runtime,
        );
        execute_dxb_sync(context)
    }

    fn execute_datex_script_debug_with_result(
        datex_script: &str,
    ) -> ValueContainer {
        execute_datex_script_debug(datex_script).unwrap()
    }

    async fn execute_datex_script_with_runtime(
        config: RuntimeConfig,
        datex_script: &str,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        RuntimeRunner::new(config)
            .run(async |runtime| {
                let (dxb, _) = compile_script(
                    datex_script,
                    CompileOptions::default(),
                    runtime.clone(),
                )
                .unwrap();
                let context = ExecutionInput::new(
                    DXBWithSharedValues::new(dxb, vec![]),
                    ExecutionCallerMetadata::local_default(),
                    ExecutionOptions { verbose: true },
                    runtime,
                );
                execute_dxb(context).await
            })
            .await
    }

    #[test]
    fn empty_script() {
        assert_eq!(execute_datex_script_debug(""), None);
    }

    #[test]
    fn empty_script_semicolon() {
        assert_eq!(execute_datex_script_debug(";;;"), None);
    }

    #[test]
    fn single_value() {
        assert_eq!(
            execute_datex_script_debug_with_result("42"),
            Integer::from(42i8).into()
        );
    }

    #[test]
    fn single_value_semicolon() {
        assert_eq!(execute_datex_script_debug("42;"), None)
    }

    #[test]
    fn is() {
        let result = execute_datex_script_debug_with_result("1 is 1");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn equality() {
        let result = execute_datex_script_debug_with_result("1 == 1");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("1 == 2");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));

        let result = execute_datex_script_debug_with_result("1 != 2");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("1 != 1");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
        let result = execute_datex_script_debug_with_result("1 === 1");
        assert_eq!(result, true.into());

        assert_structural_eq!(result, ValueContainer::from(true));
        let result = execute_datex_script_debug_with_result("1 !== 2");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("1 !== 1");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn single_value_scope() {
        let result = execute_datex_script_debug_with_result("(42)");
        assert_eq!(result, Integer::from(42i8).into());
        assert_structural_eq!(result, ValueContainer::from(42_u128));
    }

    #[test]
    fn add() {
        let result = execute_datex_script_debug_with_result("1 + 2");
        assert_eq!(result, Integer::from(3i8).into());
        assert_structural_eq!(result, ValueContainer::from(3i8));
    }

    #[test]
    fn nested_scope() {
        let result = execute_datex_script_debug_with_result("1 + (2 + 3)");
        assert_eq!(result, Integer::from(6i8).into());
    }

    #[test]
    fn empty_list() {
        let result = execute_datex_script_debug_with_result("[]");
        let list: &List = result.try_as().unwrap();
        assert_eq!(list.len(), 0);
        assert_eq!(result, Vec::<ValueContainer>::new().into());
        assert_eq!(result, ValueContainer::from(Vec::<ValueContainer>::new()));
    }

    #[test]
    fn empty_tag() {
        let result = execute_datex_script_debug_with_result("#Example");
        if let ValueContainer::Local(value) = result {
            assert_eq!(&value.inner, &CoreValue::Null);
            assert_eq!(
                value.classification(),
                &ValueClassification::Tag(ValueTag {
                    tag: "Example".to_string(),
                    is_empty: true
                })
            )
        } else {
            panic!("Result should be Local value");
        }
    }

    #[test]
    fn empty_with_map() {
        let result =
            execute_datex_script_debug_with_result("#Example {a: true}");
        if let ValueContainer::Local(value) = result {
            assert_eq!(
                &value.inner,
                &CoreValue::Map(Map::structural_with_string_keys(vec![(
                    "a".to_string(),
                    ValueContainer::from(true)
                )]))
            );
            assert_eq!(
                value.classification(),
                &ValueClassification::Tag(ValueTag {
                    tag: "Example".to_string(),
                    is_empty: false
                })
            )
        } else {
            panic!("Result should be Local value");
        }
    }

    #[test]
    fn list() {
        let result = execute_datex_script_debug_with_result("[1, 2, 3]");
        let list: &List = result.try_as().unwrap();
        let expected = datex_list![
            Integer::from(1i8),
            Integer::from(2i8),
            Integer::from(3i8)
        ];
        assert_eq!(list.len(), 3);
        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1, 2, 3]));
        assert_structural_eq!(result, ValueContainer::from(vec![1, 2, 3]));
    }

    #[test]
    fn list_with_nested_scope() {
        let result = execute_datex_script_debug_with_result("[1, (2 + 3), 4]");
        let expected = datex_list![
            Integer::from(1i8),
            Integer::from(5i8),
            Integer::from(4i8)
        ];

        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1_u8, 5_u8, 4_u8]));
        assert_structural_eq!(
            result,
            ValueContainer::from(vec![1_u8, 5_u8, 4_u8])
        );
    }

    #[test]
    fn boolean() {
        let result = execute_datex_script_debug_with_result("true");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("false");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn decimal() {
        let result = execute_datex_script_debug_with_result("1.5");
        assert_eq!(result, Decimal::try_from_string("1.5").unwrap().into());
        assert_structural_eq!(result, ValueContainer::from(1.5));
    }

    #[test]
    fn decimal_and_integer() {
        let result = execute_datex_script_debug_with_result("-2341324.0");
        assert_eq!(
            result,
            Decimal::try_from_string("-2341324").unwrap().into()
        );
        assert!(!result.structural_eq(&ValueContainer::from(-2341324)));
    }

    #[test]
    fn integer() {
        let result = execute_datex_script_debug_with_result("2");
        assert_eq!(result, Integer::from(2).into());
        assert_ne!(result, 2_u8.into());
        assert_structural_eq!(result, ValueContainer::from(2_i8));
    }

    #[test]
    fn typed_integer() {
        let result = execute_datex_script_debug_with_result("-2i16");
        assert_eq!(result, TypedInteger::from(-2i16).into());
        assert_structural_eq!(result, ValueContainer::from(-2_i16));

        let result = execute_datex_script_debug_with_result("2i32");
        assert_eq!(result, TypedInteger::from(2i32).into());
        assert_structural_eq!(result, ValueContainer::from(2_i32));

        let result = execute_datex_script_debug_with_result("-2i64");
        assert_eq!(result, TypedInteger::from(-2i64).into());
        assert_structural_eq!(result, ValueContainer::from(-2_i64));

        let result = execute_datex_script_debug_with_result("2i128");
        assert_eq!(result, TypedInteger::from(2i128).into());
        assert_structural_eq!(result, ValueContainer::from(2_i128));

        let result = execute_datex_script_debug_with_result("2u8");
        assert_eq!(result, TypedInteger::from(2_u8).into());
        assert_structural_eq!(result, ValueContainer::from(2_u8));

        let result = execute_datex_script_debug_with_result("2u16");
        assert_eq!(result, TypedInteger::from(2_u16).into());
        assert_structural_eq!(result, ValueContainer::from(2_u16));

        let result = execute_datex_script_debug_with_result("2u32");
        assert_eq!(result, TypedInteger::from(2_u32).into());
        assert_structural_eq!(result, ValueContainer::from(2_u32));

        let result = execute_datex_script_debug_with_result("2u64");
        assert_eq!(result, TypedInteger::from(2_u64).into());
        assert_structural_eq!(result, ValueContainer::from(2_u64));

        let result = execute_datex_script_debug_with_result("2u128");
        assert_eq!(result, TypedInteger::from(2_u128).into());
        assert_structural_eq!(result, ValueContainer::from(2_u128));

        let result = execute_datex_script_debug_with_result("2ibig");
        assert_eq!(result, TypedInteger::IBig(Integer::from(2)).into());
        assert_structural_eq!(result, ValueContainer::from(2));
    }

    #[test]
    fn null() {
        let result = execute_datex_script_debug_with_result("null");
        assert_eq!(result, ValueContainer::from(CoreValue::Null));
        assert_eq!(result, CoreValue::Null.into());
        assert_structural_eq!(result, ValueContainer::from(CoreValue::Null));
    }

    #[test]
    fn map() {
        let result =
            execute_datex_script_debug_with_result("{x: 1, y: 2, z: 42}");
        let map: CoreValue = result.get_cloned_value().inner;
        let map = map.try_into_value::<Map>().unwrap();

        // form and size
        assert_eq!(map.to_string(), "{\"x\": 1, \"y\": 2, \"z\": 42}");
        assert_eq!(map.size(), 3);

        info!("Map: {:?}", map);

        // access by key
        assert_eq!(map.try_get("x"), Ok(&Integer::from(1).into()));
        assert_eq!(map.try_get("y"), Ok(&Integer::from(2).into()));
        assert_eq!(map.try_get("z"), Ok(&Integer::from(42).into()));

        // structural equality checks
        let expected_se: Map = Map::from(vec![
            ("x".to_string(), 1.into()),
            ("y".to_string(), 2.into()),
            ("z".to_string(), 42.into()),
        ]);
        assert_structural_eq!(map, expected_se);

        // strict equality checks
        let expected_strict: Map = Map::from(vec![
            ("x".to_string(), Integer::from(1).into()),
            ("y".to_string(), Integer::from(2).into()),
            ("z".to_string(), Integer::from(42).into()),
        ]);
        debug!("Expected map: {expected_strict}");
        debug!("Map result: {map}");
        // FIXME #104 type information gets lost on compile
        // assert_eq!(result, expected.into());
    }

    #[test]
    fn empty_map() {
        let result = execute_datex_script_debug_with_result("{}");
        let map: CoreValue = result.clone().get_cloned_value().inner;
        let map = map.try_into_value::<Map>().unwrap();

        // form and size
        assert_eq!(map.to_string(), "{}");
        assert_eq!(map.size(), 0);

        info!("Map: {:?}", map);
    }

    #[test]
    fn statements() {
        let result = execute_datex_script_debug_with_result("1; 2; 3");
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn empty_function() {
        let result =
            execute_datex_script_debug_with_result("function test() ()");
        let callable: Callable = result.try_into_value().unwrap();

        assert_eq!(
            callable,
            Callable {
                name: Some("test".to_string()),
                signature: CallableTypeDefinition {
                    kind: CallableKind::Function,
                    requires_async: false,
                    parameters: vec![],
                    rest_parameter: None,
                    return_type: None,
                    yeet_type: None,
                },
                body: CallableBody::DatexBytecode(DatexBytecodeCallable {
                    requires_async: false,
                    injected_values: vec![],
                    body: compile_instruction(RegularInstruction::statements(
                        0, false
                    )),
                }),
                creator: Endpoint::LOCAL,
            }
        )
    }

    #[test]
    fn function_no_params() {
        let result =
            execute_datex_script_debug_with_result("function test() (null)");
        let callable: Callable = result.try_into_value().unwrap();

        assert_eq!(
            callable,
            Callable {
                name: Some("test".to_string()),
                signature: CallableTypeDefinition {
                    kind: CallableKind::Function,
                    requires_async: false,
                    parameters: vec![],
                    rest_parameter: None,
                    return_type: None,
                    yeet_type: None,
                },
                body: CallableBody::DatexBytecode(DatexBytecodeCallable {
                    requires_async: false,
                    injected_values: vec![],
                    body: compile_instruction(RegularInstruction::Null),
                }),
                creator: Endpoint::LOCAL,
            }
        );
    }

    #[test]
    fn function() {
        let result = execute_datex_script_debug_with_result(
            "function test(a: integer) -> null (null)",
        );
        let callable: Callable = result.try_into_value().unwrap();

        assert_eq!(
            callable,
            Callable {
                name: Some("test".to_string()),
                signature: CallableTypeDefinition {
                    kind: CallableKind::Function,
                    requires_async: false,
                    parameters: vec![(
                        Some("a".to_string()),
                        Type::core(CoreLibBaseTypeId::Integer)
                    )],
                    rest_parameter: None,
                    return_type: Some(Box::new(Type::core(
                        CoreLibBaseTypeId::Null
                    ))),
                    yeet_type: None,
                },
                body: CallableBody::DatexBytecode(DatexBytecodeCallable {
                    requires_async: false,
                    injected_values: vec![],
                    body: compile_instruction(RegularInstruction::Null),
                }),
                creator: Endpoint::LOCAL,
            }
        );
    }

    #[test]
    fn function_call_no_args() {
        let result = execute_datex_script_debug_with_result(
            "function test() -> integer (1 + 2)()",
        );
        let integer: Integer = result.try_into_value().unwrap();
        assert_eq!(integer, Integer::from(3));
    }

    #[test]
    fn function_call_with_arg() {
        let result = execute_datex_script_debug_with_result(
            "function test(x: integer) -> integer (x + 2)(1)",
        );
        let integer: Integer = result.try_into_value().unwrap();
        assert_eq!(integer, Integer::from(3));
    }

    #[test]
    fn function_call_with_arg_and_injected_value() {
        let result = execute_datex_script_debug_with_result(
            "const y = 42; function test(x: integer) -> integer (y + x + 2)(1)",
        );
        let integer: Integer = result.try_into_value().unwrap();
        assert_eq!(integer, Integer::from(45));
    }

    #[test]
    fn single_terminated_statement() {
        let result = execute_datex_script_debug("1;");
        assert_eq!(result, None);
    }

    #[test]
    fn const_declaration() {
        let result = execute_datex_script_debug_with_result("const x = 42; x");
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn const_declaration_with_addition() {
        let result =
            execute_datex_script_debug_with_result("const x = 1 + 2; x");
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn unbox_shared() {
        let result =
            execute_datex_script_debug_with_result("const x = shared 42; *x");
        assert_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn shared_creation_mut_ref_to_mut() {
        let result = execute_datex_script_debug_with_result(
            "const x = 'mut shared mut 42; x",
        );
        assert_matches!(result, ValueContainer::Shared(SharedContainer::Referenced(ref container)) if
            container.container_mutability() == SharedContainerMutability::Mutable &&
            container.reference_mutability() == ReferenceMutability::Mutable
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn shared_creation_immut_ref_to_mut() {
        let result = execute_datex_script_debug_with_result(
            "const x = 'shared mut 42; x",
        );
        assert_matches!(result, ValueContainer::Shared(SharedContainer::Referenced(ref container)) if
            container.container_mutability() == SharedContainerMutability::Mutable &&
            container.reference_mutability() == ReferenceMutability::Immutable
        );

        assert_value_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn shared_creation_immut_ref() {
        let result =
            execute_datex_script_debug_with_result("const x = 'shared 42; x");
        assert_matches!(result, ValueContainer::Shared(SharedContainer::Referenced(ref container)) if
            container.container_mutability() == SharedContainerMutability::Immutable &&
            container.reference_mutability() == ReferenceMutability::Immutable
        );

        assert_value_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn shared_creation_immut() {
        let result =
            execute_datex_script_debug_with_result("const x = shared 42; x");
        assert_matches!(result, ValueContainer::Shared(SharedContainer::Owned(ref container)) if
            container.container_mutability() == SharedContainerMutability::Immutable
        );

        assert_value_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn shared_creation_mut() {
        let result = execute_datex_script_debug_with_result(
            "const x = shared mut 42; x",
        );
        assert_matches!(result, ValueContainer::Shared(SharedContainer::Owned(
            ref container @ OwnedSharedContainer { .. }
        )) if container.container_mutability() == SharedContainerMutability::Mutable);
        assert_value_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn shared_creation_mut_ref_to_immut() {
        let result = execute_datex_script_debug_with_error(
            "const x = 'mut shared 42; x",
        );
        assert_matches!(
            result,
            Err(ExecutionError::MutableReferenceToNonMutableValue)
        );
    }

    #[test]
    fn shared_value_add_assignment() {
        let result = execute_datex_script_debug_with_result(
            "var x = shared mut 42; *x += 1; x",
        );

        assert_value_eq!(result, ValueContainer::from(Integer::from(43)));
        assert_matches!(result, ValueContainer::Shared(..));
        if let ValueContainer::Shared(shared) = &result {
            assert_eq!(
                *shared.inner().base_shared_container().mutability(),
                SharedContainerMutability::Mutable
            );
        } else {
            panic!("Expected shared value");
        }
    }

    #[test]
    fn shared_value_sub_assignment() {
        let result = execute_datex_script_debug_with_result(
            "const x = 'mut shared mut 42; *x -= 1; x",
        );

        assert_matches!(result, ValueContainer::Shared(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(41)));
    }

    #[test]
    fn shared_value_assignment() {
        let result = execute_datex_script_debug_with_result(
            "const x = 'mut shared mut 42; *x = 100; x",
        );

        assert_matches!(result, ValueContainer::Shared(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(100)));
    }

    #[tokio::test]
    #[cfg(feature = "std")]
    async fn env_slot() {
        let res = execute_datex_script_with_runtime(
            RuntimeConfig {
                env: Some(IndexMap::from([(
                    "TEST_ENV_VAR".to_string(),
                    "test_value".to_string(),
                )])),
                ..Default::default()
            },
            "$.env",
        )
        .await
        .unwrap();
        assert!(res.is_some());
        let env = res
            .unwrap()
            .try_into_value::<IndexMap<String, String>>()
            .unwrap();
        assert_eq!(env.get("TEST_ENV_VAR"), Some(&"test_value".into()));
    }

    #[test]
    fn shebang() {
        let result = execute_datex_script_debug_with_result("#!datex\n42");
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn single_line_comment() {
        let result =
            execute_datex_script_debug_with_result("// this is a comment\n42");
        assert_eq!(result, Integer::from(42).into());

        let result = execute_datex_script_debug_with_result(
            "// this is a comment\n// another comment\n42",
        );
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn multi_line_comment() {
        let result = execute_datex_script_debug_with_result(
            "/* this is a comment */\n42",
        );
        assert_eq!(result, Integer::from(42).into());

        let result = execute_datex_script_debug_with_result(
            "/* this is a comment\n   with multiple lines */\n42",
        );
        assert_eq!(result, Integer::from(42).into());

        let result = execute_datex_script_debug_with_result("[1, /* 2, */ 3]");
        let expected = datex_list![Integer::from(1), Integer::from(3)];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn continuous_execution() {
        assert_unbounded_input_matches_output(
            vec!["1", "2"],
            vec![Some(Integer::from(1).into()), Some(Integer::from(2).into())],
            Runtime::stub(),
        )
    }

    #[test]
    fn resolve_core_lib_type_reference() {
        let result = execute_datex_script_debug_with_result("integer");
        assert_eq!(
            result,
            ValueContainer::Local(Value::from(Type::core(
                CoreLibBaseTypeId::Integer,
            )))
        );
    }

    #[test]
    fn continuous_execution_multiple_external_interrupts() {
        let runtime = Runtime::stub();

        assert_unbounded_input_matches_output(
            vec!["1", "integer", "boolean"],
            vec![
                Some(Integer::from(1).into()),
                Some(ValueContainer::Local(Value::from(Type::core(
                    CoreLibBaseTypeId::Integer,
                )))),
                Some(ValueContainer::Local(Value::from(Type::core(
                    CoreLibBaseTypeId::Boolean,
                )))),
            ],
            runtime,
        )
    }

    #[test]
    fn property_text_access() {
        let result =
            execute_datex_script_debug_with_result("var x = {a: 42}; x.a");
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn property_index_access() {
        let result =
            execute_datex_script_debug_with_result("var x = [1,2,3]; x.1");
        assert_eq!(result, Integer::from(2).into());
    }

    #[test]
    fn property_text_update() {
        let result = execute_datex_script_debug_with_result(
            "var x = {a: 42}; x.a = 100; x.a",
        );
        assert_eq!(result, Integer::from(100).into());
    }

    #[test]
    fn nested_stack() {
        let result = execute_datex_script_debug_with_result(
            "var x = 1; var y = (var x = 2; x); [x, y]",
        );
        assert_eq!(
            result,
            List::from(vec![Integer::from(1), Integer::from(2)]).into()
        );
    }
    #[test]
    fn conditional_true_branch() {
        let result =
            execute_datex_script_debug_with_result("if (true) (42) else (43)");
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn conditional_false_branch() {
        let result =
            execute_datex_script_debug_with_result("if (false) (42) else (43)");
        assert_eq!(result, Integer::from(43).into());
    }

    #[test]
    fn conditional_no_else() {
        let result = execute_datex_script_debug("if (false) (42)");
        assert_eq!(result, None);
    }

    #[test]
    fn conditional_nested_if_else() {
        let result = execute_datex_script_debug_with_result(
            "if (true) (1) else if (false) (2) else (3)",
        );
        assert_eq!(result, Integer::from(1).into());
    }

    #[test]
    fn conditional_nested_else_if() {
        let result = execute_datex_script_debug_with_result(
            "if (false) (1) else if (true) (2) else (3)",
        );
        assert_eq!(result, Integer::from(2).into());
    }

    #[test]
    fn conditional_nested_else_fallback() {
        let result = execute_datex_script_debug_with_result(
            "if (false) (1) else if (false) (2) else (3)",
        );
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn conditional_with_variable() {
        let result = execute_datex_script_debug_with_result(
            "const x = 42; if (true) (x) else (0)",
        );
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn conditional_with_variable_else_branch() {
        let result = execute_datex_script_debug_with_result(
            "const x = 99; if (false) (0) else (x)",
        );
        assert_eq!(result, Integer::from(99).into());
    }

    #[test]
    fn conditional_complex_condition() {
        let result = execute_datex_script_debug_with_result(
            "if (1 + 1 == 2) (100) else (200)",
        );
        assert_eq!(result, Integer::from(100).into());
    }

    #[test]
    fn conditional_mutation_in_branch() {
        let script = "
            var x = 1;
            if (true) (x = 2);
            x
            ";
        let result = execute_datex_script_debug_with_result(script);
        assert_eq!(result, Integer::from(2).into());
    }

    #[test]
    fn conditional_mutation_in_false_branch() {
        let script = "
            var x = 1;
            if (false) (x = 2) else (x = 3);
            x
            ";
        let result = execute_datex_script_debug_with_result(script);
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn conditional_results_return() {
        let script = "
            var a = 1;
            var b = 2;
            if (false) (
                a = 2
            ) else (
                a = 3;
                b = 2;
            );
            b
            ";
        let result = execute_datex_script_debug_with_result(script);
        assert_eq!(result, Integer::from(2).into());
    }

    #[test]
    fn conditional_complex_result() {
        let script = "
            var c = 0;
            const b = 5;
            const x = if (b==3) (
                250u8
            ) else (
                130u8
            );
            if (x==250) (
                c = 0u8;
            ) else if (x==130) (
                c = 1u8;
            ) else (
                c = 2u8;
            );
            c
            ";
        let result = execute_datex_script_debug_with_result(script);
        assert_eq!(result, TypedInteger::from(1u8).into());
    }
}
