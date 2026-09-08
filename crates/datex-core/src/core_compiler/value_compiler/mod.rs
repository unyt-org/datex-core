use crate::{
    core_compiler::{
        buffer_provider::BufferProvider,
        core_compilation_context::{CompileInput, DXBWithSharedValues},
        type_compiler::append_type_instruction,
        value_visitor::ValueVisitor,
    },
    instruction::{
        instruction_codes::InstructionCode,
        regular_instruction::RegularInstruction,
    },
    utils::buffers::{append_i16, append_i32},
    values::{
        core_value::CoreValue,
        core_values::{
            Instant,
            decimal::{Decimal, typed_decimal::TypedDecimal},
            integer::{Integer, typed_integer::TypedInteger},
        },
        value::Value,
        value_container::ValueContainer,
    },
};
use binrw::{
    BinWrite,
    io::{Cursor, Write},
};

use crate::{
    core_compiler::core_compilation_context::{
        ByteCursor, CoreCompilationContext,
    },
    instruction::{
        Instruction,
        instruction_data::{
            CallableData, CallableDataBody, CallableSignatureData,
            ShortTextData,
        },
    },
    libs::core::{
        core_lib_id::{CoreLibId, CoreLibIdIndex},
        type_id::CoreLibTypeId,
        value_id::CoreLibValueId,
    },
    prelude::*,
    runtime::execution::ExecutionError,
    shared_values::{
        PointerAddress, ReferenceMutability, SharedContainer,
        SharedContainerOwnership, traits::SharedContainerCommon,
    },
    values::{
        core_values::callable::CallableBody,
        value::value_classification::{ValueClassification, ValueTag},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum InjectedValueValidationError {
    ExpectedOwnedSharedValue,
    ExpectedSharedValue,
    ExpectedLocalValue,
}

impl From<InjectedValueValidationError> for ExecutionError {
    fn from(error: InjectedValueValidationError) -> ExecutionError {
        match error {
            InjectedValueValidationError::ExpectedOwnedSharedValue => {
                ExecutionError::ExpectedOwnedSharedValue
            }
            InjectedValueValidationError::ExpectedSharedValue => {
                ExecutionError::ExpectedSharedValue
            }
            InjectedValueValidationError::ExpectedLocalValue => {
                ExecutionError::ExpectedLocalValue
            }
        }
    }
}

/// Compiles a given value container to a DXB body
/// For local values, the value is just serialized
/// For shared values, a reference with maximum mutability is serialized (no move)
pub fn compile_value_container(
    value_container: &ValueContainer,
    compile_input: CompileInput,
) -> DXBWithSharedValues {
    let mut context =
        CoreCompilationContext::new(Vec::with_capacity(256), compile_input);
    context.visit_value_container(value_container);
    context.into_dxb_with_shared_values()
}

pub fn compile_value(
    value: Value,
    compile_input: CompileInput,
) -> DXBWithSharedValues {
    compile_value_container(&ValueContainer::Local(value), compile_input)
}

// TODO: add struct for panics
pub fn compile_panic(
    panic_value: String,
    compile_input: CompileInput,
) -> Vec<u8> {
    let mut context =
        CoreCompilationContext::new(Vec::with_capacity(256), compile_input);

    append_apply(
        &mut context,
        RegularInstruction::get_core_lib_value(
            CoreLibId::Value(CoreLibValueId::Panic).into(),
        ),
        vec![ValueContainer::Local(panic_value.into())],
    );

    context.into_dxb_with_shared_values().dxb
}

/// Appends a shared container to the buffer by registering it in the shared value tracking and appending the stack index
#[deprecated(note = "Use ToInstructions trait instead")]
pub fn append_shared_container_from_preamble(
    context: &mut CoreCompilationContext,
    shared_container: &SharedContainer,
) {
    let ownership = shared_container.ownership();
    let index = context
        .shared_value_tracking
        .borrow_mut()
        .register_shared_value(shared_container);
    context.write(match ownership {
        SharedContainerOwnership::Owned => {
            RegularInstruction::take_stack_value(index)
        }
        SharedContainerOwnership::Referenced(
            ReferenceMutability::Immutable,
        ) => RegularInstruction::get_stack_value_shared_ref(index),
        SharedContainerOwnership::Referenced(ReferenceMutability::Mutable) => {
            RegularInstruction::get_stack_value_shared_ref_mut(index)
        }
    });
}

pub fn append_local_pointer_address(
    cursor: &mut ByteCursor,
    local_address: [u8; 5],
) {
    cursor.write_all(&local_address).unwrap();
}

/// Compiles a value container to the buffer of the provided context
#[deprecated(note = "Use ToInstructions trait instead")]
pub fn append_value_container<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    value_container: &ValueContainer,
) {
    context.visit_value_container(value_container);
}

/// Compiles a value to the buffer of the provided context
#[deprecated(note = "Use ToInstructions trait instead")]
pub fn append_value<'ctx, T: BufferProvider + ValueVisitor<'ctx> + 'ctx>(
    context: &mut T,
    value: &Value,
) {
    // append classified type information
    match &value.classification {
        // unit tagged value (e.g. #Example)
        ValueClassification::Tag(ValueTag { tag, is_empty }) => {
            context.write(RegularInstruction::tagged_value(
                tag.clone(),
                *is_empty,
            ));
            if *is_empty {
                return;
            }; // early return, don't append null value; TODO: assert that value is actually null?
        }
        // entity value
        ValueClassification::Entity(entity_type) => {
            context.write(RegularInstruction::EntityValue(
                entity_type.pointer_address(),
            ));
        }
        // impls value
        ValueClassification::Impls(_impls) => {
            todo!(
                "Compiling values with Impls classification is not yet implemented"
            );
        }
        // no classification, just append the value
        ValueClassification::None => {}
    }

    let _: () = match &value.inner {
        CoreValue::Type(ty) => {
            if let Some(core_id) = ty.try_as_core_lib_type() {
                append_get_core_lib_value(
                    context.cursor_mut(),
                    CoreLibId::Type(core_id),
                );
            } else {
                context.write(RegularInstruction::type_expression());
                context.visit_type(ty);
            }
        }
        CoreValue::Callable(callable) => {
            let (body, injected_values) = match &callable.body {
                CallableBody::DatexBytecode(datex_bytecode) => (
                    CallableDataBody {
                        injected_value_count: datex_bytecode
                            .injected_values
                            .len()
                            as u32,
                        length: datex_bytecode.body.len() as u32,
                        body: datex_bytecode.body.clone(),
                    },
                    datex_bytecode.injected_values.clone(), // FIXME avoid clone!
                ),
                _ => (
                    CallableDataBody {
                        injected_value_count: 0,
                        length: 0,
                        body: vec![],
                    },
                    vec![],
                ),
            };

            context.write(RegularInstruction::Callable(CallableData {
                signature: CallableSignatureData {
                    name: ShortTextData(
                        callable.name.clone().unwrap_or_default(),
                    ),
                    kind: callable.signature.kind,
                    requires_async: callable.signature.requires_async,
                    parameter_count: callable.signature.parameters.len() as u8,
                    has_rest_parameter: callable
                        .signature
                        .rest_parameter
                        .is_some(),
                    has_return_type: callable.signature.return_type.is_some(),
                    has_yeet_type: callable.signature.yeet_type.is_some(),
                    parameter_names: callable
                        .signature
                        .parameters
                        .iter()
                        .map(|(name, _)| {
                            ShortTextData(name.clone().unwrap_or_default())
                        })
                        .collect(),
                    rest_parameter_name: callable
                        .signature
                        .rest_parameter
                        .as_ref()
                        .map(|(name, _)| {
                            ShortTextData(name.clone().unwrap_or_default())
                        }),
                },
                body,
            }));

            // add parameter types
            for (_, param) in &callable.signature.parameters {
                context.visit_type(param);
            }
            // add rest parameter type
            if let Some((_, param)) = &callable.signature.rest_parameter {
                context.visit_type(param);
            }
            // add return type
            if let Some(ty) = &callable.signature.return_type {
                context.visit_type(ty);
            }
            // add yield type
            if let Some(ty) = &callable.signature.yeet_type {
                context.visit_type(ty);
            }

            for value in injected_values {
                context.visit_value_container(&value);
            }
        }
        CoreValue::Integer(integer) => {
            // NOTE: we might optimize this later, but using INT with big integer encoding
            // for all integers for now
            // let integer = integer.to_smallest_fitting();
            // append_encoded_integer(buffer, &integer);
            context.write(RegularInstruction::integer(integer.clone()));
        }
        CoreValue::TypedInteger(integer) => {
            append_encoded_integer(context.cursor_mut(), integer)
        }

        CoreValue::Endpoint(endpoint) => {
            context.write(RegularInstruction::endpoint(endpoint.clone()));
        }
        CoreValue::Decimal(decimal) => {
            append_decimal(context.cursor_mut(), decimal)
        }
        CoreValue::TypedDecimal(val) => {
            append_encoded_decimal(context.cursor_mut(), val)
        }
        CoreValue::Boolean(val) => append_boolean(context.cursor_mut(), val.0),
        CoreValue::Null => context.write(RegularInstruction::null()),
        CoreValue::Text(val) => {
            context.write(RegularInstruction::text(val.0.clone()))
        }
        CoreValue::List(val) => {
            // if list size < 256, use SHORT_LIST
            context.write(RegularInstruction::list(val.len()));

            for item in val.into_iter() {
                context.visit_value_container(item);
            }
        }
        CoreValue::Map(val) => {
            context.write(RegularInstruction::map(val.size() as u32));
            for (key, value) in val.iter() {
                append_key_value_pair(
                    context,
                    &ValueContainer::from(key),
                    value,
                );
            }
        }
        CoreValue::Range(range) => {
            context.write(RegularInstruction::range());
            context.visit_value_container(&range.start);
            context.visit_value_container(&range.end);
        }
        CoreValue::EntityTypeDefinition(_) => {
            todo!()
        }
        CoreValue::Box(inner) => {
            context.write(RegularInstruction::boxed_value());
            context.visit_value_container(inner);
        }
        CoreValue::Uninitialized => {
            panic!("Tried to compile uninitialized value")
        }
        CoreValue::Native(native) => {
            let instructions = (*native.value)
                .to_instructions(context)
                .collect::<Vec<Instruction>>();

            for instruction in instructions {
                match instruction {
                    Instruction::Regular(instruction) => {
                        context.write(instruction);
                    }
                    Instruction::Type(instruction) => {
                        context.write(instruction);
                    }
                }
            }
        }
    };
}

