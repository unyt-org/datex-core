use core::{cell::RefCell, ops::Deref};

use crate::{
    collections::HashMap,
    core_compiler::{
        core_compilation_context::ByteCursor,
        shared_value_tracking::{TrackedValueCollection, TrackedValueMetadata},
        to_instructions::{ToInstructions, yield_all, yield_all_collected},
        value_visitor::ValueVisitor,
    },
    global::stack_index::StackIndex,
    instruction::{
        BytecodeItem, Instruction,
        instruction_data::{MoveWithValue, SharedRef, SharedRefWithValue},
        regular_instruction::RegularInstruction,
    },
    prelude::*,
    shared_values::{
        PointerAddress, ReferenceMutability, ReferencedSharedContainer,
        SelfOwnedPointerAddress, SharedContainer, SharedContainerMutability,
        SharedContainerOwnership, traits::SharedContainerCommon,
    },
    values::value_container::ValueContainer,
};
use binrw::{BinWrite, io::Write};

#[derive(Debug)]
struct PreambleContext {
    self_referencing_containers: HashMap<SharedContainer, StackIndex>,
    known_container: HashSet<SharedContainer>,
    current_stack_index: StackIndex,
}

// var x = shared {a: {b: null}};
// x.a.b = x; /// #0.a.b = x;

impl ValueVisitor for PreambleContext {
    fn visit_shared_container<'a>(
        &'a mut self,
        shared_container: &'a SharedContainer,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        match self.self_referencing_containers.get(shared_container) {
            // not self referencing
            None => {
                // placeholder, will be later replaced via setter
                // TODO Make to yield
                Box::new(append_shared_container(
                    self,
                    shared_container,
                    self.known_container.contains(shared_container),
                ))
            }
            // shared container has already been inserted, use stack value
            // depending on the move flag, we either take the value or borrow it
            Some(stack_index) => {
                if shared_container.treat_as_move() {
                    todo!("Create test case here!");
                    // RegularInstruction::take_stack_value(*stack_index)
                    //     .write(self.cursor)
                    //     .unwrap();
                } else {
                    match shared_container.ownership() {
                        // FIXME can we be sure, that this can not happen?
                        SharedContainerOwnership::Owned => {
                            unreachable!("Owned shared container")
                        }
                        SharedContainerOwnership::Referenced(
                            reference_mut,
                        ) => match reference_mut {
                            ReferenceMutability::Immutable => {
                                Box::new(core::iter::once(
                                    RegularInstruction::get_stack_value_shared_ref(*stack_index).into()
                                ))
                            }
                            ReferenceMutability::Mutable => {
                                Box::new(core::iter::once(
                                    RegularInstruction::get_stack_value_shared_ref_mut(*stack_index).into()
                                ))
                            }
                        },
                    }
                }
            }
        }
    }
}

impl PreambleContext {
    fn get_next_stack_index(&mut self) -> StackIndex {
        let stack_index = self.current_stack_index;
        self.current_stack_index = StackIndex(self.current_stack_index.0 + 1);
        stack_index
    }
}

