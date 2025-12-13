use crate::global::protocol_structures::instructions::*;
use crate::global::slots::InternalSlot;
use crate::libs::core::{get_core_lib_type_reference, CoreLibPointerId};
use crate::references::reference::{Reference};
use crate::runtime::RuntimeInternal;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::ToString;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::traits::apply::Apply;
use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::{ValueContainer};
use core::cell::RefCell;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::unreachable;
use num_enum::TryFromPrimitive;
use datex_core::runtime::execution::context::RemoteExecutionContext;
pub use execution_input::ExecutionOptions;
pub use execution_input::ExecutionInput;
pub use errors::*;
pub use memory_dump::*;
use crate::runtime::execution::execution_loop::{execute_loop, ExecutionStep, InterruptProvider};

mod stack;
pub mod macros;
mod execution_input;
mod errors;
mod memory_dump;
pub mod context;
mod execution_loop;



pub fn execute_dxb_sync(
    input: ExecutionInput,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let interrupt_provider = Rc::new(RefCell::new(None));
    let runtime_internal =
        input.context.borrow_mut().runtime_internal().clone();

    for output in execute_loop(input, interrupt_provider.clone()) {
        match output? {
            ExecutionStep::Return(result) => return Ok(result),
            ExecutionStep::ResolvePointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveLocalPointer(address) => {
                // TODO #401: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_local_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveInternalPointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(
                        get_internal_pointer_value(address)?,
                    ));
            }
            ExecutionStep::GetInternalSlot(slot) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_internal_slot_value(
                        &runtime_internal,
                        slot,
                    )?));
            }
            _ => return Err(ExecutionError::RequiresAsyncExecution),
        }
    }

    Err(ExecutionError::RequiresAsyncExecution)
}