pub fn append_core_type_cast(
    _context: &mut impl BufferProvider,
    _core_lib_type_id: CoreLibTypeId,
) {
    // TODO: append type cast with only id (no need to access shared container)
    todo!()
}

pub fn append_apply<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    callee: RegularInstruction,
    args: Vec<ValueContainer>,
) {
    context.write(RegularInstruction::apply(args.len() as u8));
    for arg in args {
        context.visit_value_container(&arg);
    }
    context.write(callee);
}

/// Appends a boolean value using the TRUE or FALSE instruction
pub fn append_boolean(cursor: &mut ByteCursor, boolean: bool) {
    if boolean {
        RegularInstruction::r#true().write(cursor).unwrap();
    } else {
        RegularInstruction::r#false().write(cursor).unwrap();
    }
}

// Append a decimal value using the DECIMAL instruction code and big decimal encoding
pub fn append_decimal(cursor: &mut ByteCursor, decimal: &Decimal) {
    append_instruction_code(cursor, InstructionCode::DECIMAL);
    append_big_decimal(cursor, decimal);
}

pub fn append_big_decimal(cursor: &mut ByteCursor, decimal: &Decimal) {
    decimal.write_le(cursor).unwrap();
}

/// Appends a typed integer with explicit type casts
pub fn append_typed_integer(
    context: &mut impl BufferProvider,
    integer: &TypedInteger,
) {
    append_core_type_cast(context, CoreLibTypeId::from(integer));
    append_encoded_integer(context.cursor_mut(), integer);
}

