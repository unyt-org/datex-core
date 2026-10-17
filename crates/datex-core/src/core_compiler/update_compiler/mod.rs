//! Compiler for update operations on shared containers into DXB instructions.

use crate::{
    core_compiler::{
        core_compilation_context::{
            CompileInput, CoreCompilationContext, DXBWithSharedValues,
        },
        value_visitor::ValueVisitor,
    },
    global::stack_index::StackIndex,
    instruction::{
        instruction_data::SharedRef, regular_instruction::RegularInstruction,
    },
    prelude::*,
    shared_values::{
        ReferenceMutability, SharedContainer, SharedContainerMutability,
    },
    value_updates::update_data::{
        AppendEntryUpdateData, DeleteEntryUpdateData, ListSpliceUpdateData,
        ReplaceUpdateData, SetEntryUpdateData, UpdateData, UpdateOperation,
    },
    values::value_container::ValueContainer,
};
use crate::core_compiler::to_instructions::{yield_all, ToInstructions};
use crate::instruction::Instruction;

/// Compiles an update operation on a shared container into DXB
/// e.g. [UpdateData::SetEntry] to $x.a = b;
pub fn compile_updates(
    container: &SharedContainer,
    updates: &[&UpdateData],
    compile_input: CompileInput,
) -> DXBWithSharedValues {
    CoreCompilationContext::new(compile_input).compile_to_dxb_with_shared_values(
        &(container, updates)
    )
}

impl ToInstructions for (&SharedContainer, &[&UpdateData]) {
    fn to_instructions<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a> {
        let (container, updates) = self;
        Box::new(gen move {
            yield_all!(append_updates(visitor, container, updates));
        })
    }
}

/// Appends a list of update operations on a shared container
/// The updates are appended as a single statement block, with the shared container being pushed to the stack first, followed by each update operation.
gen fn append_updates<'a>(
    visitor: &mut dyn ValueVisitor,
    container: &SharedContainer,
    updates: &[&UpdateData],
) -> Instruction {
    let statements_count = 1 + updates.len() as u32; // 1 for the push to stack, and 1 for each update operation

    yield RegularInstruction::statements(statements_count, true).into();
    yield RegularInstruction::push_to_stack().into();
    yield RegularInstruction::shared_ref(SharedRef {
        address: container.pointer_address(),
        ref_mutability: ReferenceMutability::Mutable, // can always be upgraded to mutable since the executing endpoint is the owner
        container_mutability: SharedContainerMutability::Mutable, // must be mutable for updates
    }).into();

    for update in updates {
        // TODO append update.path
        yield_all!(append_update_operation(visitor, update.operation()));
    }
}

/// Appends a single update operation on a shared container
gen fn append_update_operation<'a>(
    visitor: &mut dyn ValueVisitor,
    operation: &UpdateOperation,
) -> Instruction {
    match operation {
        UpdateOperation::SetEntry(data) => yield_all!(append_set_entry(visitor, data)),
        UpdateOperation::AppendEntry(data) => yield_all!(append_append_entry(visitor, data)),
        UpdateOperation::Replace(data) => yield_all!(append_replace(visitor, data)),
        UpdateOperation::Clear => yield_all!(append_clear(visitor)),
        UpdateOperation::DeleteEntry(data) => yield_all!(append_delete_entry(visitor, data)),
        UpdateOperation::ListSplice(data) => yield_all!(append_list_splice(visitor, data)),
        UpdateOperation::Increment(_data) => {
            todo!()
        }
        UpdateOperation::Decrement(_data) => {
            todo!()
        }
    }

    return;
}

/// Appends a set entry operation on a shared container
gen fn append_set_entry<'a>(
    visitor: &mut dyn ValueVisitor,
    set_entry_update_data: &SetEntryUpdateData,
) -> Instruction {
    // key
    yield_all!(append_set_property_value_key(visitor, &set_entry_update_data.key));
    // value
    yield_all!(set_entry_update_data.value.to_instructions(visitor));
    // target
    yield RegularInstruction::borrow_stack_value(StackIndex(0)).into();

    return;
}

/// Appends a replace operation on a shared container
gen fn append_replace<'a>(
    visitor: &mut dyn ValueVisitor,
    replace_update_data: &ReplaceUpdateData,
) -> Instruction {
    yield RegularInstruction::set_shared_container_value().into();
    yield_all!(replace_update_data.value.to_instructions(visitor));
    // target
    yield RegularInstruction::borrow_stack_value(StackIndex(0)).into();
    return;
}

/// Appends an append entry operation on a shared container
gen fn append_append_entry<'a>(
    visitor: &mut dyn ValueVisitor,
    append_entry_update_data: &AppendEntryUpdateData,
) -> Instruction {
    yield RegularInstruction::append_entry().into();
    yield_all!(append_entry_update_data.value.to_instructions(visitor));

    // target
    yield RegularInstruction::borrow_stack_value(StackIndex(0)).into();
    return;
}

/// Appends a list splice operation on a shared container
gen fn append_list_splice<'a>(
    visitor: &mut dyn ValueVisitor,
    list_splice_update_data: &ListSpliceUpdateData,
) -> Instruction {
    yield RegularInstruction::splice(
        list_splice_update_data.start,
        list_splice_update_data.delete_count,
        list_splice_update_data.items.len() as u32,
    ).into();

    for item in &list_splice_update_data.items {
        yield_all!(item.to_instructions(visitor));
    }

    // target
    yield RegularInstruction::borrow_stack_value(StackIndex(0)).into();

    return;
}