pub(super) fn append_injected_values_preamble(
    collection: TrackedValueCollection, // top level shared container roots
    body: Vec<u8>,
) -> (Vec<u8>, Vec<SharedContainer>) {
    // no injected values
    if !collection.has_tracked_values() {
        return (body, vec![]);
    }

    let tracked_values = collection.tracked_values;
    let mut cursor = ByteCursor::new(vec![]);

    let mut root_containers =
        Vec::<SharedContainer>::with_capacity(collection.root_count);
    let mut root_container_stack_indices: Vec<Option<StackIndex>> =
        vec![None; collection.root_count];
    // This keeps track of the number of inner statements of the preamble.
    // The count is the number of all stack values, plus the number of patches per stack value applied, plus one for the list return
    let inner_statement_count = &mut 0u32;

    {
        let context = &mut PreambleContext {
            self_referencing_containers: HashMap::new(),
            current_stack_index: StackIndex(0),
            known_container: HashSet::new(),
        };

        // loop through the tracked values in reverse order (as optimization)
        for (tracked_value, metadata) in tracked_values.iter() {
            if metadata.is_self_referencing() {
                let new_index = context.get_next_stack_index();
                for instruction in append_uninitialized_shared_container(
                    context,
                    tracked_value,
                    metadata,
                    inner_statement_count,
                )
                .collect::<Vec<_>>()
                {
                    match instruction {
                        BytecodeItem::Instruction(instruction) => {
                            instruction.write(&mut cursor);
                        }
                        _ => unreachable!(),
                    }
                }
                context
                    .self_referencing_containers
                    .insert(tracked_value.clone(), new_index);
            }
            if metadata.is_known() {
                context.known_container.insert(tracked_value.clone());
            }
        }
        for (tracked_value, metadata) in tracked_values.into_iter() {
            if let TrackedValueMetadata::Root {
                index,
                is_self_referencing,
                is_known,
            } = metadata
            {
                let new_index = if is_self_referencing {
                    let uninitialized_index = *context.self_referencing_containers.get(&tracked_value).expect("Self referencing container should have been inserted");
                    for instruction in
                        append_self_referencing_shared_container_to_stack(
                            context,
                            &tracked_value,
                            inner_statement_count,
                            uninitialized_index,
                        )
                        .collect::<Vec<crate::instruction::BytecodeItem>>()
                    {
                        match instruction {
                            BytecodeItem::Instruction(instruction) => {
                                instruction.write(&mut cursor).unwrap();
                            }
                            _ => unreachable!(),
                        }
                    }
                    uninitialized_index
                } else {
                    let index = context.get_next_stack_index();
                    for instruction in append_shared_container_to_stack(
                        context,
                        &tracked_value,
                        is_known,
                        inner_statement_count,
                    )
                    .collect::<Vec<crate::instruction::BytecodeItem>>()
                    {
                        match instruction {
                            BytecodeItem::Instruction(instruction) => {
                                instruction.write(&mut cursor).unwrap();
                            }
                            _ => unreachable!(),
                        }
                    }
                    index
                };
                root_containers.push(tracked_value);
                root_container_stack_indices[index.0 as usize] =
                    Some(new_index);
            }
        }

        // asserting that root stack index keys are a contiguous list of 0-n -> inner stack indices
        let root_container_stack_indices_sorted = root_container_stack_indices
            .iter()
            .map(|index| {
                index.expect("Root stack index should have been registered")
            })
            .collect::<Vec<_>>();

        // Final root list
        RegularInstruction::list(
            root_container_stack_indices_sorted.len() as u32
        )
        .write(&mut cursor)
        .unwrap();

        // insert all root stack values into the list, which is the final value of the preamble
        // append [#0,...#x]
        for stack_index in root_container_stack_indices_sorted {
            RegularInstruction::take_stack_value(stack_index)
                .write(&mut cursor)
                .unwrap();
        }

        // The final root list is also one inner statement.
        *inner_statement_count += 1;
    }
    let mut byte_cursor = ByteCursor::new(vec![]);

    // statements (preamble + body) start
    RegularInstruction::statements(2, false)
        .write(&mut byte_cursor)
        .unwrap();

    // the preamble pushes a list of the root shared containers to the stack
    RegularInstruction::push_list_to_stack()
        .write(&mut byte_cursor)
        .unwrap();

    // the inner preamble statements, which push the shared containers to the stack
    // and apply the patches
    RegularInstruction::statements(*inner_statement_count, false)
        .write(&mut byte_cursor)
        .unwrap();
    // the inner preamble content, which pushes the shared containers to the stack and applies the patches
    byte_cursor.write_all(&cursor.into_inner()).unwrap();

    // append body
    byte_cursor.write_all(&body).unwrap();
    (byte_cursor.into_inner(), root_containers)
}