pub async fn execute_dxb(
    input: ExecutionInput<'_>,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let interrupt_provider = Rc::new(RefCell::new(None));
    let runtime_internal =
        input.context.borrow_mut().runtime_internal().clone();

    for output in execute_loop(input, interrupt_provider.clone()) {
        match output? {
            ExecutionStep::Return(result) => return Ok(result),
            ExecutionStep::ResolvePointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveLocalPointer(address) => {
                // TODO #402: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_local_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveInternalPointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(
                        get_internal_pointer_value(address)?,
                    ));
            }
            ExecutionStep::RemoteExecution(receivers, body) => {
                if let Some(runtime) = &runtime_internal {
                    // assert that receivers is a single endpoint
                    // TODO #230: support advanced receivers
                    let receiver_endpoint = receivers
                        .to_value()
                        .borrow()
                        .cast_to_endpoint()
                        .unwrap();
                    let mut remote_execution_context =
                        RemoteExecutionContext::new(receiver_endpoint, true);
                    let res = RuntimeInternal::execute_remote(
                        runtime.clone(),
                        &mut remote_execution_context,
                        body,
                    )
                        .await?;
                    *interrupt_provider.borrow_mut() =
                        Some(InterruptProvider::Result(res));
                } else {
                    return Err(ExecutionError::RequiresRuntime);
                }
            }
            ExecutionStep::GetInternalSlot(slot) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_internal_slot_value(
                        &runtime_internal,
                        slot,
                    )?));
            }
            _ => core::todo!("#99 Undescribed by author."),
        }
    }

    unreachable!("Execution loop should always return a result");
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
) -> Result<Option<ValueContainer>, ExecutionError> {
    let core_lib_id =
        CoreLibPointerId::try_from(&PointerAddress::Internal(address.id));
    core_lib_id
        .map_err(|_| ExecutionError::ReferenceNotFound)
        .map(|id| {
            Some(ValueContainer::Reference(Reference::TypeReference(
                get_core_lib_type_reference(id),
            )))
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
    use crate::compiler::{compile_script, CompileOptions};
    use crate::global::instruction_codes::InstructionCode;
    use crate::logger::init_logger_debug;
    use crate::traits::structural_eq::StructuralEq;
    use crate::{assert_structural_eq, assert_value_eq, datex_list};
    use datex_core::values::core_values::integer::typed_integer::TypedInteger;
    use log::{debug, info};
    use crate::runtime::execution::execution_input::ExecutionOptions;

    fn execute_datex_script_debug(
        datex_script: &str,
    ) -> Option<ValueContainer> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context = ExecutionInput::new_with_dxb_and_options(
            &dxb,
            ExecutionOptions { verbose: true },
        );
        execute_dxb_sync(context).unwrap_or_else(|err| {
            core::panic!("Execution failed: {err}");
        })
    }

    fn execute_datex_script_debug_with_error(
        datex_script: &str,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context = ExecutionInput::new_with_dxb_and_options(
            &dxb,
            ExecutionOptions { verbose: true },
        );
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
        let context = ExecutionInput::new_with_dxb_and_options(
            dxb_body,
            ExecutionOptions { verbose: true },
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
    fn invalid_scope_close() {
        let result = execute_dxb_debug(&[
            InstructionCode::SCOPE_START.into(),
            InstructionCode::SCOPE_END.into(),
            InstructionCode::SCOPE_END.into(),
        ]);
        assert!(core::matches!(
            result,
            Err(ExecutionError::InvalidProgram(
                InvalidProgramError::InvalidScopeClose
            ))
        ));
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

        let result = execute_datex_script_debug_with_result("2big");
        assert_eq!(result, TypedInteger::Big(Integer::from(2)).into());
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
        assert_eq!(map.get("x"), Ok(&Integer::from(1i8).into()));
        assert_eq!(map.get("y"), Ok(&Integer::from(2i8).into()));
        assert_eq!(map.get("z"), Ok(&Integer::from(42i8).into()));

        // structural equality checks
        let expected_se: Map = Map::from(vec![
            ("x".to_string(), 1.into()),
            ("y".to_string(), 2.into()),
            ("z".to_string(), 42.into()),
        ]);
        assert_structural_eq!(map, expected_se);

        // strict equality checks
        let expected_strict: Map = Map::from(vec![
            ("x".to_string(), Integer::from(1_u32).into()),
            ("y".to_string(), Integer::from(2_u32).into()),
            ("z".to_string(), Integer::from(42_u32).into()),
        ]);
        debug!("Expected map: {expected_strict}");
        debug!("Map result: {map}");
        // FIXME #104 type information gets lost on compile
        // assert_eq!(result, expected.into());
    }

    #[test]
    fn val_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("const x = 42; x");
        assert_eq!(result, Integer::from(42i8).into());
    }

    #[test]
    fn val_assignment_with_addition() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = 1 + 2; x");
        assert_eq!(result, Integer::from(3i8).into());
    }

    #[test]
    fn val_assignment_inside_scope() {
        init_logger_debug();
        // FIXME #412: This should be probably disallowed (we can not use x in this scope due to hoisting behavior)
        let result =
            execute_datex_script_debug_with_result("[const x = 42, 2, x]");
        let expected = datex_list![
            Integer::from(42i8),
            Integer::from(2i8),
            Integer::from(42i8)
        ];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn deref() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &42; *x");
        assert_eq!(result, ValueContainer::from(Integer::from(42i8)));
    }

    #[test]
    fn ref_assignment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &mut 42; x");
        assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(42i8)));
    }

    #[test]
    fn ref_add_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x += 1",
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(43i8)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x += 1; x",
        );

        // FIXME #413 due to addition the resulting value container of the slot
        // is no longer a reference but a value what is incorrect.
        // assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(43i8)));
    }

    #[test]
    fn ref_sub_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x -= 1",
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(41i8)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x -= 1; x",
        );

        // FIXME #414 due to addition the resulting value container of the slot
        // is no longer a reference but a value what is incorrect.
        // assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(41i8)));
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
        assert_eq!(result, Integer::from(42i8).into());
    }

    #[test]
    fn single_line_comment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("// this is a comment\n42");
        assert_eq!(result, Integer::from(42i8).into());

        let result = execute_datex_script_debug_with_result(
            "// this is a comment\n// another comment\n42",
        );
        assert_eq!(result, Integer::from(42i8).into());
    }

    #[test]
    fn multi_line_comment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "/* this is a comment */\n42",
        );
        assert_eq!(result, Integer::from(42i8).into());

        let result = execute_datex_script_debug_with_result(
            "/* this is a comment\n   with multiple lines */\n42",
        );
        assert_eq!(result, Integer::from(42i8).into());

        let result = execute_datex_script_debug_with_result("[1, /* 2, */ 3]");
        let expected = datex_list![Integer::from(1i8), Integer::from(3i8)];
        assert_eq!(result, expected.into());
    }
}
