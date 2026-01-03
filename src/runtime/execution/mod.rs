use crate::global::protocol_structures::instructions::*;
use crate::global::slots::InternalSlot;
use crate::libs::core::{CoreLibPointerId, get_core_lib_type_reference};
use crate::references::reference::Reference;
use crate::runtime::RuntimeInternal;
use crate::runtime::execution::context::ExecutionMode;
use crate::runtime::execution::context::RemoteExecutionContext;
use crate::runtime::execution::execution_loop::interrupts::{ExecutionInterrupt, ExternalExecutionInterrupt, InterruptResult};
use crate::stdlib::rc::Rc;
use crate::traits::apply::Apply;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::unreachable;
pub use errors::*;
pub use execution_input::ExecutionInput;
pub use execution_input::ExecutionOptions;
pub use memory_dump::*;
use num_enum::TryFromPrimitive;

pub mod context;
mod errors;
mod execution_input;
pub mod execution_loop;
pub mod macros;
mod memory_dump;

pub fn execute_dxb_sync(
    input: ExecutionInput,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let runtime_internal = input.runtime.clone();
    let (interrupt_provider, execution_loop) = input.execution_loop();

    for output in execution_loop {
        match output? {
            ExternalExecutionInterrupt::Result(result) => return Ok(result),
            ExternalExecutionInterrupt::ResolvePointer(address) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(get_pointer_value(
                        &runtime_internal,
                        address,
                    )?),
                )
            }
            ExternalExecutionInterrupt::ResolveLocalPointer(address) => {
                // TODO #401: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(get_local_pointer_value(
                        &runtime_internal,
                        address,
                    )?),
                );
            }
            ExternalExecutionInterrupt::ResolveInternalPointer(address) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(Some(
                        get_internal_pointer_value(address)?,
                    )),
                );
            }
            ExternalExecutionInterrupt::GetInternalSlotValue(slot) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(get_internal_slot_value(
                        &runtime_internal,
                        slot,
                    )?),
                );
            }
            ExternalExecutionInterrupt::Apply(callee, args) => {
                let res = handle_apply(&callee, &args)?;
                interrupt_provider
                    .provide_result(InterruptResult::ResolvedValue(res));
            }
            _ => return Err(ExecutionError::RequiresAsyncExecution),
        }
    }

    Err(ExecutionError::RequiresAsyncExecution)
}

pub async fn execute_dxb(
    input: ExecutionInput<'_>,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let runtime_internal = input.runtime.clone();
    let (interrupt_provider, execution_loop) = input.execution_loop();

    for output in execution_loop {
        match output? {
            ExternalExecutionInterrupt::Result(result) => return Ok(result),
            ExternalExecutionInterrupt::ResolvePointer(address) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(get_pointer_value(
                        &runtime_internal,
                        address,
                    )?),
                );
            }
            ExternalExecutionInterrupt::ResolveLocalPointer(address) => {
                // TODO #402: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(get_local_pointer_value(
                        &runtime_internal,
                        address,
                    )?),
                );
            }
            ExternalExecutionInterrupt::ResolveInternalPointer(address) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(Some(
                        get_internal_pointer_value(address)?,
                    )),
                );
            }
            ExternalExecutionInterrupt::RemoteExecution(receivers, body) => {
                if let Some(runtime) = &runtime_internal {
                    // assert that receivers is a single endpoint
                    // TODO #230: support advanced receivers
                    let receiver_endpoint = receivers
                        .to_value()
                        .borrow()
                        .cast_to_endpoint()
                        .unwrap();
                    let mut remote_execution_context =
                        RemoteExecutionContext::new(
                            receiver_endpoint,
                            ExecutionMode::Static,
                        );
                    let res = RuntimeInternal::execute_remote(
                        runtime.clone(),
                        &mut remote_execution_context,
                        body,
                    )
                    .await?;
                    interrupt_provider
                        .provide_result(InterruptResult::ResolvedValue(res));
                } else {
                    return Err(ExecutionError::RequiresRuntime);
                }
            }
            ExternalExecutionInterrupt::GetInternalSlotValue(slot) => {
                interrupt_provider.provide_result(
                    InterruptResult::ResolvedValue(get_internal_slot_value(
                        &runtime_internal,
                        slot,
                    )?),
                );
            }
            ExternalExecutionInterrupt::Apply(callee, args) => {
                let res = handle_apply(&callee, &args)?;
                interrupt_provider
                    .provide_result(InterruptResult::ResolvedValue(res));
            }
            ExternalExecutionInterrupt::SetProperty {
                mut target,
                key,
                value,
            } => {
                if let Some(runtime) = &runtime_internal {
                    target.try_set_property(
                        0, // TODO: set correct source id
                        &runtime.memory,
                        key,
                        value
                    )?;
                }
                else {
                    return Err(ExecutionError::RequiresRuntime);
                }
                
            }
        }
    }

    unreachable!("Execution loop should always return a result");
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

