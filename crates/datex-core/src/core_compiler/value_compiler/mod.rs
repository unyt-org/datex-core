use crate::{
    core_compiler::{
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
        core_value_with_classification::CoreValueWithClassification,
        core_values::{
            decimal::{Decimal, typed_decimal::TypedDecimal},
            instant::Instant,
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
    core_compiler::{
        core_compilation_context::{ByteCursor, CoreCompilationContext},
        to_instructions::{ToInstructions, yield_all, yield_all_collected},
    },
    instruction::Instruction,
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
    CoreCompilationContext::new(compile_input)
        .compile_to_dxb_with_shared_values(value_container)
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
    CoreCompilationContext::new(compile_input)
        .compile_to_dxb_with_shared_values(&ApplyWithArgs {
            callee: RegularInstruction::get_core_lib_value(
                CoreLibId::Value(CoreLibValueId::Panic).into(),
            ),
            args: vec![ValueContainer::Local(panic_value.into())],
        })
        .dxb
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyWithArgs {
    callee: RegularInstruction,
    args: Vec<ValueContainer>,
}

impl ToInstructions for ApplyWithArgs {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen {
            yield_all!(append_apply(
                visitor,
                self.callee.clone(),
                self.args.clone()
            ));
        })
    }
}

gen fn append_apply<'a>(
    visitor: &'a mut dyn ValueVisitor,
    callee: RegularInstruction,
    args: Vec<ValueContainer>,
) -> crate::instruction::BytecodeItem {
    yield RegularInstruction::apply(args.len() as u8).into();
    for arg in args {
        yield_all_collected!(arg.to_bytecode_item(visitor)); // FIXME: don't collect
    }
    yield callee.into();
    return;
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
            to_instructions::ToInstructions,
            value_compiler::compile_value,
            value_visitor::{ValueVisitor, compile_instructions_to_dxb},
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
        let value = Value::core_with_classification(
            CoreValue::Null,
            ValueClassification::new_with_tag(ValueTag {
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
        let value = Value::core_with_classification(
            CoreValue::Null,
            ValueClassification::new_with_tag(ValueTag {
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

        let dxb = compile_instructions_to_dxb(
            context.shared_value_tracking.get_mut(),
            &shared_container,
        );

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

        let dxb = context.append_preamble_to_dxb(dxb).dxb;

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
        let dxb = compile_instructions_to_dxb(
            context.shared_value_tracking.get_mut(),
            &shared_container,
        );

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

        let dxb = context.append_preamble_to_dxb(dxb).dxb;

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
        let dxb = compile_instructions_to_dxb(
            context.shared_value_tracking.get_mut(),
            &shared_container,
        );

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

        let dxb = context.append_preamble_to_dxb(dxb).dxb;

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

        let dxb = compile_instructions_to_dxb(
            context.shared_value_tracking.get_mut(),
            &shared_container,
        );

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

        let dxb = context.append_preamble_to_dxb(dxb).dxb;

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
        let dxb = compile_instructions_to_dxb(
            context.shared_value_tracking.get_mut(),
            &local,
        );

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

        let dxb = context.append_preamble_to_dxb(dxb).dxb;

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
        let shared_container = ValueContainer::Shared(shared);
        let dxb = compile_instructions_to_dxb(
            context.shared_value_tracking.get_mut(),
            &shared_container,
        );

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

        let dxb = context.append_preamble_to_dxb(dxb).dxb;

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
