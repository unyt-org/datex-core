//! Compiler for update operations on shared containers into DXB instructions.

use crate::{
    core_compiler::{
        buffer_provider::BufferProvider,
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

/// Compiles an update operation on a shared container into DXB
/// e.g. [UpdateData::SetEntry] to $x.a = b;
pub fn compile_updates(
    container: &SharedContainer,
    updates: &[&UpdateData],
    compile_input: CompileInput,
) -> DXBWithSharedValues {
    let mut context = CoreCompilationContext::new(
        Vec::with_capacity(updates.len() * 50),
        compile_input,
    );
    append_updates(&mut context, container, updates);
    context.into_dxb_with_shared_values()
}

/// Appends a list of update operations on a shared container
/// The updates are appended as a single statement block, with the shared container being pushed to the stack first, followed by each update operation.
fn append_updates<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    container: &SharedContainer,
    updates: &[&UpdateData],
) {
    let statements_count = 1 + updates.len() as u32; // 1 for the push to stack, and 1 for each update operation

    context.write(RegularInstruction::statements(statements_count, true));
    context.write(RegularInstruction::push_to_stack());
    context.write(RegularInstruction::shared_ref(SharedRef {
        address: container.pointer_address(),
        ref_mutability: ReferenceMutability::Mutable, // can always be upgraded to mutable since the executing endpoint is the owner
        container_mutability: SharedContainerMutability::Mutable, // must be mutable for updates
    }));

    for update in updates {
        // TODO append update.path
        append_update_operation(context, update.operation());
    }
}

/// Appends a single update operation on a shared container
fn append_update_operation<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    operation: &UpdateOperation,
) {
    match operation {
        UpdateOperation::SetEntry(data) => append_set_entry(context, data),
        UpdateOperation::AppendEntry(data) => {
            append_append_entry(context, data)
        }
        UpdateOperation::Replace(data) => append_replace(context, data),
        UpdateOperation::Clear => append_clear(context),
        UpdateOperation::DeleteEntry(data) => {
            append_delete_entry(context, data)
        }
        UpdateOperation::ListSplice(data) => append_list_splice(context, data),
        UpdateOperation::Increment(_data) => {
            todo!()
        }
        UpdateOperation::Decrement(_data) => {
            todo!()
        }
    }
}

/// Appends a set entry operation on a shared container
fn append_set_entry<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    set_entry_update_data: &SetEntryUpdateData,
) {
    // key
    append_set_property_value_key(context, &set_entry_update_data.key);
    // value
    context.visit_value_container(&set_entry_update_data.value);
    // target
    context.write(RegularInstruction::borrow_stack_value(StackIndex(0)));
}

/// Appends a replace operation on a shared container
fn append_replace<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    replace_update_data: &ReplaceUpdateData,
) {
    context.write(RegularInstruction::set_shared_container_value());
    context.visit_value_container(&replace_update_data.value);
    // target
    context.write(RegularInstruction::borrow_stack_value(StackIndex(0)));
}

/// Appends an append entry operation on a shared container
fn append_append_entry<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    append_entry_update_data: &AppendEntryUpdateData,
) {
    context.write(RegularInstruction::append_entry());
    context.visit_value_container(&append_entry_update_data.value);
    // target
    context.write(RegularInstruction::borrow_stack_value(StackIndex(0)));
}

/// Appends a list splice operation on a shared container
fn append_list_splice<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    context: &mut T,
    list_splice_update_data: &ListSpliceUpdateData,
) {
    context.write(RegularInstruction::splice(
        list_splice_update_data.start,
        list_splice_update_data.delete_count,
        list_splice_update_data.items.len() as u32,
    ));

    for item in &list_splice_update_data.items {
        context.visit_value_container(item);
    }

    // target
    context.write(RegularInstruction::borrow_stack_value(StackIndex(0)));
}

/// Appends a clear operation on a shared container
fn append_clear<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(context: &mut T) {
    context.write(RegularInstruction::clear());

    // target
    context.write(RegularInstruction::borrow_stack_value(StackIndex(0)));
}

/// Appends a delete entry operation on a shared container
fn append_delete_entry<'ctx, T: BufferProvider + ValueVisitor<'ctx>>(
    _context: &mut T,
    _delete_entry_update_data: &DeleteEntryUpdateData,
) {
    todo!()
}

/// Appends a set property operation on a shared container, based on the provided value key.
pub fn append_set_property_value_key<
    'ctx,
    T: BufferProvider + ValueVisitor<'ctx>,
>(
    context: &mut T,
    value_key: &ValueContainer,
) {
    if let Some(string) = value_key.try_as::<String>() {
        context.write(RegularInstruction::set_entry_text(string.clone()));
    } else if let Some(index) = value_key.try_as::<u32>() { // TODO: integer conversion
        context.write(RegularInstruction::set_entry_index(*index));
    } else {
        context.write(RegularInstruction::set_entry_dynamic());
        context.visit_value_container(value_key);
    }
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