fn get_internal_slot_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    slot: u32,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        // convert slot to InternalSlot enum
        let slot = InternalSlot::try_from_primitive(slot)
            .map_err(|_| ExecutionError::SlotNotAllocated(slot))?;
        let res = match slot {
            InternalSlot::ENDPOINT => {
                Some(ValueContainer::from(runtime.endpoint.clone()))
            }
        };
        Ok(res)
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

fn get_pointer_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    address: RawFullPointerAddress,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        let memory = runtime.memory.borrow();
        let resolved_address =
            memory.get_pointer_address_from_raw_full_address(address);
        // convert slot to InternalSlot enum
        Ok(memory
            .get_reference(&resolved_address)
            .map(|r| ValueContainer::Reference(r.clone())))
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

fn get_internal_pointer_value(
    address: RawInternalPointerAddress,
) -> Result<ValueContainer, ExecutionError> {
    let core_lib_id =
        CoreLibPointerId::try_from(&PointerAddress::Internal(address.id));
    core_lib_id
        .map_err(|_| ExecutionError::ReferenceNotFound)
        .map(|id| {
            ValueContainer::Reference(Reference::TypeReference(
                get_core_lib_type_reference(id),
            ))
        })
}

fn get_local_pointer_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    address: RawLocalPointerAddress,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        // convert slot to InternalSlot enum
        Ok(runtime
            .memory
            .borrow()
            .get_reference(&PointerAddress::Local(address.id))
            .map(|r| ValueContainer::Reference(r.clone())))
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

#[cfg(test)]
mod tests {
    use crate::stdlib::assert_matches::assert_matches;
    use crate::stdlib::vec;

    use super::*;
    use crate::compiler::scope::CompilationScope;
    use crate::compiler::{CompileOptions, compile_script};
    use crate::global::instruction_codes::InstructionCode;
    use crate::logger::init_logger_debug;
    use crate::runtime::execution::context::ExecutionContext;
    use crate::runtime::execution::context::LocalExecutionContext;
    use crate::runtime::execution::execution_input::ExecutionOptions;
    use crate::stdlib::string::ToString;
    use crate::stdlib::vec::Vec;
    use crate::traits::structural_eq::StructuralEq;
    use crate::traits::value_eq::ValueEq;
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::integer::Integer;
    use crate::values::core_values::integer::typed_integer::TypedInteger;
    use crate::values::core_values::list::List;
    use crate::values::core_values::map::Map;
    use crate::{assert_structural_eq, assert_value_eq, datex_list};
    use log::{debug, info};

