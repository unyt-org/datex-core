use crate::dif::update::{DIFProperty, DIFUpdate};
use crate::dif::value::DIFValueContainer;
use crate::references::reference::AssignmentError;
use crate::runtime::memory::Memory;
use crate::{
    references::reference::{AccessError, Reference},
    values::{core_value::CoreValue, value_container::ValueContainer},
};
use std::cell::RefCell;

impl Reference {
    /// Sets a property on the value if applicable (e.g. for objects and structs)
    pub fn try_set_property(
        &self,
        key: ValueContainer,
        val: ValueContainer,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }
        let val = val.upgrade_combined_value_to_reference();
        let dif_val = DIFValueContainer::from_value_container(&val, memory);
        let dif_key = DIFValueContainer::from_value_container(&key, memory);
        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    // If the value is an object, set the property
                    map.set(key, val)?;
                }
                _ => {
                    // If the value is not an object, we cannot set a property
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot set property '{}' on non-object value: {:?}",
                        key, value
                    )));
                }
            }
            Ok(())
        })?;
        self.notify_observers(&DIFUpdate::set(dif_key, dif_val));
        Ok(())
    }

    /// Sets a text property on the value if applicable (e.g. for structs)
    pub fn try_set_text_property(
        &self,
        key: &str,
        val: ValueContainer,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }

        // Ensure the value is a reference if it is a combined value (e.g. an object)
        let val = val.upgrade_combined_value_to_reference();
        let dif = DIFValueContainer::from_value_container(&val, memory);
        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::Map(ref mut obj) => {
                    // If the value is an object, set the property
                    obj.set(key, val)?;
                }
                _ => {
                    // If the value is not an object, we cannot set a property
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot set property '{}' on non-object value: {:?}",
                        key, value
                    )));
                }
            }
            Ok(())
        })?;

        self.notify_observers(&DIFUpdate::set(key, dif));
        Ok(())
    }

    pub fn try_set_numeric_property(
        &self,
        index: u32,
        val: ValueContainer,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }

        let val = val.upgrade_combined_value_to_reference();
        let dif = DIFValueContainer::from_value_container(&val, memory);
        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::List(ref mut list) => {
                    list.set(index, self.bind_child(val)).ok_or({
                        AccessError::IndexOutOfBounds(index)
                    })?;
                }
                CoreValue::Text(ref mut text) => {
                    if let ValueContainer::Value(v) = &val {
                        if let CoreValue::Text(new_char) = &v.inner && new_char.0.len() == 1 {
                            let char = new_char.0.chars().next().unwrap_or('\0');
                            text.set_char_at(index as usize, char).map_err(| _| AccessError::IndexOutOfBounds(index))?;
                        } else {
                            return Err(AccessError::InvalidOperation(
                                "Can only set char character in text".to_string(),
                            ));
                        }
                    } else {
                        return Err(AccessError::CanNotUseReferenceAsKey);
                    }
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot set numeric property '{}' on non-array/list/text value: {:?}",
                        index, value
                    )));
                }
            }
            Ok(())
        })?;

        self.notify_observers(&DIFUpdate::set(
            DIFProperty::Index(index as i64),
            dif,
        ));
        Ok(())
    }

    /// Sets a value on the reference if it is mutable and the type is compatible.
    pub fn try_set_value<T: Into<ValueContainer>>(
        &self,
        value: T,
        memory: &RefCell<Memory>,
    ) -> Result<(), AssignmentError> {
        if !self.is_mutable() {
            return Err(AssignmentError::ImmutableReference);
        }
        // TODO: ensure type compatibility with allowed_type
        let value_container = &value.into();
        self.with_value_unchecked(|core_value| {
            // Set the value directly, ensuring it is a ValueContainer
            core_value.inner =
                value_container.to_value().borrow().inner.clone();
            Ok(())
        })?;

        self.notify_observers(&DIFUpdate::replace(
            DIFValueContainer::from_value_container(value_container, memory),
        ));
        Ok(())
    }

    /// Pushes a value to the reference if it is a list or array.
    pub fn try_push_value<T: Into<ValueContainer>>(
        &self,
        value: T,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }

        let value_container =
            value.into().upgrade_combined_value_to_reference();
        let dif =
            DIFValueContainer::from_value_container(&value_container, memory);
        self.with_value_unchecked(move |core_value| {
            match &mut core_value.inner {
                CoreValue::List(list) => {
                    list.push(value_container);
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot push value to non-list/array value: {:?}",
                        core_value
                    )));
                }
            }
            Ok(())
        })?;

        self.notify_observers(&DIFUpdate::push(dif));
        Ok(())
    }

    /// Tries to delete a property from the reference if it is a map/object.
    /// Notifies observers if successful.
    pub fn try_delete_property(
        &self,
        key: ValueContainer,
        memory: &RefCell<Memory>,
    ) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }
        let key = key.upgrade_combined_value_to_reference();
        let dif_key = DIFValueContainer::from_value_container(&key, memory);
        self.with_value_unchecked(|value| {
            match value.inner {
                CoreValue::Map(ref mut map) => {
                    map.remove(&key)?;
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot delete property '{:?}' on non-object value: {:?}",
                        key, value
                    )));
                }
            }
            Ok(())
        })?;

        self.notify_observers(&DIFUpdate::remove(dif_key));
        Ok(())
    }

    pub fn try_clear(&self) -> Result<(), AccessError> {
        if !self.is_mutable() {
            return Err(AccessError::ImmutableReference);
        }

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
            Ok(())
        })?;

        self.notify_observers(&DIFUpdate::clear());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::references::reference::{
        AccessError, AssignmentError, ReferenceMutability,
    };
    use crate::runtime::memory::Memory;
    use crate::values::core_values::list::List;
    use crate::values::core_values::map::Map;
    use crate::{
        references::reference::Reference,
        values::value_container::ValueContainer,
    };
    use std::assert_matches::assert_matches;
    use std::cell::RefCell;

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
            .try_push_value(ValueContainer::from(4), memory)
            .expect("Failed to push value to list");
        let updated_value = list_ref.get_numeric_property(3).unwrap();
        assert_eq!(updated_value, ValueContainer::from(4));

        // Try to push to non-list value
        let int_ref = Reference::from(42);
        let result = int_ref.try_push_value(ValueContainer::from(99), memory);
        assert_matches!(result, Err(AccessError::ImmutableReference));

        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result = int_ref.try_push_value(ValueContainer::from(99), memory);
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
            .try_set_property("key1".into(), ValueContainer::from(42), memory)
            .expect("Failed to set existing property");
        let updated_value = map_ref
            .try_get_property(ValueContainer::from("key1"))
            .unwrap();
        assert_eq!(updated_value, 42.into());

        // Set new property
        let result = map_ref.try_set_property(
            "new".into(),
            ValueContainer::from(99),
            memory,
        );
        assert!(result.is_ok());
        let new_value = map_ref
            .try_get_property(ValueContainer::from("new"))
            .unwrap();
        assert_eq!(new_value, 99.into());
    }

    #[test]
    fn numeric_property() {
        let memory = &RefCell::new(Memory::default());

        let arr = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let arr_ref =
            Reference::try_mut_from(ValueContainer::from(arr)).unwrap();

        // Set existing index
        arr_ref
            .try_set_numeric_property(1, ValueContainer::from(42), memory)
            .expect("Failed to set existing index");
        let updated_value = arr_ref.get_numeric_property(1).unwrap();
        assert_eq!(updated_value, ValueContainer::from(42));

        // Try to set out-of-bounds index
        let result = arr_ref.try_set_numeric_property(
            5,
            ValueContainer::from(99),
            memory,
        );
        assert_matches!(result, Err(AccessError::IndexOutOfBounds(5)));

        // Try to set index on non-array value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result = int_ref.try_set_numeric_property(
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
            .try_set_text_property("name", ValueContainer::from("Bob"), memory)
            .expect("Failed to set existing property");
        let name = struct_ref.try_get_text_property("name").unwrap();
        assert_eq!(name, "Bob".into());

        // Try to set non-existing property
        let result = struct_ref.try_set_text_property(
            "nonexistent",
            ValueContainer::from("Value"),
            memory,
        );
        assert_matches!(result, Ok(()));

        // // Try to set property on non-struct value
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let result = int_ref.try_set_text_property(
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
            r.try_set_value(43, memory),
            Err(AssignmentError::ImmutableReference)
        );

        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Final,
        )
        .unwrap();
        assert_matches!(
            r.try_set_value(43, memory),
            Err(AssignmentError::ImmutableReference)
        );

        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Immutable,
        )
        .unwrap();
        assert_matches!(
            r.try_set_value(43, memory),
            Err(AssignmentError::ImmutableReference)
        );
    }
}
