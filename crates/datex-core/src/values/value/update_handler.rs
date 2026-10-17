use core::cell::RefCell;

use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::base_shared_value_container::observers::TransceiverId,
    traits::local_child_path_resolver::LocalChildPathResolver,
    value_updates::{
        update_data::UpdateOperation,
        update_handler::{
            InternalMutabilityUpdateHandler, UpdateCallbackData,
            UpdateCallbackDataAccess, UpdateHandlerImpl, UpdateResult,
        },
    },
    values::{value::Value, value_container::ValueContainer},
};

impl InternalMutabilityUpdateHandler for Value {
    fn set_update_callback_data(
        &mut self,
        observe_data: Option<UpdateCallbackData>,
    ) {
        match self {
            Value::Core(core) => core.set_update_callback_data(observe_data),
            Value::Native(_native) => {
                todo!()
            }
        }
    }
}

impl UpdateCallbackDataAccess for Value {
    fn get_update_callback_data(&self) -> Option<&UpdateCallbackData> {
        match &self {
            Value::Core(core) => core.get_update_callback_data(),
            Value::Native(native) => native.get_update_callback_data(),
        }
    }
}

impl Value {
    pub(crate) fn try_update_collapsed_local_inner(
        &mut self,
        operation: UpdateOperation,
        path: Vec<ValueContainer>,
        source_id: Option<TransceiverId>,
        cache: &RefCell<SharedReferencesCache>,
    ) -> UpdateResult {
        // first collapse path to most inner nested value
        let inner_local =
            if let Some(([first], rest)) = path.split_at_checked(1) {
                self.resolve_value_for_path(first, rest)?
            } else {
                self
            };

        // then apply update
        inner_local.try_update(operation, source_id, cache)
    }
}

impl UpdateHandlerImpl for Value {
    /// Tries to update the value with the given operation.
    /// If a path first needs to be resolved, use [Value::try_update_collapsed_local_inner]
    fn try_update(
        &mut self,
        operation: UpdateOperation,
        source_id: Option<TransceiverId>,
        cache: &RefCell<SharedReferencesCache>,
    ) -> UpdateResult {
        match self {
            Value::Core(core) => core.try_update(operation, source_id, cache),
            Value::Native(native) => {
                native.try_update(operation, source_id, cache)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::*,
        runtime::cache::shared_references_cache::SharedReferencesCache,
        shared_values::{
            base_shared_value_container::observers::TransceiverId,
            errors::{AccessError, IndexOutOfBoundsError},
        },
        value_updates::{
            errors::UpdateError,
            update_data::UpdateOperation,
            update_handler::{
                InternalMutabilityUpdateHandler, UpdateCallbackData,
                UpdateCallbackDataAccess, UpdateHandlerImpl,
            },
        },
        values::{
            core_values::{list::List, map::Map},
            value::Value,
            value_container::{ValueContainer},
        },
    };
    use core::{assert_matches, cell::RefCell, ops::Deref};

    #[test]
    fn push() {
        let cache = RefCell::new(SharedReferencesCache::default());
        // Push to list value
        let mut list = Value::from(List::from(vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ]));
        list.try_update_collapsed_local_inner(
            UpdateOperation::append_entry(ValueContainer::from(4)),
            vec![],
            None,
            &cache,
        )
        .expect("Failed to push value to list");
        let updated_value = list.try_get_property(&ValueContainer::from(3u32)).unwrap();
        assert_eq!(updated_value.try_as::<i32>().unwrap().deref(), &4);

        // Try to push to non-list value
        let mut int = Value::from(42);
        let result = int.try_update_collapsed_local_inner(
            UpdateOperation::append_entry(ValueContainer::from(4)),
            vec![],
            None,
            &cache,
        );
        assert_matches!(result, Err(UpdateError::InvalidUpdate))
    }

    #[test]
    fn get_set_property() {
        let cache = RefCell::new(SharedReferencesCache::default());
        let mut map = Value::from(Map::from(vec![
            ("key1".to_string(), ValueContainer::from(1)),
            ("key2".to_string(), ValueContainer::from(2)),
        ]));

        map.try_update_collapsed_local_inner(
            UpdateOperation::set_entry("key1".into(), ValueContainer::from(42)),
            vec![],
            None,
            &cache,
        )
        .expect("Failed to set existing property");
        let updated_value = map.try_get_property(&ValueContainer::from("key1")).unwrap();
        assert_eq!(updated_value.try_as::<i32>().unwrap().deref(), &42);

        // Set new property
        let result = map.try_update_collapsed_local_inner(
            UpdateOperation::set_entry("new".into(), ValueContainer::from(99)),
            vec![],
            None,
            &cache,
        );
        assert!(result.is_ok());
        let new_value = map.try_get_property(&ValueContainer::from("new")).unwrap();
        assert_eq!(new_value.try_as::<i32>().unwrap().deref(), &99);
    }

    #[test]
    fn numeric_property() {
        let mut list = Value::native(vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ]);

        let cache = RefCell::new(SharedReferencesCache::default());
        // Set existing index
        list.try_update_collapsed_local_inner(
            UpdateOperation::set_entry(1u32.into(), ValueContainer::from(42)),
            vec![],
            None,
            &cache,
        )
        .expect("Failed to set existing index");
        let updated_value = list.try_get_property(&ValueContainer::from(1u32)).unwrap();
        assert_eq!(updated_value.try_as::<i32>().unwrap().deref(), &42);

        // Try to set out-of-bounds index
        let result = list.try_update_collapsed_local_inner(
            UpdateOperation::set_entry(5u32.into(), ValueContainer::from(99)),
            vec![],
            None,
            &cache,
        );
        assert_eq!(
            result,
            Err(UpdateError::access_error(AccessError::IndexOutOfBounds(
                IndexOutOfBoundsError { index: 5 }
            )))
        );

        // Try to set index on non-map value
        let mut int_ref = Value::from(1);
        let result = int_ref.try_update_collapsed_local_inner(
            UpdateOperation::set_entry(0u32.into(), ValueContainer::from(42)),
            vec![],
            None,
            &cache,
        );
        assert_matches!(result, Err(UpdateError::InvalidUpdate));
    }