gen fn append_uninitialized_shared_container(
    context: &mut PreambleContext,
    container: &SharedContainer,
    metadata: &TrackedValueMetadata,
    statements_count: &mut u32,
) -> crate::instruction::BytecodeItem {
    *statements_count += 1;

    // append push to stack
    yield RegularInstruction::push_to_stack().into();
    match container {
        SharedContainer::Owned(owned_container) => {
            for instruction in append_move_with_value(
                context,
                owned_container.container_mutability(),
                owned_container.pointer_address().clone(),
                None,
            ) {
                yield instruction;
            }
        }
        SharedContainer::Referenced(referenced_container) => {
            if referenced_container.treat_as_move() {
                for instruction in append_move_with_value(
                    context,
                    referenced_container.container_mutability(),
                    match referenced_container.pointer_address() {
                        PointerAddress::SelfOwned(address) => address.clone(),
                        _ => unreachable!(
                            "Referenced shared container should not have a remote pointer address"
                        ),
                    },
                    None,
                ) {
                    yield instruction;
                }
            } else if !metadata.is_known() {
                for instruction in append_referenced_shared_container_with_value(
                    context,
                    referenced_container,
                    true,
                ) {
                    yield instruction;
                }
            } else {
                for instruction in
                    append_referenced_shared_container(referenced_container)
                {
                    yield instruction;
                }
            }
        }
    };

    return;
}

gen fn append_self_referencing_shared_container_to_stack(
    context: &mut PreambleContext,
    container: &SharedContainer,
    statements_count: &mut u32,
    uninitialized_index: StackIndex,
) -> crate::instruction::BytecodeItem {
    *statements_count += 1;
    yield RegularInstruction::set_shared_container_value().into();
    yield_all_collected!(
        container
            .value_container()
            .deref()
            .to_bytecode_item(context)
    );
    yield RegularInstruction::borrow_stack_value(uninitialized_index).into();
    return;
}

gen fn append_shared_container_to_stack(
    context: &mut PreambleContext,
    container: &SharedContainer,
    is_known: bool,
    statements_count: &mut u32,
) -> crate::instruction::BytecodeItem {
    *statements_count += 1;

    // append push to stack
    yield RegularInstruction::push_to_stack().into();
    for instruction in append_shared_container(context, container, is_known) {
        yield instruction;
    }
}

gen fn append_shared_container(
    context: &mut PreambleContext,
    container: &SharedContainer,
    is_known: bool,
) -> crate::instruction::BytecodeItem {
    match container {
        SharedContainer::Owned(owned_container) => {
            let value_container = owned_container.value_container();
            for instruction in append_move_with_value(
                context,
                owned_container.container_mutability(),
                owned_container.pointer_address().clone(),
                Some(value_container.deref()),
            )
            .collect::<Vec<_>>()
            {
                yield instruction;
            }
        }
        SharedContainer::Referenced(referenced_container) => {
            if referenced_container.treat_as_move() {
                let value_container = referenced_container.value_container();
                for instruction in append_move_with_value(
                    context,
                    referenced_container.container_mutability(),
                    match referenced_container.pointer_address() {
                        PointerAddress::SelfOwned(address) => address.clone(),
                        _ => unreachable!(
                            "Referenced shared container should not have a remote pointer address"
                        ),
                    },
                    Some(value_container.deref()),
                ).collect::<Vec<_>>() {
                    yield instruction;
                }
            } else if !is_known {
                for instruction in append_referenced_shared_container_with_value(
                    context,
                    referenced_container,
                    false,
                ) {
                    yield instruction;
                }
            } else {
                for instruction in
                    append_referenced_shared_container(referenced_container)
                {
                    yield instruction;
                }
            }
        }
    }
}

/// Appends a move instruction to the preamble to move an owned shared containers
gen fn append_move_with_value(
    context: &mut PreambleContext,
    container_mutability: SharedContainerMutability,
    previous_address: SelfOwnedPointerAddress,
    value_container: Option<&ValueContainer>,
) -> crate::instruction::BytecodeItem {
    yield RegularInstruction::move_with_value(MoveWithValue {
        mutability: container_mutability,
        previous_address,
    })
    .into();

    if let Some(value_container) = value_container {
        yield_all!(value_container.to_bytecode_item(context));
    } else {
        yield RegularInstruction::Uninitialized.into();
    }

    return;
}