/// Appends an encoded integer without explicit type casts
pub fn append_encoded_integer(cursor: &mut ByteCursor, integer: &TypedInteger) {
    let instruction = match integer {
        TypedInteger::I8(val) => RegularInstruction::int8(*val),
        TypedInteger::I16(val) => RegularInstruction::int16(*val),
        TypedInteger::I32(val) => RegularInstruction::int32(*val),
        TypedInteger::I64(val) => RegularInstruction::int64(*val),
        TypedInteger::I128(val) => RegularInstruction::int128(*val),
        TypedInteger::U8(val) => RegularInstruction::uint8(*val),
        TypedInteger::U16(val) => RegularInstruction::uint16(*val),
        TypedInteger::U32(val) => RegularInstruction::uint32(*val),
        TypedInteger::U64(val) => RegularInstruction::uint64(*val),
        TypedInteger::U128(val) => RegularInstruction::uint128(*val),
        TypedInteger::IBig(val) => RegularInstruction::big_integer(val.clone()), // FIXME: no clone
    };

    instruction.write(cursor).unwrap();
}

pub fn append_instant(cursor: &mut ByteCursor, instant: &Instant) {
    RegularInstruction::instant(instant.0)
        .write(cursor)
        .unwrap();
}

/// Appends a typed decimal with explicit type casts
pub fn append_encoded_decimal(cursor: &mut ByteCursor, decimal: &TypedDecimal) {
    fn append_f32_or_f64(cursor: &mut ByteCursor, decimal: &TypedDecimal) {
        match decimal {
            TypedDecimal::F32(val) => {
                RegularInstruction::decimal_f32(val.into_inner())
                    .write(cursor)
                    .unwrap();
            }
            TypedDecimal::F64(val) => {
                RegularInstruction::decimal_f64(val.into_inner())
                    .write(cursor)
                    .unwrap();
            }
            TypedDecimal::Decimal(val) => {
                append_instruction_code(cursor, InstructionCode::DECIMAL_BIG);
                append_big_decimal(cursor, val);
            }
        }
    }

    append_f32_or_f64(cursor, decimal)

    // TODO #635: maybe use this in the future, but type casts are necessary to decide which actual type is represented
    // match decimal.as_integer() {
    //     Some(int) => {
    //         let smallest = smallest_fitting_signed(int as i128);
    //         match smallest {
    //             TypedInteger::I8(val) => {
    //                 append_float_as_i16(buffer, val as i16);
    //             }
    //             TypedInteger::I16(val) => {
    //                 append_float_as_i16(buffer, val);
    //             }
    //             TypedInteger::I32(val) => {
    //                 append_float_as_i32(buffer, val);
    //             }
    //             _ => append_f32_or_f64(buffer, decimal),
    //         }
    //     }
    //     None => append_f32_or_f64(buffer, decimal),
    // }
}

