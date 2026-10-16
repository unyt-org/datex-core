use crate::{
    core_compiler::{
        buffer_provider::BufferProvider,
        core_compilation_context::{
            ByteCursor, CompileInput, CoreCompilationContext,
            DXBWithSharedValues,
        },
        value_compiler::InjectedValueValidationError,
        value_visitor::ValueVisitor,
    },
    global::protocol_structures::injected_values::{
        InjectedValueDeclaration, InjectedValueType, SharedInjectedValueType,
    },
    instruction::{
        instruction_data::InstructionBlockData,
        regular_instruction::RegularInstruction,
    },
    prelude::*,
    shared_values::SharedContainer,
    values::value_container::ValueContainer,
};
use binrw::{BinWrite, io::Write};

/// Compiles injected values into a DXB buffer with shared values
///
/// push list to stack (
///     push list to stack (preamble)
///     list(#0,#2,#1)
/// )
/// original body (#0,#1,#2)
pub fn compile_injected_values(
    instruction_block_data: InstructionBlockData,
    injected_values: Vec<ValueContainer>,
    compile_input: CompileInput,
) -> Result<DXBWithSharedValues, InjectedValueValidationError> {
    if injected_values.is_empty() {
        Ok(DXBWithSharedValues {
            dxb: instruction_block_data.body,
            shared_values: vec![],
        })
    } else {
        let mut context =
            CoreCompilationContext::new(Vec::new(), compile_input);

        validate_injected_value_declaration_for_values(
            &instruction_block_data.injected_values,
            &injected_values,
        )?;

        compile_injected_values_with_context(&mut context, &injected_values);

        let DXBWithSharedValues {
            dxb: preambles_dxb,
            shared_values,
        } = context.compile_to_dxb_with_shared_values();

        // prepend statements block
        let mut cursor = ByteCursor::new(Vec::new());
        RegularInstruction::statements(2, false)
            .write(&mut cursor)
            .unwrap();
        RegularInstruction::push_list_to_stack()
            .write(&mut cursor)
            .unwrap();

        cursor.write_all(&preambles_dxb).unwrap();
        cursor.write_all(&instruction_block_data.body).unwrap();

        Ok(DXBWithSharedValues::new(cursor.into_inner(), shared_values))
    }
}

/// Validates that the injected values match the injected value declarations in the instruction block data
///  - local injected values must be local containers
///  - shared injected move values must be owned shared containers
///  - shared injected ref values must be shared containers (owned or referenced)
///    TODO: check full type constraints for values here in the future, not just ownership
fn validate_injected_value_declaration_for_values(
    injected_value_declarations: &[InjectedValueDeclaration],
    injected_values: &[ValueContainer],
) -> Result<(), InjectedValueValidationError> {
    if injected_value_declarations.len() != injected_values.len() {
        unreachable!(); // length must always match
    }

    for (injected_value_declaration, value_container) in
        injected_value_declarations
            .iter()
            .zip(injected_values.iter())
    {
        match &injected_value_declaration.ty {
            // local injected value expects local container
            InjectedValueType::Local(_ty) => {
                if let ValueContainer::Shared(_) = value_container {
                    return Err(
                        InjectedValueValidationError::ExpectedLocalValue,
                    );
                }
            }
            InjectedValueType::Shared(ty) => match ty {
                // shared injected move expects owned shared container
                SharedInjectedValueType::Move => match value_container {
                    ValueContainer::Shared(SharedContainer::Owned(
                        _owned_container,
                    )) => {}
                    _ => return Err(
                        InjectedValueValidationError::ExpectedOwnedSharedValue,
                    ),
                },
                // shared injected ref expects shared container
                SharedInjectedValueType::Ref
                | SharedInjectedValueType::RefMut => match value_container {
                    ValueContainer::Shared(_container) => {}
                    _ => return Err(
                        InjectedValueValidationError::ExpectedOwnedSharedValue,
                    ),
                },
            },
        }
    }

    Ok(())
}