gen fn append_referenced_shared_container(
    referenced_container: &ReferencedSharedContainer,
) -> crate::instruction::BytecodeItem {
    match referenced_container.pointer_address() {
        PointerAddress::SelfOwned(pointer_address) => {
            yield RegularInstruction::shared_ref(SharedRef {
                address: PointerAddress::SelfOwned(pointer_address),
                ref_mutability: referenced_container.reference_mutability(),
                container_mutability: referenced_container
                    .container_mutability(),
            })
            .into();
        }
        PointerAddress::Remote(pointer_address) => {
            yield RegularInstruction::shared_ref(SharedRef {
                address: PointerAddress::Remote(pointer_address),
                ref_mutability: referenced_container.reference_mutability(),
                container_mutability: referenced_container
                    .container_mutability(),
            })
            .into();
        }
    }

    return;
}

gen fn append_referenced_shared_container_with_value(
    context: &mut PreambleContext,
    referenced_container: &ReferencedSharedContainer,
    unitialized: bool,
) -> crate::instruction::BytecodeItem {
    match referenced_container.pointer_address() {
        // insert with value for self owned references
        PointerAddress::SelfOwned(pointer_address) => {
            yield RegularInstruction::shared_ref_with_value(
                SharedRefWithValue {
                    address: pointer_address,
                    ref_mutability: referenced_container.reference_mutability(),
                    container_mutability: referenced_container
                        .container_mutability(),
                },
            )
            .into();
            if unitialized {
                yield RegularInstruction::Uninitialized.into();
            } else {
                // TODO: no collect?
                yield_all_collected!(
                    referenced_container
                        .value_container()
                        .deref()
                        .to_bytecode_item(context)
                );
            }
        }
        // insert without value for non self owned references
        PointerAddress::Remote(pointer_address) => {
            yield RegularInstruction::shared_ref(SharedRef {
                address: PointerAddress::Remote(pointer_address),
                ref_mutability: referenced_container.reference_mutability(),
                container_mutability: referenced_container
                    .container_mutability(),
            })
            .into();
        }
    }

    return;
}

#[cfg(test)]
#[cfg(feature = "disassembler")]
mod tests {
    use binrw::BinWrite;

    use crate::{
        core_compiler::{
            core_compilation_context::ByteCursor,
            preamble::append_injected_values_preamble,
            shared_value_tracking::{
                TrackedValueCollection, TrackedValueMetadata,
            },
        },
        disassembler::{
            InstructionTree,
            assertions::{assert_instructions_equal, instructions},
        },
        global::stack_index::StackIndex,
        instruction,
        instruction::{
            Instruction,
            instruction_data::{
                Int32Data, ListData, MoveWithValue, SharedRefWithValue,
                ShortListData, ShortMapData, ShortTextData, UInt32Data,
            },
            regular_instruction::RegularInstruction,
        },
        prelude::*,
        runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
        shared_values::{
            OwnedSharedContainer, ReferenceMutability,
            ReferencedSharedContainer, SelfOwnedPointerAddress,
            SharedContainer, SharedContainerMutability,
            SharedContainerOwnership, traits::SharedContainerCommon,
        },
        values::{
            core_values::{list::List, map::Map},
            value_container::ValueContainer,
        },
    };

    fn assert_preamble_instructions(
        tracked_values: Vec<(SharedContainer, TrackedValueMetadata)>,
        instructions: Vec<impl Into<InstructionTree<Instruction>>>,
    ) {
        // mock body
        let mut cursor = ByteCursor::new(vec![]);
        RegularInstruction::null().write(&mut cursor).unwrap();

        let root_count = tracked_values
            .iter()
            .filter(|(_, metadata)| {
                matches!(metadata, TrackedValueMetadata::Root { .. })
            })
            .count();

        let collection = TrackedValueCollection {
            root_count,
            tracked_values,
        };
        let (bytes, _) =
            append_injected_values_preamble(collection, cursor.into_inner());

        assert_instructions_equal!(&bytes, instructions);
    }

