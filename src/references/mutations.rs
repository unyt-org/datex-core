use crate::dif::update::{DIFKey, DIFUpdateData};
use crate::dif::value::DIFValueContainer;
use crate::references::observers::TransceiverId;
use crate::runtime::memory::Memory;
use crate::stdlib::format;
use crate::stdlib::string::ToString;
use crate::{
    references::reference::{AccessError, Reference},
    values::{core_value::CoreValue, value_container::ValueContainer},
};
use core::cell::RefCell;
use core::ops::FnOnce;
use core::prelude::rust_2024::*;
use crate::values::value_container::ValueKey;

impl Reference {
    /// Internal function that handles updates
    /// - Checks if the reference is mutable
    /// - Calls the provided handler to perform the update and get the DIFUpdateData
    /// - Notifies observers with the update data
    /// - Returns any AccessError encountered
    fn handle_update(
        &self,
        source_id: TransceiverId,
        handler: impl FnOnce() -> Result<DIFUpdateData, AccessError>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }
        let update_data = handler()?;
        self.notify_observers(&update_data.with_source(source_id));
        Ok(())
    }

    /// Sets a property on the value if applicable (e.g. for maps)
    pub fn try_set_property<'a>(
        &self,
        source_id: TransceiverId,
        key: impl Into<ValueKey<'a>>,
        val: ValueContainer,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        let key = key.into();
        self.handle_update(source_id, move || {
            self.with_value_unchecked(|value| {
                match value.inner {
                    CoreValue::Map(ref mut map) => {
                        // If the value is an map, set the property
                        map.try_set(key.clone(), val.clone())?;
                    }
                    CoreValue::List(ref mut list) => {
                        if let Some(index) = key.try_as_index() {
                            list.set(index, val.clone()).map_err(|err| {
                                AccessError::IndexOutOfBounds(err)
                            })?;
                        }
                        else {
                            return Err(AccessError::InvalidIndexKey);
                        }
                    }
                    CoreValue::Text(ref mut text) => {
                        if let Some(index) = key.try_as_index() {
                            if let ValueContainer::Value(v) = &val &&
                                let CoreValue::Text(new_char) = &v.inner && new_char.0.len() == 1 {
                                let char = new_char.0.chars().next().unwrap_or('\0');
                                text.set_char_at(index, char).map_err(|err| AccessError::IndexOutOfBounds(err))?;
                            } else {
                                return Err(AccessError::InvalidOperation(
                                    "Can only set char character in text".to_string(),
                                ));
                            }
                        }
                        else {
                            return Err(AccessError::InvalidIndexKey);
                        }
                    }
                    _ => {
                        // If the value is not an map, we cannot set a property
                        return Err(AccessError::InvalidOperation(format!(
                            "Cannot set property '{}' on non-map value: {:?}",
                            key, value
                        )));
                    }
                }
                Ok(DIFUpdateData::set(
                    DIFKey::from_value_key(&key, memory),
                    DIFValueContainer::from_value_container(&val, memory),
                ))
            })
        })
    }

    /// Sets a value on the reference if it is mutable and the type is compatible.
    pub fn try_replace<T: Into<ValueContainer>>(
        &self,
        source_id: TransceiverId,
        value: T,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        self.handle_update(source_id, move || {
            // TODO #306: ensure type compatibility with allowed_type
            let value_container = &value.into();
            self.with_value_unchecked(|core_value| {
                // Set the value directly, ensuring it is a ValueContainer
                core_value.inner =
                    value_container.to_value().borrow().inner.clone();
                Ok(DIFUpdateData::replace(
                    DIFValueContainer::from_value_container(
                        value_container,
                        memory,
                    ),
                ))
            })
        })
    }

    /// Pushes a value to the reference if it is a list.
    pub fn try_append_value<T: Into<ValueContainer>>(
        &self,
        // TODO #307 move to end
        source_id: TransceiverId,
        value: T,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        self.handle_update(source_id, move || {
            let value_container = value.into();
            self.with_value_unchecked(move |core_value| {
                match &mut core_value.inner {
                    CoreValue::List(list) => {
                        // TODO #308: Can we avoid the clone?
                        list.push(value_container.clone());
                    }
                    _ => {
                        return Err(AccessError::InvalidOperation(format!(
                            "Cannot push value to non-list value: {:?}",
                            core_value
                        )));
                    }
                }
                Ok(DIFUpdateData::append(
                    DIFValueContainer::from_value_container(
                        &value_container,
                        memory,
                    ),
                ))
            })
        })
    }

    /// Tries to delete a property from the reference if it is a map.
    /// Notifies observers if successful.
    pub fn try_delete_property<'a>(
        &self,
        source_id: TransceiverId,
        key: impl Into<ValueKey<'a>>,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        let key = key.into();
        self.handle_update(source_id, move || {
            self.with_value_unchecked(|value| {
                match value.inner {
                    CoreValue::Map(ref mut map) => {
                        key.with_value_container(|key| {
                            map.remove(key)
                        })?;
                    }
                    CoreValue::List(ref mut list) => {
                        if let Some(index) = key.try_as_index() {
                            list.delete(index).map_err(|err| {
                                AccessError::IndexOutOfBounds(err)
                            })?;
                        }
                        else {
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
                Ok(DIFUpdateData::delete(DIFKey::from_value_key(&key, memory)))
            })
        })
    }

    pub fn try_clear(
        &self,
        source_id: TransceiverId,
    ) -> Result<(), AccessError> {
        self.handle_update(source_id, move || {
            self.with_value_unchecked(|value| {
                match value.inner {
                    CoreValue::Map(ref mut map) => {
                        map.clear()?;
                    }
                    _ => {
                        return Err(AccessError::InvalidOperation(format!(
                            "Cannot clear non-list/map value: {:?}",
                            value
                        )));
                    }
                }
                Ok(DIFUpdateData::clear())
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::references::reference::{AccessError, AssignmentError, IndexOutOfBoundsError, ReferenceMutability};
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
            .try_append_value(0, ValueContainer::from(4), memory)
            .expect("Failed to push value to list");
        let updated_value = list_ref.try_get_property(3).unwrap();
        assert_eq!(updated_value, ValueContainer::from(4));

        // Try to push to immutable value
        let int_ref =
            Reference::from(List::from(vec![ValueContainer::from(42)]));
        let result =
            int_ref.try_append_value(0, ValueContainer::from(99), memory);
        assert_matches!(result, Err(AccessError::ImmutableReference));

        // Try to push to non-list value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result =
            int_ref.try_append_value(0, ValueContainer::from(99), memory);
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
            .try_set_property(
                0,
                "key1",
                ValueContainer::from(42),
                memory,
            )
            .expect("Failed to set existing property");
        let updated_value = map_ref
            .try_get_property("key1")
            .unwrap();
        assert_eq!(updated_value, 42.into());

        // Set new property
        let result = map_ref.try_set_property(
            0,
            "new",
            ValueContainer::from(99),
            memory,
        );
        assert!(result.is_ok());
        let new_value = map_ref
            .try_get_property("new")
            .unwrap();
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
            .try_set_property(0, 1, ValueContainer::from(42), memory)
            .expect("Failed to set existing index");
        let updated_value = list_ref.try_get_property(1).unwrap();
        assert_eq!(updated_value, ValueContainer::from(42));

        // Try to set out-of-bounds index
        let result = list_ref.try_set_property(
            0,
            5,
            ValueContainer::from(99),
            memory,
        );
        assert_matches!(result, Err(AccessError::IndexOutOfBounds(IndexOutOfBoundsError { index: 5 })));

        // Try to set index on non-map value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result = int_ref.try_set_property(
            0,
            0,
            ValueContainer::from(99),
            memory,
        );
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
            .try_set_property(
                0,
                "name",
                ValueContainer::from("Bob"),
                memory,
            )
            .expect("Failed to set existing property");
        let name = struct_ref.try_get_property("name").unwrap();
        assert_eq!(name, "Bob".into());

        // Try to set non-existing property
        let result = struct_ref.try_set_property(
            0,
            "nonexistent",
            ValueContainer::from("Value"),
            memory,
        );
        assert_matches!(result, Ok(()));

        // // Try to set property on non-struct value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result = int_ref.try_set_property(
            0,
            "name",
            ValueContainer::from("Bob"),
            memory,
        );
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn immutable_reference_fails() {
        let memory = &RefCell::new(Memory::default());

        let r = Reference::from(42);
        assert_matches!(
            r.try_replace(0, 43, memory),
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
            r.try_replace(0, 43, memory),
            Err(AccessError::ImmutableReference)
        );
    }
}