/// Compiles injected values into a DXB buffer with shared values, using the provided compilation context.
/// # Safety
/// The caller must ensure that minimum a single injected value is provided, as this function assumes that the injected values are not empty.
fn compile_injected_values_with_context(
    compilation_context: &mut CoreCompilationContext,
    injected_values: &[ValueContainer],
) {
    if injected_values.is_empty() {
        unreachable!(); // injected values should not be empty, this function should only be called if there are injected values
    }
    compilation_context
        .write(RegularInstruction::list(injected_values.len() as u32));

    for value_container in injected_values {
        for instruction in compilation_context.visit_shared_container(value_container) {
            compilation_context.write(instruction);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "disassembler")]
mod tests {
    use crate::{
        core_compiler::{
            self,
            core_compilation_context::{CompileInput, DXBWithSharedValues},
            value_compiler::InjectedValueValidationError,
        },
        disassembler::assertions::{assert_instructions_equal, instructions},
        global::{
            protocol_structures::injected_values::{
                InjectedValueDeclaration, InjectedValueType,
                SharedInjectedValueType,
            },
            stack_index::StackIndex,
        },
        instruction::{
            instruction_codes::InstructionCode,
            instruction_data::{
                InstructionBlockData, Int32Data, ListData, MoveWithValue,
                SharedRefWithValue, ShortListData, StatementsData, UInt32Data,
            },
            regular_instruction::RegularInstruction,
        },
        prelude::*,
        runtime::{
            pointer_address_provider::SelfOwnedPointerAddressProvider,
            pointer_availability_lookup::PointerAvailabilityLookup,
        },
        shared_values::{
            OwnedSharedContainer, PointerAddress, ReferenceMutability,
            SharedContainer, SharedContainerMutability,
        },
        values::{
            core_values::endpoint::Endpoint, value_container::ValueContainer,
        },
    };

    fn compile_injected_values_test_with_receivers(
        instruction_block_data: InstructionBlockData,
        injected_values: Vec<ValueContainer>,
        receivers: &[Endpoint],
    ) -> Result<DXBWithSharedValues, InjectedValueValidationError> {
        let pointer_availability_lookup =
            PointerAvailabilityLookup::new(Endpoint::default());
        core_compiler::injected_values::compile_injected_values(
            instruction_block_data,
            injected_values,
            CompileInput {
                pointer_lookup: &pointer_availability_lookup,
                receivers,
            },
        )
    }
    fn compile_injected_values_test(
        instruction_block_data: InstructionBlockData,
        injected_values: Vec<ValueContainer>,
    ) -> Result<DXBWithSharedValues, InjectedValueValidationError> {
        compile_injected_values_test_with_receivers(
            instruction_block_data,
            injected_values,
            &[],
        )
    }

    #[test]
    fn remote_execution_no_injected_values() {
        let exec_block_data = InstructionBlockData {
            injected_value_count: 0,
            length: 0,
            injected_values: vec![],
            body: vec![InstructionCode::NULL as u8],
        };
        let res = compile_injected_values_test(exec_block_data, vec![])
            .unwrap()
            .dxb;
        assert_instructions_equal!(&res, (RegularInstruction::Null))
    }

    #[test]
    fn remote_execution_with_injected_ref_value() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let owned_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Immutable,
                address_provider,
            );
        let owned_address = owned_container.pointer_address().clone();

        let exec_block_data = InstructionBlockData {
            injected_value_count: 1,
            length: 1,
            injected_values: vec![InjectedValueDeclaration {
                index: StackIndex(1),
                ty: InjectedValueType::Shared(SharedInjectedValueType::Ref),
            }],
            body: vec![InstructionCode::NULL as u8],
        };
        let res = compile_injected_values_test(
            exec_block_data,
            vec![ValueContainer::Shared(SharedContainer::Referenced(
                owned_container.derive_with_max_mutability(),
            ))],
        )
        .unwrap()
        .dxb;

        assert_instructions_equal!(
            &res,
               (
                    RegularInstruction::statements_with_children(false, instructions!(
                        RegularInstruction::PushListToStack,
                        RegularInstruction::statements_with_children(false, instructions!(
                            // injected values preamble
                            RegularInstruction::PushListToStack,
                            RegularInstruction::statements_with_children(false, instructions!(
                                RegularInstruction::PushToStack,
                                RegularInstruction::SharedRefWithValue(SharedRefWithValue {
                                    address: owned_address,
                                    ref_mutability: ReferenceMutability::Immutable,
                                    container_mutability: SharedContainerMutability::Immutable
                                }),
                                RegularInstruction::Int32(Int32Data(42)),
                                RegularInstruction::list_with_children(instructions!(
                                    RegularInstruction::TakeStackValue(StackIndex(0)),
                                )),
                            )),
                            // remote execution preamble
                            RegularInstruction::list_with_children(instructions!(
                                RegularInstruction::GetStackValueSharedRef(StackIndex(0)),
                            )),
                        )),
                        // original body
                        RegularInstruction::Null,
                    )),
               )
        );
    }

    #[test]
    fn remote_execution_multiple_ref_values() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let shared_value1 = SharedContainer::Referenced(
            SharedContainer::new_owned_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Immutable,
                address_provider,
            )
            .derive_reference_with_max_mutability(),
        );
        let shared_value2 = SharedContainer::Referenced(
            SharedContainer::new_owned_with_inferred_allowed_type(
                100,
                SharedContainerMutability::Mutable,
                address_provider,
            )
            .derive_reference_with_max_mutability(),
        );
        let owned_address_1 = match shared_value1.pointer_address().clone() {
            PointerAddress::SelfOwned(address) => address,
            _ => unreachable!(),
        };
        let owned_address_2 = match shared_value2.pointer_address().clone() {
            PointerAddress::SelfOwned(address) => address,
            _ => unreachable!(),
        };

        let exec_block_data = InstructionBlockData {
            injected_value_count: 2,
            length: 1,
            injected_values: vec![
                InjectedValueDeclaration {
                    index: StackIndex(0),
                    ty: InjectedValueType::Shared(SharedInjectedValueType::Ref),
                },
                InjectedValueDeclaration {
                    index: StackIndex(1),
                    ty: InjectedValueType::Shared(
                        SharedInjectedValueType::RefMut,
                    ),
                },
            ],
            body: vec![InstructionCode::NULL as u8],
        };
        let res = compile_injected_values_test(
            exec_block_data,
            vec![
                ValueContainer::Shared(shared_value1),
                ValueContainer::Shared(shared_value2),
            ],
        )
        .unwrap()
        .dxb;

        assert_instructions_equal!(
            &res,
            (
                RegularInstruction::statements_with_children(false, instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(false, instructions!(
                        // injected values preamble
                        RegularInstruction::PushListToStack,
                        RegularInstruction::statements_with_children(false, instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(SharedRefWithValue {
                                address: owned_address_1,
                                ref_mutability: ReferenceMutability::Immutable,
                                container_mutability: SharedContainerMutability::Immutable
                            }),
                            RegularInstruction::Int32(Int32Data(42)),
                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(SharedRefWithValue {
                                address: owned_address_2,
                                ref_mutability: ReferenceMutability::Mutable,
                                container_mutability: SharedContainerMutability::Mutable
                            }),
                            RegularInstruction::Int32(Int32Data(100)),
                            RegularInstruction::list_with_children(instructions!(
                                RegularInstruction::TakeStackValue(StackIndex(0)),
                                RegularInstruction::TakeStackValue(StackIndex(1)),
                            )),
                        )),

                        // remote execution preamble
                        RegularInstruction::list_with_children(instructions!(
                            RegularInstruction::GetStackValueSharedRef(StackIndex(0)),
                            RegularInstruction::GetStackValueSharedRefMut(StackIndex(1)),
                        )),
                    )),

                    // original body
                    RegularInstruction::Null,
                )),
            )
        );
    }

    #[test]
    fn remote_execution_with_injected_moved_value() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let shared_value =
            SharedContainer::new_owned_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Immutable,
                address_provider,
            );

        let owned_address = match shared_value.pointer_address().clone() {
            PointerAddress::SelfOwned(address) => address,
            _ => unreachable!(),
        };

        let exec_block_data = InstructionBlockData {
            injected_value_count: 1,
            length: 1,
            injected_values: vec![InjectedValueDeclaration {
                index: StackIndex(0),
                ty: InjectedValueType::Shared(SharedInjectedValueType::Move),
            }],
            body: vec![InstructionCode::NULL as u8],
        };
        let res = compile_injected_values_test(
            exec_block_data,
            vec![ValueContainer::Shared(shared_value)],
        )
        .unwrap()
        .dxb;

        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // injected values preamble
                            RegularInstruction::PushListToStack,
                            RegularInstruction::statements_with_children(
                                false,
                                instructions!(
                                    RegularInstruction::PushToStack,
                                    RegularInstruction::MoveWithValue(
                                        MoveWithValue {
                                            mutability: SharedContainerMutability::Immutable,
                                            previous_address: owned_address
                                        }
                                    ),
                                    RegularInstruction::Int32(Int32Data(42)),
                                    RegularInstruction::list_with_children(
                                        instructions!(
                                            RegularInstruction::TakeStackValue(
                                                StackIndex(0)
                                            ),
                                        )
                                    ),
                                )
                            ),
                            // remote execution preamble
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    ),
                                )
                            ),
                        )
                    ),
                    // original body
                    RegularInstruction::Null,
                )
            ),)
        );
    }

    #[test]
    fn remote_execution_moved_value_and_ref() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let shared_value1 =
            SharedContainer::new_owned_with_inferred_allowed_type(
                42,
                SharedContainerMutability::Immutable,
                address_provider,
            );

        let shared_value1_address =
            match shared_value1.pointer_address().clone() {
                PointerAddress::SelfOwned(address) => address,
                _ => unreachable!(),
            };

        let shared_value2 =
            SharedContainer::new_owned_with_inferred_allowed_type(
                100,
                SharedContainerMutability::Immutable,
                address_provider,
            );

        let shared_value2_address =
            match shared_value2.pointer_address().clone() {
                PointerAddress::SelfOwned(address) => address,
                _ => unreachable!(),
            };

        let exec_block_data = InstructionBlockData {
            injected_value_count: 2,
            length: 1,
            injected_values: vec![
                InjectedValueDeclaration {
                    index: StackIndex(0),
                    ty: InjectedValueType::Shared(
                        SharedInjectedValueType::Move,
                    ),
                },
                InjectedValueDeclaration {
                    index: StackIndex(1),
                    ty: InjectedValueType::Shared(SharedInjectedValueType::Ref),
                },
            ],
            body: vec![InstructionCode::NULL as u8],
        };
        let res = compile_injected_values_test(
            exec_block_data,
            vec![
                ValueContainer::Shared(shared_value1),
                ValueContainer::Shared(SharedContainer::Referenced(
                    shared_value2.derive_reference_with_max_mutability(),
                )),
            ],
        )
        .unwrap()
        .dxb;

        assert_instructions_equal!(
            &res,
            (
                RegularInstruction::statements_with_children(false, instructions!(
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(false, instructions!(
                        // injected values preamble
                        RegularInstruction::PushListToStack,
                        RegularInstruction::statements_with_children(false, instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability: SharedContainerMutability::Immutable,
                                previous_address: shared_value1_address
                            }),
                            RegularInstruction::Int32(Int32Data(42)),

                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(SharedRefWithValue {
                                address: shared_value2_address,
                                ref_mutability: ReferenceMutability::Immutable,
                                container_mutability: SharedContainerMutability::Immutable
                            }),
                            RegularInstruction::Int32(Int32Data(100)),

                            RegularInstruction::list_with_children(instructions!(
                                RegularInstruction::TakeStackValue(StackIndex(0)),
                                RegularInstruction::TakeStackValue(StackIndex(1)),
                            )),
                        )),

                        // remote execution preamble
                        RegularInstruction::list_with_children(instructions!(
                            RegularInstruction::TakeStackValue(StackIndex(0)),
                            RegularInstruction::GetStackValueSharedRef(StackIndex(1)),
                        )),
                    )),

                    // original body
                    RegularInstruction::Null,
                )),
            )
        );
    }
}