    fn generate_shared_owned_value(
        address_provider: &mut SelfOwnedPointerAddressProvider,
        value: impl Into<ValueContainer>,
        mutability: SharedContainerMutability,
    ) -> (OwnedSharedContainer, SelfOwnedPointerAddress) {
        match generate_shared_value(
            address_provider,
            value,
            SharedContainerOwnership::Owned,
            mutability,
        ) {
            (SharedContainer::Owned(owned_container), address) => {
                (owned_container, address)
            }
            _ => unreachable!(),
        }
    }

    fn generate_shared_referenced_value(
        address_provider: &mut SelfOwnedPointerAddressProvider,
        value: impl Into<ValueContainer>,
        reference_mutability: ReferenceMutability,
        mutability: SharedContainerMutability,
    ) -> (ReferencedSharedContainer, SelfOwnedPointerAddress) {
        match generate_shared_value(
            address_provider,
            value,
            SharedContainerOwnership::Referenced(reference_mutability),
            mutability,
        ) {
            (SharedContainer::Referenced(referenced_container), address) => {
                (referenced_container, address)
            }
            _ => unreachable!(),
        }
    }

    fn generate_shared_value(
        address_provider: &mut SelfOwnedPointerAddressProvider,
        value: impl Into<ValueContainer>,
        ownership: SharedContainerOwnership,
        mutability: SharedContainerMutability,
    ) -> (SharedContainer, SelfOwnedPointerAddress) {
        let value_container = value.into();
        let mut shared_container =
            SharedContainer::new_owned_with_inferred_allowed_type(
                value_container,
                mutability,
                address_provider,
            );
        let address = match &shared_container {
            SharedContainer::Owned(owned_container) => {
                owned_container.pointer_address().clone()
            }
            _ => unreachable!(),
        };

        if let SharedContainerOwnership::Referenced(mutability) = ownership {
            shared_container = SharedContainer::Referenced(
                shared_container
                    .try_derive_reference_with_mutability(mutability)
                    .unwrap(),
            );
        };

        (shared_container, address)
    }

    #[test]
    fn preamble_no_injected_values() {
        assert_preamble_instructions(vec![], vec![RegularInstruction::Null]);
    }

    #[test]
    fn preamble_single_non_referencing_ref() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();
        let (reference, address) = generate_shared_referenced_value(
            address_provider,
            42,
            ReferenceMutability::Immutable,
            SharedContainerMutability::Immutable,
        );