/// Appends a big integer using the BIG_INTEGER instruction code and big integer encoding
pub fn append_big_integer(cursor: &mut ByteCursor, integer: &Integer) {
    integer
        .write_le(cursor)
        .expect("Failed to write big integer");
}

/// Appends a typed decimal with explicit type casts
pub fn append_typed_decimal(
    context: &mut impl BufferProvider,
    decimal: &TypedDecimal,
) {
    append_core_type_cast(context, CoreLibTypeId::from(decimal));
    append_encoded_decimal(context.cursor_mut(), decimal);
}

/// Appends a decimal as an i16 with the DECIMAL_AS_INT_16 instruction code
pub fn append_float_as_i16(cursor: &mut ByteCursor, int: i16) {
    append_instruction_code(cursor, InstructionCode::DECIMAL_AS_INT_16);
    append_i16(cursor, int);
}

/// Appends a decimal as an i32 with the DECIMAL_AS_INT_32 instruction code
pub fn append_float_as_i32(cursor: &mut ByteCursor, int: i32) {
    append_instruction_code(cursor, InstructionCode::DECIMAL_AS_INT_32);
    append_i32(cursor, int);
}

/// Appends a type cast to a core library type, using the GET_CORE_LIB_VALUE instruction with the type id
pub fn append_get_shared_ref(
    context: &mut impl BufferProvider,
    address: PointerAddress,
    mutability: &ReferenceMutability,
) {
    match address {
        PointerAddress::SelfOwned(local_address) => {
            context
                .write(RegularInstruction::get_local_shared_ref(local_address));
        }
        PointerAddress::Remote(address) => match mutability {
            ReferenceMutability::Immutable => {
                context.write(RegularInstruction::request_remote_shared_ref(
                    address,
                ));
            }
            ReferenceMutability::Mutable => {
                context.write(
                    RegularInstruction::request_remote_shared_ref_mut(address),
                );
            }
        },
    }
}