    #[test]
    fn text_property() {
        let cache = RefCell::new(SharedReferencesCache::default());
        let mut struct_val = Value::from(Map::from(vec![
            (
                ValueContainer::from("name"),
                ValueContainer::from("Alice"),
            ),
            (
                ValueContainer::from("age"),
                ValueContainer::from(30),
            ),
        ]));

        // Set existing property
        struct_val
            .try_update_collapsed_local_inner(
                UpdateOperation::set_entry(
                    "name".into(),
                    ValueContainer::from("Bob".to_string()),
                ),
                vec![],
                None,
                &cache,
            )
            .expect("Failed to set existing property");
        let name = struct_val.try_get_property(&ValueContainer::from("name")).unwrap();
        assert_eq!(name.try_as::<str>().unwrap().deref(), "Bob");

        // Try to set non-existing property
        let result = struct_val.try_update_collapsed_local_inner(
            UpdateOperation::set_entry(
                "non_existing".into(),
                ValueContainer::from("Charlie".to_string()),
            ),
            vec![],
            None,
            &cache,
        );
        assert_matches!(result, Ok(_));

        // // Try to set property on non-struct value
        let mut int = Value::from(1);
        let result = int.try_update_collapsed_local_inner(
            UpdateOperation::set_entry(
                "name".into(),
                ValueContainer::from("Bob".to_string()),
            ),
            vec![],
            None,
            &cache,
        );
        assert_matches!(result, Err(UpdateError::InvalidUpdate));
    }

    #[test]
    fn nested_map_property() {
        let cache = RefCell::new(SharedReferencesCache::default());
        let mut nested_map = Value::from(Map::from(vec![(
            ValueContainer::from("outer".to_string()),
            ValueContainer::from(Map::from(vec![(
                ValueContainer::from("inner".to_string()),
                ValueContainer::from(1),
            )])),
        )]));

        // Set existing nested property
        nested_map
            .try_update_collapsed_local_inner(
                UpdateOperation::set_entry(
                    "inner".into(),
                    ValueContainer::from(42),
                ),
                vec![ValueContainer::local("outer".to_string())],
                None,
                &cache,
            )
            .expect("Failed to set existing nested property");
        let prop = nested_map.try_get_property(&ValueContainer::from("outer")).unwrap();
        let inner_value = prop.try_as::<Map>().unwrap();
        let inner_value = inner_value.try_get(&ValueContainer::from("inner")).unwrap();
        assert_eq!(inner_value.try_as::<i32>().unwrap().deref(), &42);
    }