        assert_preamble_instructions(
            vec![(
                SharedContainer::Referenced(reference),
                TrackedValueMetadata::Root {
                    index: StackIndex(0),
                    is_known: false,
                    is_self_referencing: false,
                },
            )],
            vec![RegularInstruction::statements_with_children(
                false,
                instructions!(
                    // preamble
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(
                                SharedRefWithValue {
                                    address,
                                    ref_mutability:
                                        ReferenceMutability::Immutable,
                                    container_mutability:
                                        SharedContainerMutability::Immutable,
                                }
                            ),
                            RegularInstruction::Int32(Int32Data(42)),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    )
                                )
                            ),
                        )
                    ),
                    // body
                    RegularInstruction::Null,
                ),
            )],
        );
    }

    #[test]
    fn preamble_single_recursive_ref() {
        // var a = shared mut [];
        // a.0 = 'a;

        let address_provider = &mut SelfOwnedPointerAddressProvider::default();
        let (owned, address) = generate_shared_owned_value(
            address_provider,
            ValueContainer::Local(List::default().into()),
            SharedContainerMutability::Mutable,
        );
        let owned_container = SharedContainer::Owned(owned);

        // self referencing list
        {
            let container_ref = SharedContainer::Referenced(
                owned_container.derive_reference_with_max_mutability(),
            );
            let mut container_mut = owned_container.value_container_mut();
            let list = container_mut.try_as_mut::<List>().unwrap();
            list.push(ValueContainer::Shared(container_ref));
        }

        assert_preamble_instructions(
            vec![(
                owned_container,
                TrackedValueMetadata::Root {
                    index: StackIndex(0),
                    is_known: false,
                    is_self_referencing: true,
                },
            )],
            vec![RegularInstruction::statements_with_children(
                false,
                instructions!(
                    // preamble
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // shared mut a = [[Uninitialized]];
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability: SharedContainerMutability::Mutable,
                                previous_address: address.clone(),
                            })
                            .with_children(instructions!(RegularInstruction::Uninitialized)),

                            // *a = ['mut a];
                            RegularInstruction::SetSharedContainerValue.with_children(
                                instructions!(
                                    RegularInstruction::ShortList(
                                        ShortListData { element_count: 1 }
                                    )
                                    .with_children(instructions!(
                                        RegularInstruction::GetStackValueSharedRefMut(
                                            StackIndex(0)
                                        )
                                    )),
                                    RegularInstruction::BorrowStackValue(StackIndex(0))
                                )
                            ),

                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    )
                                )
                            ),
                        )
                    ),
                    // body
                    RegularInstruction::Null
                ),
            )],
        );
    }

    #[test]
    fn preamble_nested_recursive_ref() {
        // var x = shared {a: {b: null}};
        // x.a.b = 'x;

        let address_provider = &mut SelfOwnedPointerAddressProvider::default();
        let (owned, address) = generate_shared_owned_value(
            address_provider,
            ValueContainer::Local(Map::default().into()),
            SharedContainerMutability::Mutable,
        );
        let owned_container = SharedContainer::Owned(owned);

        // self referencing map
        {
            let container_ref = SharedContainer::Referenced(
                owned_container.derive_reference_with_max_mutability(),
            );
            let mut container_mut = owned_container.value_container_mut();
            let map = container_mut.try_as_mut::<Map>().unwrap();
            map.set_unchecked(
                "a",
                Map::from(vec![(
                    ValueContainer::from("b"),
                    ValueContainer::Shared(container_ref),
                )]),
            );
        }

        assert_preamble_instructions(
            vec![(
                owned_container,
                TrackedValueMetadata::Root {
                    index: StackIndex(0),
                    is_known: false,
                    is_self_referencing: true,
                },
            )],
            vec![RegularInstruction::statements_with_children(
                false,
                instructions!(
                    // preamble
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability: SharedContainerMutability::Mutable,
                                previous_address: address.clone(),
                            })
                            .with_children(
                                instructions!(
                                    RegularInstruction::Uninitialized
                                )
                            ),
                            RegularInstruction::SetSharedContainerValue.with_children(instructions!(
                                RegularInstruction::ShortMap(ShortMapData {
                                    element_count: 1
                                })
                                .with_children(instructions!(
                                    RegularInstruction::KeyValueShortText(
                                        ShortTextData("a".to_string())
                                    )
                                    .with_children(instructions!(
                                        RegularInstruction::ShortMap(
                                            ShortMapData { element_count: 1 }
                                        )
                                        .with_children(instructions!(
                                            RegularInstruction::KeyValueShortText(
                                                ShortTextData("b".to_string())
                                            )
                                            .with_children(instructions!(
                                                RegularInstruction::GetStackValueSharedRefMut(StackIndex(0))
                                            ))
                                        ))
                                    )))),
                                RegularInstruction::BorrowStackValue(StackIndex(0))
                            )),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(0)
                                    )
                                )
                            )
                        )
                    ),
                    // body
                    RegularInstruction::Null
                ),
            )],
        );
    }

    #[test]
    fn preamble_triple_ref() {
        // var a = shared {b: null};
        // var b = shared {c: null};
        // a.b = 'mut b;
        // var c = shared {a: 'mut a};
        // b.c = 'mut c;

        let address_provider = &mut SelfOwnedPointerAddressProvider::default();
        let (owned_container_a, address_a) = generate_shared_owned_value(
            address_provider,
            ValueContainer::Local(Map::default().into()),
            SharedContainerMutability::Mutable,
        );
        let (owned_container_b, address_b) = generate_shared_owned_value(
            address_provider,
            ValueContainer::Local(Map::default().into()),
            SharedContainerMutability::Mutable,
        );
        let (owned_container_c, address_c) = generate_shared_owned_value(
            address_provider,
            ValueContainer::Local(
                Map::from(vec![(
                    ValueContainer::from("a"),
                    ValueContainer::Shared(SharedContainer::Referenced(
                        owned_container_a.derive_with_max_mutability(),
                    )),
                )])
                .into(),
            ),
            SharedContainerMutability::Mutable,
        );
        // set a.b = b;
        {
            let mut container_mut = owned_container_a.value_container_mut();
            let map = container_mut.try_as_mut::<Map>().unwrap();
            map.set_unchecked(
                "b",
                ValueContainer::Shared(SharedContainer::Referenced(
                    owned_container_b.derive_with_max_mutability(),
                )),
            );
        }
        // set b.c = c;
        {
            let mut container_mut = owned_container_b.value_container_mut();
            let map = container_mut.try_as_mut::<Map>().unwrap();
            map.set_unchecked(
                "c",
                ValueContainer::Shared(SharedContainer::Referenced(
                    owned_container_c.derive_with_max_mutability(),
                )),
            );
        }

        assert_preamble_instructions(
            vec![
                (
                    SharedContainer::Referenced(
                        owned_container_c.derive_with_max_mutability(),
                    ),
                    TrackedValueMetadata::Child {
                        is_known: false,
                        is_self_referencing: true,
                    },
                ),
                (
                    SharedContainer::Referenced(
                        owned_container_b.derive_with_max_mutability(),
                    ),
                    TrackedValueMetadata::Child {
                        is_known: false,
                        is_self_referencing: true,
                    },
                ),
                (
                    SharedContainer::Owned(owned_container_a),
                    TrackedValueMetadata::Root {
                        index: StackIndex(0),
                        is_known: false,
                        is_self_referencing: true,
                    },
                ),
            ],
            vec![RegularInstruction::statements_with_children(
                false,
                instructions!(
                    // preamble
                    RegularInstruction::PushListToStack,
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            // push 1
                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(
                                SharedRefWithValue {
                                    ref_mutability:
                                        ReferenceMutability::Mutable,
                                    address: address_c.clone(),
                                    container_mutability:
                                        SharedContainerMutability::Mutable,
                                }
                            )
                            .with_children(
                                instructions!(
                                    RegularInstruction::Uninitialized
                                )
                            ),
                            // push 2
                            RegularInstruction::PushToStack,
                            RegularInstruction::SharedRefWithValue(
                                SharedRefWithValue {
                                    ref_mutability:
                                        ReferenceMutability::Mutable,
                                    address: address_b.clone(),
                                    container_mutability:
                                        SharedContainerMutability::Mutable,
                                }
                            )
                            .with_children(
                                instructions!(
                                    RegularInstruction::Uninitialized
                                )
                            ),
                            // push 3
                            RegularInstruction::PushToStack,
                            RegularInstruction::MoveWithValue(MoveWithValue {
                                mutability: SharedContainerMutability::Mutable,
                                previous_address: address_a.clone(),
                            })
                            .with_children(
                                instructions!(
                                    RegularInstruction::Uninitialized
                                )
                            ),
                            RegularInstruction::SetSharedContainerValue.with_children(
                                instructions!(
                                    RegularInstruction::ShortMap(
                                        ShortMapData { element_count: 1 }
                                    )
                                    .with_children(instructions!(
                                        RegularInstruction::KeyValueShortText(
                                            ShortTextData("b".to_string())
                                        )
                                        .with_children(instructions!(
                                            RegularInstruction::GetStackValueSharedRefMut(
                                                StackIndex(1)
                                            )
                                        ))
                                    )),
                                    RegularInstruction::BorrowStackValue(StackIndex(2))
                                )
                            ),
                            RegularInstruction::list_with_children(
                                instructions!(
                                    RegularInstruction::TakeStackValue(
                                        StackIndex(2)
                                    )
                                )
                            )
                        )
                    ),
                    // body
                    RegularInstruction::Null
                ),
            )],
        );
    }
}