/// Appends a GET_CORE_LIB_VALUE instruction with the given core library id
pub fn append_get_core_lib_value(cursor: &mut ByteCursor, id: CoreLibId) {
    RegularInstruction::get_core_lib_value(CoreLibIdIndex::from(id))
        .write(cursor)
        .unwrap();
}

/// Appends a key-value pair for map entries, optimizing for short text keys
pub fn append_key_value_pair<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    key: &ValueContainer,
    value: &ValueContainer,
) {
    // insert key
    match key {
        // if text, append_key_string, else dynamic
        ValueContainer::Local(Value {
            inner: CoreValue::Text(text),
            ..
        }) => {
            append_key_string(context, &text.0);
        }
        _ => {
            context.write(RegularInstruction::key_value_dynamic());
            context.visit_value_container(key);
        }
    }
    // insert value
    context.visit_value_container(value);
}

/// Appends a key string for map entries, optimizing for short text keys
#[deprecated(note = "Use ToInstructions trait instead")]
pub fn append_key_string<T: BufferProvider>(
    context: &mut T,
    key_string: &String,
) {
    if key_string.len() < 256 {
        context.write(RegularInstruction::key_value_short_text(
            key_string.clone(),
        ));
    } else {
        context.write(RegularInstruction::key_value_dynamic());
        context.write(RegularInstruction::text(key_string.clone()));
    }
}

/// Helper function to directly compile an instruction into a byte vector
pub fn compile_instruction(instruction: impl Into<Instruction>) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    append_instruction(&mut cursor, instruction.into());
    cursor.into_inner()
}
#[deprecated(note = "Use ToInstructions trait instead")]
pub fn append_instruction(cursor: &mut ByteCursor, instruction: Instruction) {
    match instruction {
        Instruction::Regular(instruction) => {
            instruction.write(cursor).unwrap();
        }
        Instruction::Type(instruction) => {
            append_type_instruction(cursor, instruction)
        }
    }
}

#[deprecated(note = "Use context.write(RegularInstruction) instead")]
pub fn append_instruction_code(cursor: &mut ByteCursor, code: InstructionCode) {
    cursor.write_all(&[code as u8]).unwrap();
}

#[cfg(test)]
#[cfg(feature = "disassembler")]
mod tests {
    use crate::{
        core_compiler::{
            core_compilation_context::{
                CompileInput, CoreCompilationContext,
                default_core_compilation_context,
            },
            shared_value_tracking::TrackedValueMetadata,
            value_compiler::compile_value,
            value_visitor::ValueVisitor,
        },
        disassembler::{
            assertions::{assert_instructions_equal, instructions},
            print_disassembled,
        },
        global::stack_index::StackIndex,
        instruction::{
            instruction_data::{
                Int32Data, MoveWithValue, SharedRefWithValue, ShortListData,
                ShortTextData, TaggedValue,
            },
            regular_instruction::RegularInstruction,
        },
        libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
        prelude::*,
        runtime::{
            pointer_address_provider::SelfOwnedPointerAddressProvider,
            pointer_availability_lookup::PointerAvailabilityLookup,
        },
        shared_values::{
            PointerAddress, ReferenceMutability, SharedContainer,
            SharedContainerMutability, traits::SharedContainerCommon,
        },
        types::{
            r#type::Type,
            type_definition::{
                TypeDefinition, tagged_type::TaggedTypeDefinition,
            },
        },
        values::{
            core_value::CoreValue,
            core_values::list::List,
            value::{
                Value,
                value_classification::{ValueClassification, ValueTag},
            },
            value_container::ValueContainer,
        },
    };
    use core::assert_matches;
    use log::info;

    fn compile_value_assert_instructions(
        value: Value,
        expected_instructions: Vec<RegularInstruction>,
    ) {
        let compiled = compile_value(
            value,
            CompileInput {
                pointer_lookup: &PointerAvailabilityLookup::default(),
                receivers: &[],
            },
        );
        assert_instructions_equal!(&compiled.dxb, expected_instructions,);
    }