/// Appends a clear operation on a shared container
gen fn append_clear(visitor: &mut dyn ValueVisitor) -> Instruction {
    yield RegularInstruction::clear().into();

    // target
    yield RegularInstruction::borrow_stack_value(StackIndex(0)).into();
    return;
}

/// Appends a delete entry operation on a shared container
gen fn append_delete_entry(
    visitor: &mut dyn ValueVisitor,
    _delete_entry_update_data: &DeleteEntryUpdateData,
) -> Instruction {
    todo!()
}

/// Appends a set property operation on a shared container, based on the provided value key.
pub gen fn append_set_property_value_key<'a>(
    visitor: &mut dyn ValueVisitor,
    value_key: &ValueContainer,
) -> Instruction {
    if let Some(string) = value_key.try_as_str() {
        yield RegularInstruction::set_entry_text(string.to_string()).into();
    }
    else if let Some(index) = value_key.try_as::<u32>() { // TODO: integer conversion
        yield RegularInstruction::set_entry_index(*index).into();
    } else {
        yield RegularInstruction::set_entry_dynamic().into();
        yield_all!(value_key.to_instructions(visitor));
    }

    return;
}

#[cfg(test)]
#[cfg(feature = "disassembler")]
mod tests {
    use crate::{
        core_compiler::{
            core_compilation_context::CompileInput,
            update_compiler::compile_updates,
        },
        disassembler::assertions::{assert_instructions_equal, instructions},
        global::stack_index::StackIndex,
        instruction::{
            instruction_data::{SharedRef, ShortTextData, UInt8Data},
            regular_instruction::RegularInstruction,
        },
        prelude::*,
        runtime::{
            pointer_address_provider::SelfOwnedPointerAddressProvider,
            pointer_availability_lookup::PointerAvailabilityLookup,
        },
        shared_values::{
            ReferenceMutability, SharedContainer, SharedContainerMutability,
        },
        value_updates::update_data::{
            SetEntryUpdateData, UpdateData, UpdateOperation,
        },
        values::{
            core_values::map::Map,
            value_container::{ValueContainer},
        },
    };

    #[test]
    fn test_compile_update_set_entry() {
        let container = SharedContainer::new_owned_with_inferred_allowed_type(
            Map::from(vec![(
                ValueContainer::from("test_key"),
                ValueContainer::from(42u8),
            )]),
            SharedContainerMutability::Mutable,
            &mut SelfOwnedPointerAddressProvider::default(),
        );

        let address = container.pointer_address();

        let update_data = UpdateData::new(UpdateOperation::set_entry(
            ValueContainer::from("test_key"),
            ValueContainer::from(100u8),
        ));

        let lookup = PointerAvailabilityLookup::default();

        let compile_input = CompileInput::new(&lookup, &[]);
        let dxb_with_shared_values =
            compile_updates(&container, &[&update_data], compile_input);

        assert_instructions_equal!(
            &dxb_with_shared_values.dxb,
            (RegularInstruction::statements_with_children(
                true,
                instructions!(
                    RegularInstruction::PushToStack.with_children(
                        instructions!(RegularInstruction::SharedRef(
                            SharedRef {
                                address,
                                ref_mutability: ReferenceMutability::Mutable,
                                container_mutability:
                                    SharedContainerMutability::Mutable,
                            }
                        ),)
                    ),
                    RegularInstruction::SetEntryText(ShortTextData(
                        "test_key".to_string()
                    ))
                    .with_children(instructions!(
                        RegularInstruction::UInt8(UInt8Data(100)),
                        RegularInstruction::BorrowStackValue(StackIndex(0)),
                    ))
                )
            ))
        )
    }

    #[test]
    fn test_compile_update_replace() {
        let container = SharedContainer::new_owned_with_inferred_allowed_type(
            ValueContainer::from(42u8),
            SharedContainerMutability::Mutable,
            &mut SelfOwnedPointerAddressProvider::default(),
        );

        let address = container.pointer_address();

        let update_data = UpdateData::new(UpdateOperation::replace(
            ValueContainer::from(100u8),
        ));

        let lookup = PointerAvailabilityLookup::default();

        let compile_input = CompileInput::new(&lookup, &[]);
        let dxb_with_shared_values =
            compile_updates(&container, &[&update_data], compile_input);

        assert_instructions_equal!(
            &dxb_with_shared_values.dxb,
            (RegularInstruction::statements_with_children(
                true,
                instructions!(
                    RegularInstruction::PushToStack.with_children(
                        instructions!(RegularInstruction::SharedRef(
                            SharedRef {
                                address,
                                ref_mutability: ReferenceMutability::Mutable,
                                container_mutability:
                                    SharedContainerMutability::Mutable,
                            }
                        ),)
                    ),
                    RegularInstruction::SetSharedContainerValue.with_children(
                        instructions!(
                            RegularInstruction::UInt8(UInt8Data(100)),
                            RegularInstruction::BorrowStackValue(StackIndex(0))
                        )
                    )
                )
            ))
        )
    }
}