    fn execute_datex_script_debug(
        datex_script: &str,
    ) -> Option<ValueContainer> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context =
            ExecutionInput::new(&dxb, ExecutionOptions { verbose: true }, None);
        execute_dxb_sync(context).unwrap_or_else(|err| {
            core::panic!("Execution failed: {err}");
        })
    }

    fn execute_datex_script_debug_unbounded(
        datex_script_parts: impl Iterator<Item = &'static str>,
    ) -> impl Iterator<Item = Result<Option<ValueContainer>, ExecutionError>>
    {
        gen move {
            let datex_script_parts = datex_script_parts.collect::<Vec<_>>();
            let mut execution_context = ExecutionContext::Local(
                LocalExecutionContext::new(ExecutionMode::unbounded()),
            );
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
                    CompileOptions::new_with_scope(compilation_scope),
                )
                .unwrap();
                compilation_scope = new_compilation_scope;
                yield execution_context.execute_dxb_sync(&dxb)
            }
        }
    }

    fn assert_unbounded_input_matches_output(
        input: Vec<&'static str>,
        expected_output: Vec<Option<ValueContainer>>,
    ) {
        let input = input.into_iter();
        let expected_output = expected_output.into_iter();
        for (result, expected) in
            execute_datex_script_debug_unbounded(input.into_iter())
                .zip(expected_output.into_iter())
        {
            let result = result.unwrap();
            assert_eq!(result, expected);
        }
    }

    fn execute_datex_script_debug_with_error(
        datex_script: &str,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context =
            ExecutionInput::new(&dxb, ExecutionOptions { verbose: true }, None);
        execute_dxb_sync(context)
    }

    fn execute_datex_script_debug_with_result(
        datex_script: &str,
    ) -> ValueContainer {
        execute_datex_script_debug(datex_script).unwrap()
    }

    fn execute_dxb_debug(
        dxb_body: &[u8],
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let context = ExecutionInput::new(
            dxb_body,
            ExecutionOptions { verbose: true },
            None,
        );
        execute_dxb_sync(context)
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
        let list: List = result.to_value().borrow().cast_to_list().unwrap();
        assert_eq!(list.len(), 0);
        assert_eq!(result, Vec::<ValueContainer>::new().into());
        assert_eq!(result, ValueContainer::from(Vec::<ValueContainer>::new()));
    }

    #[test]
    fn list() {
        let result = execute_datex_script_debug_with_result("[1, 2, 3]");
        let list: List = result.to_value().borrow().cast_to_list().unwrap();
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
        init_logger_debug();
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
        assert_eq!(result, Decimal::from_string("1.5").unwrap().into());
        assert_structural_eq!(result, ValueContainer::from(1.5));
    }

    #[test]
    fn decimal_and_integer() {
        let result = execute_datex_script_debug_with_result("-2341324.0");
        assert_eq!(result, Decimal::from_string("-2341324").unwrap().into());
        assert!(!result.structural_eq(&ValueContainer::from(-2341324)));
    }

    #[test]
    fn integer() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("2");
        assert_eq!(result, Integer::from(2).into());
        assert_ne!(result, 2_u8.into());
        assert_structural_eq!(result, ValueContainer::from(2_i8));
    }

    #[test]
    fn typed_integer() {
        init_logger_debug();
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
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("{x: 1, y: 2, z: 42}");
        let map: CoreValue = result.clone().to_value().borrow().clone().inner;
        let map: Map = map.try_into().unwrap();

        // form and size
        assert_eq!(map.to_string(), "{\"x\": 1, \"y\": 2, \"z\": 42}");
        assert_eq!(map.size(), 3);

        info!("Map: {:?}", map);

        // access by key
        assert_eq!(map.get("x"), Ok(&Integer::from(1).into()));
        assert_eq!(map.get("y"), Ok(&Integer::from(2).into()));
        assert_eq!(map.get("z"), Ok(&Integer::from(42).into()));

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
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("{}");
        let map: CoreValue = result.clone().to_value().borrow().clone().inner;
        let map: Map = map.try_into().unwrap();

        // form and size
        assert_eq!(map.to_string(), "{}");
        assert_eq!(map.size(), 0);

        info!("Map: {:?}", map);
    }

    #[test]
    fn statements() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("1; 2; 3");
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn single_terminated_statement() {
        init_logger_debug();
        let result = execute_datex_script_debug("1;");
        assert_eq!(result, None);
    }

    #[test]
    fn const_declaration() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("const x = 42; x");
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn const_declaration_with_addition() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = 1 + 2; x");
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn var_assignment_inside_scope() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("var x = 0; [x = 42, 2, x]");
        let expected =
            datex_list![Integer::from(42), Integer::from(2), Integer::from(42)];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn deref() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &42; *x");
        assert_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn ref_assignment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &mut 42; x");
        assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(42)));
    }

    #[test]
    fn ref_add_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x += 1",
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(43)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x += 1; x",
        );

        assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(43)));
    }

    #[test]
    fn ref_sub_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x -= 1",
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(41)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x -= 1; x",
        );

        // FIXME #414 due to addition the resulting value container of the slot
        // is no longer a reference but a value what is incorrect.
        // assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(41)));
    }

    #[test]
    fn endpoint_slot() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_error("#endpoint");
        assert_matches!(result.unwrap_err(), ExecutionError::RequiresRuntime);
    }

    #[test]
    fn shebang() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("#!datex\n42");
        assert_eq!(result, Integer::from(42).into());
    }

    #[test]
    fn single_line_comment() {
        init_logger_debug();
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
        init_logger_debug();
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
        )
    }

    #[test]
    fn continuous_execution_multiple_external_interrupts() {
        assert_unbounded_input_matches_output(
            vec!["1", "integer", "integer"],
            vec![
                Some(Integer::from(1).into()),
                Some(ValueContainer::Reference(Reference::TypeReference(
                    get_core_lib_type_reference(CoreLibPointerId::Integer(
                        None,
                    )),
                ))),
                Some(ValueContainer::Reference(Reference::TypeReference(
                    get_core_lib_type_reference(CoreLibPointerId::Integer(
                        None,
                    )),
                ))),
            ],
        )
    }
}