    #[test]
    fn compile_tagged_empty_value() {
        let value = Value::new(
            CoreValue::Null,
            ValueClassification::Tag(ValueTag {
                tag: "Example".to_string(),
                is_empty: true,
            }),
        );

        compile_value_assert_instructions(
            value,
            vec![RegularInstruction::TaggedValue(TaggedValue {
                tag: ShortTextData("Example".to_string()),
                is_empty: true,
            })],
        );
    }

    #[test]
    fn compile_tagged_value() {
        let value = Value::new(
            CoreValue::Null,
            ValueClassification::Tag(ValueTag {
                tag: "Example".to_string(),
                is_empty: false,
            }),
        );

        compile_value_assert_instructions(
            value,
            vec![
                RegularInstruction::TaggedValue(TaggedValue {
                    tag: ShortTextData("Example".to_string()),
                    is_empty: false,
                }),
                RegularInstruction::Null,
            ],
        );
    }

    #[test]
    fn compile_core_type_value_integer() {
        let value = Value::from(CoreValue::Type(
            TypeDefinition::CoreType(CoreLibTypeId::Base(
                CoreLibBaseTypeId::Integer,
            ))
            .into(),
        ));

        compile_value_assert_instructions(
            value,
            vec![RegularInstruction::GetCoreLibValue(
                CoreLibBaseTypeId::Integer.into(),
            )],
        );
    }