    #[test]
    fn observer_callbacks() {
        let cache = RefCell::new(SharedReferencesCache::default());
        let mut list = Value::from(List::from(vec![
            ValueContainer::from(List::from(vec![1])),
            ValueContainer::from(2),
        ]));

        let callback_updates = Rc::new(RefCell::new(vec![]));
        let callback_updates_clone = callback_updates.clone();

        list.set_update_callback_data(Some(UpdateCallbackData {
            callback: Rc::new(move |update| {
                callback_updates_clone.borrow_mut().push(update.clone());
            }),
            path: vec![],
        }));

        // Push to list value (should not trigger callback since source id is None)
        list.try_update_collapsed_local_inner(
            UpdateOperation::append_entry(ValueContainer::from(4)),
            vec![],
            None,
            &cache,
        )
        .expect("Failed to push value to list");

        // Push to list value (should trigger callback)
        list.try_update_collapsed_local_inner(
            UpdateOperation::append_entry(ValueContainer::from(5)),
            vec![],
            Some(TransceiverId::Local),
            &cache,
        )
        .expect("Failed to push value to list");

        // Check that the callback was triggered
        {
            let updates = callback_updates.borrow();
            assert_eq!(updates.len(), 1);
            assert_eq!(
                updates[0].operation(),
                &UpdateOperation::append_entry(ValueContainer::from(5))
            );
            assert_eq!(updates[0].path(), &vec![]);
        }

        {
            // get inner list value and check that it has the correct update callback data
            // (should be auto derived for children)
            let inner = list.try_get_property_mut(&ValueContainer::from(0)).unwrap();
            let mut inner_list = inner.try_as_mut::<List>().unwrap();
            assert_matches!(inner_list.get_update_callback_data(), Some(UpdateCallbackData {
                path,
                ..
            }) if path == &vec![ValueContainer::local(0)]);

            // reset callback tracking
            callback_updates.borrow_mut().clear();

            // Push to inner list value (should trigger callback)
            inner_list
                .try_update(
                    UpdateOperation::append_entry(ValueContainer::from(6)),
                    Some(TransceiverId::Local),
                    &cache,
                )
                .expect("Failed to push value to inner list");
        }

        {
            let updates = callback_updates.borrow();
            assert_eq!(updates.len(), 1);
            assert_eq!(
                updates[0].operation(),
                &UpdateOperation::append_entry(ValueContainer::from(6))
            );
            assert_eq!(updates[0].path(), &vec![ValueContainer::local(0)]);
        }

        // reset callback tracking
        callback_updates.borrow_mut().clear();

        // Update inner list via outer list and check that callback is triggered with correct path
        list.try_update_collapsed_local_inner(
            UpdateOperation::set_entry(
                0.into(),
                ValueContainer::from(List::from(vec![7])),
            ),
            vec![ValueContainer::local(0)],
            Some(TransceiverId::Local),
            &cache,
        )
        .expect("Failed to set inner list value");

        {
            let updates = callback_updates.borrow();
            assert_eq!(updates.len(), 1);
            assert_eq!(
                updates[0].operation(),
                &UpdateOperation::set_entry(
                    0.into(),
                    ValueContainer::from(List::from(vec![7]))
                )
            );
            assert_eq!(updates[0].path(), &vec![ValueContainer::local(0)]);
        }

        // reset callback tracking
        callback_updates.borrow_mut().clear();

        // update inner list with normal push method directly
        {
            // get inner list value
            let inner = list.try_get_property_mut(&ValueContainer::from(0)).unwrap();
            let mut inner_list = inner.try_as_mut::<List>().unwrap();
            inner_list.push(42);
        }

        {
            let updates = callback_updates.borrow();
            assert_eq!(updates.len(), 1);
            assert_eq!(
                updates[0].operation(),
                &UpdateOperation::append_entry(ValueContainer::from(42))
            );
            assert_eq!(updates[0].path(), &vec![ValueContainer::local(0)]);
        }

        // reset callback tracking
        callback_updates.borrow_mut().clear();

        // add new inner list
        {
            // add new inner list value
            let new_inner = ValueContainer::from(List::from(vec![7]));
            list.try_update_collapsed_local_inner(
                UpdateOperation::set_entry(1.into(), new_inner),
                vec![],
                Some(TransceiverId::Local),
                &cache,
            )
            .unwrap();

            // check that the callback data was set on the new inner list
            let inner = list.try_get_property_mut(&ValueContainer::from(1)).unwrap();
            let mut inner_list = inner.try_as_mut::<List>().unwrap();
            assert_matches!(inner_list.get_update_callback_data(), Some(UpdateCallbackData {
                path,
                ..
            }) if path == &vec![ValueContainer::local(1)]);

            // check that update callback was triggered
            {
                let updates = callback_updates.borrow();
                assert_eq!(updates.len(), 1);
                assert_eq!(
                    updates[0].operation(),
                    &UpdateOperation::set_entry(
                        1.into(),
                        ValueContainer::from(List::from(vec![7]))
                    )
                );
                assert_eq!(updates[0].path(), &vec![]);
            }

            // reset callback tracking
            callback_updates.borrow_mut().clear();

            // clear inner list should trigger callback
            inner_list.clear();
        }

        {
            let updates = callback_updates.borrow();
            assert_eq!(updates.len(), 1);
            assert_eq!(updates[0].operation(), &UpdateOperation::clear());
            assert_eq!(updates[0].path(), &vec![ValueContainer::local(1)]);
        }
    }
}
