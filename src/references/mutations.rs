use crate::dif::update::{DIFKey, DIFUpdateData};
use crate::dif::value::DIFValueContainer;
use crate::references::observers::TransceiverId;
use crate::runtime::memory::Memory;
use crate::stdlib::format;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::values::value_container::ValueKey;
use crate::{
    references::reference::{AccessError, Reference},
    values::{core_value::CoreValue, value_container::ValueContainer},
};
use core::cell::RefCell;
use core::ops::FnOnce;
use core::prelude::rust_2024::*;

pub enum DIFUpdateDataOrMemory<'a> {
    Update(&'a DIFUpdateData),
    Memory(&'a RefCell<Memory>),
}

impl<'a> From<&'a DIFUpdateData> for DIFUpdateDataOrMemory<'a> {
    fn from(update: &'a DIFUpdateData) -> Self {
        DIFUpdateDataOrMemory::Update(update)
    }
}

impl<'a> From<&'a RefCell<Memory>> for DIFUpdateDataOrMemory<'a> {
    fn from(memory: &'a RefCell<Memory>) -> Self {
        DIFUpdateDataOrMemory::Memory(memory)
    }
}

impl Reference {
    /// Internal function that handles updates
    /// - Checks if the reference is mutable
    /// - Calls the provided handler to perform the update and get the DIFUpdateData
    /// - Notifies observers with the update data
    /// - Returns any AccessError encountered
    fn handle_update<'a>(
        &self,
        source_id: TransceiverId,
        handler: impl FnOnce() -> Result<&'a DIFUpdateData, AccessError>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }
        let update_data = handler()?;
        // self.notify_observers(update_data.with_source(source_id));
        Ok(())
    }

    fn assert_mutable(&self) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }
        Ok(())
    }

    /// Sets a property on the value if applicable (e.g. for maps)
    pub fn try_set_property<'a>(
        &self,
        source_id: TransceiverId,
        dif_update_data_or_memory: impl Into<DIFUpdateDataOrMemory<'a>>,
        key: impl Into<ValueKey<'a>>,
        val: ValueContainer,
    ) -> Result<(), AccessError> {
        self.assert_mutable()?;

        let key = key.into();
        let dif_update_data_or_memory = dif_update_data_or_memory.into();

        let dif_update = match dif_update_data_or_memory {
            DIFUpdateDataOrMemory::Update(update) => update,
            DIFUpdateDataOrMemory::Memory(memory) => &DIFUpdateData::set(
                DIFKey::from_value_key(&key, memory),
                DIFValueContainer::from_value_container(&val, memory),
            ),
        };

        self.with_value_unchecked(|value| {
            value.try_set_property(key, val.clone())
        })?;

        self.notify_observers(&dif_update.with_source(source_id));
        Ok(())
    }

    /// Sets a value on the reference if it is mutable and the type is compatible.
    pub fn try_replace<'a>(
        &self,
        source_id: TransceiverId,
        dif_update_data_or_memory: impl Into<DIFUpdateDataOrMemory<'a>>,
        value: impl Into<ValueContainer>,
    ) -> Result<(), AccessError> {
        self.assert_mutable()?;
        let dif_update_data_or_memory = dif_update_data_or_memory.into();

        // TODO #306: ensure type compatibility with allowed_type
        let value_container = &value.into();

        let dif_update = match dif_update_data_or_memory {
            DIFUpdateDataOrMemory::Update(update) => update,
            DIFUpdateDataOrMemory::Memory(memory) => &DIFUpdateData::replace(
                DIFValueContainer::from_value_container(
                    value_container,
                    memory,
                ),
            ),
        };

        self.with_value_unchecked(|core_value| {
            // Set the value directly, ensuring it is a ValueContainer
            core_value.inner =
                value_container.to_value().borrow().inner.clone();
        });

        self.notify_observers(&dif_update.with_source(source_id));
        Ok(())
    }

    /// Pushes a value to the reference if it is a list.
    pub fn try_append_value<'a>(
        &self,
        source_id: TransceiverId,
        dif_update_data_or_memory: impl Into<DIFUpdateDataOrMemory<'a>>,
        value: impl Into<ValueContainer>,
    ) -> Result<(), AccessError> {
        self.assert_mutable()?;
        let dif_update_data_or_memory = dif_update_data_or_memory.into();
        let value_container = value.into();

        let dif_update = match dif_update_data_or_memory {
            DIFUpdateDataOrMemory::Update(update) => update,
            DIFUpdateDataOrMemory::Memory(memory) => {
                &DIFUpdateData::append(DIFValueContainer::from_value_container(
                    &value_container,
                    memory,
                ))
            }
        };

        self.with_value_unchecked(move |core_value| {
            match &mut core_value.inner {
                CoreValue::List(list) => {
                    list.push(value_container);
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot push value to non-list value: {:?}",
                        core_value
                    )));
                }
            }

            Ok(())
        })?;

        self.notify_observers(&dif_update.with_source(source_id));
        Ok(())
    }

    /// Tries to delete a property from the reference if it is a map.
    /// Notifies observers if successful.
    pub fn try_delete_property<'a>(
        &self,
        source_id: TransceiverId,
        dif_update_data_or_memory: impl Into<DIFUpdateDataOrMemory<'a>>,
        key: impl Into<ValueKey<'a>>,
    ) -> Result<(), AccessError> {
        self.assert_mutable()?;
        let key = key.into();
        let dif_update_data_or_memory = dif_update_data_or_memory.into();

        let dif_update = match dif_update_data_or_memory {
            DIFUpdateDataOrMemory::Update(update) => update,
            DIFUpdateDataOrMemory::Memory(memory) => {
                &DIFUpdateData::delete(DIFKey::from_value_key(&key, memory))
            }
        };

        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    key.with_value_container(|key| map.delete(key))?;
                }
                CoreValue::List(ref mut list) => {
                    if let Some(index) = key.try_as_index() {
                        list.delete(index).map_err(|err| {
                            AccessError::IndexOutOfBounds(err)
                        })?;
                    } else {
                        return Err(AccessError::InvalidIndexKey);
                    }
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot delete property '{:?}' on non-map value: {:?}",
                        key, value
                    )));
                }
            }

            Ok(())
        })?;

        self.notify_observers(&dif_update.with_source(source_id));
        Ok(())
    }

    pub fn try_clear(
        &self,
        source_id: TransceiverId,
    ) -> Result<(), AccessError> {
        self.assert_mutable()?;

        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    map.clear()?;
                }
                CoreValue::List(ref mut list) => {
                    list.clear();
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot clear non-list/map value: {:?}",
                        value
                    )));
                }
            }

            Ok(())
        })?;

        self.notify_observers(&DIFUpdateData::clear().with_source(source_id));
        Ok(())
    }

    pub fn try_list_splice<'a>(
        &self,
        source_id: TransceiverId,
        dif_update_data_or_memory: impl Into<DIFUpdateDataOrMemory<'a>>,
        range: core::ops::Range<u32>,
        items: Vec<ValueContainer>,
    ) -> Result<(), AccessError> {
        self.assert_mutable()?;
        let dif_update_data_or_memory = dif_update_data_or_memory.into();

        let dif_update = match dif_update_data_or_memory {
            DIFUpdateDataOrMemory::Update(update) => update,
            DIFUpdateDataOrMemory::Memory(memory) => {
                &DIFUpdateData::list_splice(
                    range.clone(),
                    items
                        .iter()
                        .map(|item| {
                            DIFValueContainer::from_value_container(
                                item, memory,
                            )
                        })
                        .collect(),
                )
            }
        };

        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::List(ref mut list) => {
                    list.splice(range, items);
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot apply splice operation on non-list value: {:?}",
                        value
                    )));
                }
            }

            Ok(())
        })?;

        self.notify_observers(&dif_update.with_source(source_id));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::references::reference::{
        AccessError, AssignmentError, IndexOutOfBoundsError,
        ReferenceMutability,
    };
    use crate::runtime::memory::Memory;
    use crate::stdlib::assert_matches::assert_matches;
    use crate::values::core_values::list::List;
    use crate::values::core_values::map::Map;
    use crate::{
        references::reference::Reference,
        values::value_container::ValueContainer,
    };
    use core::cell::RefCell;

    #[test]
    fn push() {
        let memory = &RefCell::new(Memory::default());
        let list = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let list_ref =
            Reference::try_mut_from(List::from(list).into()).unwrap();
        list_ref
            .try_append_value(0, memory, ValueContainer::from(4))
            .expect("Failed to push value to list");
        let updated_value = list_ref.try_get_property(3).unwrap();
        assert_eq!(updated_value, ValueContainer::from(4));

        // Try to push to immutable value
        let int_ref =
            Reference::from(List::from(vec![ValueContainer::from(42)]));
        let result =
            int_ref.try_append_value(0, memory, ValueContainer::from(99));
        assert_matches!(result, Err(AccessError::ImmutableReference));

        // Try to push to non-list value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result =
            int_ref.try_append_value(0, memory, ValueContainer::from(99));
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn property() {
        let memory = &RefCell::new(Memory::default());

        let map = Map::from(vec![
            ("key1".to_string(), ValueContainer::from(1)),
            ("key2".to_string(), ValueContainer::from(2)),
        ]);
        let map_ref =
            Reference::try_mut_from(ValueContainer::from(map)).unwrap();
        // Set existing property
        map_ref
            .try_set_property(0, memory, "key1", ValueContainer::from(42))
            .expect("Failed to set existing property");
        let updated_value = map_ref.try_get_property("key1").unwrap();
        assert_eq!(updated_value, 42.into());

        // Set new property
        let result = map_ref.try_set_property(
            0,
            memory,
            "new",
            ValueContainer::from(99),
        );
        assert!(result.is_ok());
        let new_value = map_ref.try_get_property("new").unwrap();
        assert_eq!(new_value, 99.into());
    }

    #[test]
    fn numeric_property() {
        let memory = &RefCell::new(Memory::default());

        let list = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let list_ref =
            Reference::try_mut_from(ValueContainer::from(list)).unwrap();

        // Set existing index
        list_ref
            .try_set_property(0, memory, 1, ValueContainer::from(42))
            .expect("Failed to set existing index");
        let updated_value = list_ref.try_get_property(1).unwrap();
        assert_eq!(updated_value, ValueContainer::from(42));

        // Try to set out-of-bounds index
        let result =
            list_ref.try_set_property(0, memory, 5, ValueContainer::from(99));
        assert_matches!(
            result,
            Err(AccessError::IndexOutOfBounds(IndexOutOfBoundsError {
                index: 5
            }))
        );

        // Try to set index on non-map value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result =
            int_ref.try_set_property(0, memory, 0, ValueContainer::from(99));
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn text_property() {
        let memory = &RefCell::new(Memory::default());

        let struct_val = Map::from(vec![
            (ValueContainer::from("name"), ValueContainer::from("Alice")),
            (ValueContainer::from("age"), ValueContainer::from(30)),
        ]);
        let struct_ref =
            Reference::try_mut_from(ValueContainer::from(struct_val)).unwrap();

        // Set existing property
        struct_ref
            .try_set_property(0, memory, "name", ValueContainer::from("Bob"))
            .expect("Failed to set existing property");
        let name = struct_ref.try_get_property("name").unwrap();
        assert_eq!(name, "Bob".into());

        // Try to set non-existing property
        let result = struct_ref.try_set_property(
            0,
            memory,
            "nonexistent",
            ValueContainer::from("Value"),
        );
        assert_matches!(result, Ok(()));

        // // Try to set property on non-struct value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result = int_ref.try_set_property(
            0,
            memory,
            "name",
            ValueContainer::from("Bob"),
        );
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn immutable_reference_fails() {
        let memory = &RefCell::new(Memory::default());

        let r = Reference::from(42);
        assert_matches!(
            r.try_replace(0, memory, 43),
            Err(AccessError::ImmutableReference)
        );

        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Immutable,
        )
        .unwrap();
        assert_matches!(
            r.try_replace(0, memory, 43),
            Err(AccessError::ImmutableReference)
        );
    }
}