    fn core_compilation_context() -> CoreCompilationContext<'static> {
        unsafe { default_core_compilation_context() }
    }

    #[test]
    fn shared_value_compilation() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let owned_shared =
            SharedContainer::new_owned_with_inferred_allowed_type(
                CoreValue::Null,
                SharedContainerMutability::Immutable,
                &mut provider,
            );
        let owned_shared_clone = owned_shared.clone();

        let pointer_address = match &owned_shared {
            SharedContainer::Owned(owned) => owned.pointer_address().clone(),
            _ => unreachable!(),
        };

        let shared_container = ValueContainer::Shared(owned_shared);
        let mut context = core_compilation_context();

        context.visit_value_container(&shared_container);

        // The address should now be registered in the shared value tracking
        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&owned_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(0),
                ..
            }
        );

        assert_instructions_equal!(
            &context.into_dxb_with_shared_values().dxb,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability:
                                    SharedContainerMutability::Immutable,
                                previous_address: pointer_address,
                            }),
                            RegularInstruction::Null,
                            RegularInstruction::list(1),
                            RegularInstruction::TakeStackValue(StackIndex(0)),
                        )
                    ),
                    RegularInstruction::TakeStackValue(StackIndex(0))
                )
            ),)
        );
    }

    #[test]
    fn shared_value_nested_multiple_children() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let inner_a_shared =
            SharedContainer::new_owned_with_inferred_allowed_type(
                1,
                SharedContainerMutability::Mutable,
                &mut provider,
            );
        let inner_b_shared =
            SharedContainer::new_owned_with_inferred_allowed_type(
                2,
                SharedContainerMutability::Mutable,
                &mut provider,
            );
        let inner_b_shared_clone = inner_b_shared.clone();

        let outer_shared =
            SharedContainer::new_owned_with_inferred_allowed_type(
                List::new(vec![
                    ValueContainer::Shared(SharedContainer::Referenced(
                        inner_a_shared.try_derive_mutable_reference().unwrap(),
                    )),
                    ValueContainer::Shared(inner_b_shared),
                ]),
                SharedContainerMutability::Immutable,
                &mut provider,
            );
        let outer_shared_clone = outer_shared.clone();

        let inner_pointer_address_a = match inner_a_shared.pointer_address() {
            PointerAddress::SelfOwned(owned) => owned,
            _ => unreachable!(),
        };
        let inner_pointer_address_b =
            match inner_b_shared_clone.pointer_address() {
                PointerAddress::SelfOwned(owned) => owned,
                _ => unreachable!(),
            };

        let outer_pointer_address = match &outer_shared {
            SharedContainer::Owned(owned) => owned.pointer_address().clone(),
            _ => unreachable!(),
        };

        let shared_container = ValueContainer::Shared(outer_shared);
        let mut context = core_compilation_context();
        context.visit_value_container(&shared_container);

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&inner_a_shared)
                .unwrap(),
            TrackedValueMetadata::Child { .. }
        );

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&inner_b_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Child { .. }
        );

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&outer_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(0),
                ..
            }
        );

        let dxb = context.into_dxb_with_shared_values().dxb;

        assert_instructions_equal!(
            &dxb,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // val 1
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability:
                                    SharedContainerMutability::Immutable,
                                previous_address: outer_pointer_address,
                            }),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::SharedRefWithValue(
                                        SharedRefWithValue {
                                            address: inner_pointer_address_a,
                                            ref_mutability:
                                                ReferenceMutability::Mutable,
                                            container_mutability:
                                                SharedContainerMutability::Mutable,
                                        }
                                    ).with_children(instructions!(
                                        RegularInstruction::Int32(Int32Data(1))
                                    )),
                                    RegularInstruction::MoveWithValue(MoveWithValue {
                                        mutability: SharedContainerMutability::Mutable,
                                        previous_address: inner_pointer_address_b,
                                    }),
                                    RegularInstruction::Int32(Int32Data(2)),
                                )
                            ),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    ),
                                )
                            ),
                        )
                    ),
                    RegularInstruction::TakeStackValue(StackIndex(0)),
                )
            ),)
        );
    }

    #[test]
    fn shared_value_nested_direct() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let inner_shared =
            SharedContainer::new_owned_with_inferred_allowed_type(
                1,
                SharedContainerMutability::Mutable,
                &mut provider,
            );
        let inner_shared_clone = inner_shared.clone();

        let outer_shared =
            SharedContainer::new_owned_with_inferred_allowed_type(
                ValueContainer::Shared(inner_shared),
                SharedContainerMutability::Immutable,
                &mut provider,
            );
        let outer_shared_clone = outer_shared.clone();

        let inner_pointer_address = match inner_shared_clone.pointer_address() {
            PointerAddress::SelfOwned(owned) => owned,
            _ => unreachable!(),
        };
        let outer_pointer_address = match &outer_shared {
            SharedContainer::Owned(owned) => owned.pointer_address().clone(),
            _ => unreachable!(),
        };

        let shared_container = ValueContainer::Shared(outer_shared);
        let mut context = core_compilation_context();
        context.visit_value_container(&shared_container);

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&inner_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Child { .. }
        );

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&outer_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(0),
                is_self_referencing: false,
                is_known: false,
            }
        );

        let dxb = context.into_dxb_with_shared_values().dxb;

        assert_instructions_equal!(
            &dxb,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // val 1
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability:
                                    SharedContainerMutability::Immutable,
                                previous_address: outer_pointer_address,
                            }),
                            // val 2
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability: SharedContainerMutability::Mutable,
                                previous_address: inner_pointer_address,
                            }),
                            RegularInstruction::Int32(Int32Data(1)),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    ),
                                )
                            ),
                        )
                    ),
                    RegularInstruction::TakeStackValue(StackIndex(0)),
                )
            ),)
        );
    }

    #[test]
    fn shared_ref() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let reference = SharedContainer::Referenced(
            SharedContainer::new_owned_with_inferred_allowed_type(
                5,
                SharedContainerMutability::Immutable,
                &mut provider,
            )
            .derive_immutable_reference(),
        );
        let pointer_address = match reference.pointer_address() {
            PointerAddress::SelfOwned(local_address) => local_address,
            _ => unreachable!(),
        };
        let shared_container = ValueContainer::Shared(reference.clone());
        let mut context = core_compilation_context();

        context.visit_value_container(&shared_container);

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&reference)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(0),
                is_known: false,
                is_self_referencing: false,
            }
        );

        let dxb = context.into_dxb_with_shared_values().dxb;

        assert_instructions_equal!(
            &dxb,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(
                                SharedRefWithValue {
                                    address: pointer_address,
                                    ref_mutability:
                                        ReferenceMutability::Immutable,
                                    container_mutability:
                                        SharedContainerMutability::Immutable,
                                }
                            ),
                            RegularInstruction::Int32(Int32Data(5)),
                            RegularInstruction::list(1),
                            RegularInstruction::TakeStackValue(StackIndex(0)),
                        )
                    ),
                    RegularInstruction::GetStackValueSharedRef(StackIndex(0))
                )
            ),)
        );
    }

    #[test]
    fn local_nested() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let a_shared = SharedContainer::new_owned_with_inferred_allowed_type(
            1,
            SharedContainerMutability::Immutable,
            &mut provider,
        );
        let a_shared_clone = a_shared.clone();
        let a_pointer_address = match &a_shared {
            SharedContainer::Owned(owned) => owned.pointer_address().clone(),
            _ => unreachable!(),
        };

        let b_shared = SharedContainer::new_owned_with_inferred_allowed_type(
            2,
            SharedContainerMutability::Immutable,
            &mut provider,
        );
        let b_shared_clone = b_shared.clone();
        let b_pointer_address = match &b_shared {
            SharedContainer::Owned(owned) => owned.pointer_address().clone(),
            _ => unreachable!(),
        };

        let local = ValueContainer::Local(
            List::new(vec![
                ValueContainer::from("test"),
                ValueContainer::Shared(a_shared),
                ValueContainer::Shared(b_shared),
            ])
            .into(),
        );
        let mut context = core_compilation_context();
        context.visit_value_container(&local);

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&a_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(0),
                is_known: false,
                is_self_referencing: false,
            }
        );
        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&b_shared_clone)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(1),
                is_known: false,
                is_self_referencing: false,
            }
        );

        let dxb = context.into_dxb_with_shared_values().dxb;

        assert_instructions_equal!(
            &dxb,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // val 1
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability:
                                    SharedContainerMutability::Immutable,
                                previous_address: a_pointer_address,
                            }),
                            RegularInstruction::Int32(Int32Data(1)),
                            // val 2
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability:
                                    SharedContainerMutability::Immutable,
                                previous_address: b_pointer_address,
                            }),
                            RegularInstruction::Int32(Int32Data(2)),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    ),
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(1)
                                    ),
                                )
                            )
                        )
                    ),
                    RegularInstruction::list_with_children(instructions!(
                        RegularInstruction::ShortText(ShortTextData(
                            "test".to_string()
                        )),
                        RegularInstruction::TakeStackValue(StackIndex(0)),
                        RegularInstruction::TakeStackValue(StackIndex(1)),
                    )),
                )
            ),)
        );
    }

    #[test]
    fn self_referencing_shared_container() {
        let mut provider = SelfOwnedPointerAddressProvider::default();
        let shared = SharedContainer::new_owned_with_inferred_allowed_type(
            List::default(),
            SharedContainerMutability::Mutable,
            &mut provider,
        );

        // *x = ['mut x]
        {
            let mut shared_vale_container = shared.value_container_mut();
            let list = shared_vale_container.try_as_mut::<List>().unwrap();
            list.push(ValueContainer::Shared(shared.clone()));
        }

        let shared_clone = shared.clone();

        let shared_owned_address = match shared.pointer_address() {
            PointerAddress::SelfOwned(owned) => owned,
            _ => unreachable!(),
        };

        let mut context = core_compilation_context();
        context.visit_value_container(&ValueContainer::Shared(shared));

        assert_matches!(
            context
                .shared_value_tracking
                .borrow()
                .tracked_values
                .get(&shared_clone)
                .unwrap(),
            TrackedValueMetadata::Root {
                index: StackIndex(0),
                is_known: false,
                is_self_referencing: true,
            }
        );

        let dxb = context.into_dxb_with_shared_values().dxb;

        assert_instructions_equal!(
            &dxb,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // x = shared nut [[UNINITIALIZED]]
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability:
                                    SharedContainerMutability::Mutable,
                                previous_address: shared_owned_address,
                            }).with_children(instructions!(
                                RegularInstruction::Uninitialized
                            )),
                            // *x = ['mut x]
                            RegularInstruction::SetSharedContainerValue.with_children(
                                instructions!(
                                    RegularInstruction::list_with_children(
                                        instructions!(
                                            RegularInstruction::GetStackValueSharedRefMut(
                                                StackIndex(0)
                                            ),
                                        )
                                    ),
                                    RegularInstruction::BorrowStackValue(
                                        StackIndex(0)
                                    )
                                )
                            ),
                            // [x]
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    ),
                                )
                            )
                        )
                    ),
                    RegularInstruction::TakeStackValue(StackIndex(0)),
                )
            ),)
        );
    }
}
