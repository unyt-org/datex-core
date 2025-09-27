use crate::dif::value::DIFValueContainer;
use crate::references::reference::{AssignmentError, TypeError};
use crate::{
    dif::DIFUpdate,
    references::reference::{AccessError, Reference},
    values::{
        core_value::CoreValue, value::Value, value_container::ValueContainer,
    },
};

impl Reference {
    /// Sets a property on the value if applicable (e.g. for objects and structs)
    pub fn try_set_property(
        &self,
        key: ValueContainer,
        val: ValueContainer,
    ) -> Result<(), AccessError> {
        let val = val.upgrade_combined_value_to_reference();
        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut obj) => {
                    // If the value is an object, set the property
                    obj.set(key, self.bind_child(val));
                }
                CoreValue::Struct(ref mut struct_val) => {
                    if let ValueContainer::Value(value) = &key {
                        if value.is_text() {
                            let key_str = value.cast_to_text().0;
                            // If the value is a struct, set the property if it exists
                            if struct_val.has_field(&key_str) {
                                struct_val.set(&key_str, self.bind_child(val));
                            } else {
                                return Err(AccessError::PropertyNotFound(
                                    key_str,
                                ));
                            }
                        } else {
                            return Err(AccessError::InvalidPropertyKeyType(
                                key.actual_type().to_string(),
                            ));
                        }
                    } else {
                        return Err(AccessError::CanNotUseReferenceAsKey);
                    }
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
        })
        .unwrap_or(Err(AccessError::ImmutableReference))
    }

    /// Sets a text property on the value if applicable (e.g. for structs)
    pub fn try_set_text_property(
        &self,
        key: &str,
        val: ValueContainer,
    ) -> Result<(), AccessError> {
        // Ensure the value is a reference if it is a combined value (e.g. an object)
        let val = val.upgrade_combined_value_to_reference();
        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut obj) => {
                    // If the value is an object, set the property
                    obj.set(key, self.bind_child(val));
                }
                CoreValue::Struct(ref mut struct_val) => {
                    // If the value is a struct, set the property if it exists
                    if struct_val.has_field(key) {
                        struct_val.set(key, self.bind_child(val));
                    } else {
                        return Err(AccessError::PropertyNotFound(
                            key.to_string(),
                        ));
                    }
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
        })
        .unwrap_or(Err(AccessError::ImmutableReference))
    }

    pub fn try_set_numeric_property(
        &self,
        index: u32,
        val: ValueContainer,
    ) -> Result<(), AccessError> {
        let val = val.upgrade_combined_value_to_reference();
        self.with_value(|value| {
            match value.inner {
                CoreValue::Array(ref mut arr) => {
                    if index < arr.len() {
                        arr.set(index, self.bind_child(val));
                    } else {
                        return Err(AccessError::PropertyNotFound(index.to_string()));
                    }
                }
                CoreValue::Text(ref mut text) => {
                    if let ValueContainer::Value(v) = &val {
                        if let CoreValue::Text(new_char) = &v.inner && new_char.0.len() == 1 {
                            let char = new_char.0.chars().next().unwrap_or('\0');
                            text.set_char_at(index as usize, char).map_err(| _| AccessError::IndexOutOfBounds)?;
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
        })
        .unwrap_or(Err(AccessError::ImmutableReference))
    }

    /// Sets a value on the reference if it is mutable and the type is compatible.
    pub fn try_set_value<T: Into<ValueContainer>>(
        &self,
        value: T,
    ) -> Result<(), AssignmentError> {
        if !self.is_mutable() {
            return Err(AssignmentError::ImmutableReference);
        }
        // TODO: ensure type compatibility with allowed_type
        let value_container = &value.into();
        self.with_value(|core_value| {
            // Set the value directly, ensuring it is a ValueContainer
            core_value.inner =
                value_container.to_value().borrow().inner.clone();
        });

        // Notify observers of the update
        if self.has_observers() {
            // TODO: no unwrap here
            let dif = DIFUpdate::Replace(
                DIFValueContainer::try_from(value_container).unwrap(),
            );
            self.notify_observers(&dif);
        }

        Ok(())
    }

    /// Pushes a value to the reference if it is a list or array.
    pub fn try_push_value<T: Into<ValueContainer>>(
        &self,
        value: T,
    ) -> Result<(), AccessError> {
        let value_container =
            value.into().upgrade_combined_value_to_reference();
        self.with_value(move |core_value| {
            match &mut core_value.inner {
                CoreValue::List(list) => {
                    list.push(self.bind_child(value_container));
                }
                _ => {
                    return Err(AccessError::InvalidOperation(format!(
                        "Cannot push value to non-list/array value: {:?}",
                        core_value
                    )));
                }
            }
            Ok(())
        })
        .unwrap_or(Err(AccessError::ImmutableReference))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use std::{cell::RefCell, rc::Rc};

    use crate::dif::value::DIFValueContainer;
    use crate::references::reference::{
        AccessError, AssignmentError, ReferenceMutability,
    };
    use crate::values::core_values::list::List;
    use crate::values::core_values::map::Map;
    use crate::values::core_values::r#struct::Struct;
    use crate::{
        dif::DIFUpdate, references::reference::Reference,
        values::value_container::ValueContainer,
    };

    #[test]
    fn push() {
        let list = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let list_ref =
            Reference::try_mut_from(List::from(list).into()).unwrap();
        list_ref
            .try_push_value(ValueContainer::from(4))
            .expect("Failed to push value to list");
        let updated_value = list_ref.get_numeric_property(3).unwrap();
        assert_eq!(updated_value, ValueContainer::from(4));

        // Try to push to non-list value
        let int_ref = Reference::from(42);
        let result = int_ref.try_push_value(ValueContainer::from(99));
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn property() {
        let map = Map::from(vec![
            ("key1".to_string(), ValueContainer::from(1)),
            ("key2".to_string(), ValueContainer::from(2)),
        ]);
        let map_ref =
            Reference::try_mut_from(ValueContainer::from(map)).unwrap();
        // Set existing property
        map_ref
            .try_set_property("key1".into(), ValueContainer::from(42))
            .expect("Failed to set existing property");
        let updated_value = map_ref
            .try_get_property(ValueContainer::from("key1"))
            .unwrap();
        assert_eq!(updated_value, 42.into());

        // Set new property
        let result =
            map_ref.try_set_property("new".into(), ValueContainer::from(99));
        assert!(result.is_ok());
        let new_value = map_ref
            .try_get_property(ValueContainer::from("new"))
            .unwrap();
        assert_eq!(new_value, 99.into());
    }

    #[test]
    fn numeric_property() {
        let arr = vec![
            ValueContainer::from(1),
            ValueContainer::from(2),
            ValueContainer::from(3),
        ];
        let arr_ref =
            Reference::try_mut_from(ValueContainer::from(arr)).unwrap();

        // Set existing index
        arr_ref
            .try_set_numeric_property(1, ValueContainer::from(42))
            .expect("Failed to set existing index");
        let updated_value = arr_ref.get_numeric_property(1).unwrap();
        assert_eq!(updated_value, ValueContainer::from(42));

        // Try to set out-of-bounds index
        let result =
            arr_ref.try_set_numeric_property(5, ValueContainer::from(99));
        assert_matches!(
            result,
            Err(AccessError::PropertyNotFound(idx)) if idx == "5"
        );

        // Try to set index on non-array value
        let int_ref = Reference::from(42);
        let result =
            int_ref.try_set_numeric_property(0, ValueContainer::from(99));
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn text_property() {
        let struct_val = Struct::new(vec![
            ("name".to_string(), ValueContainer::from("Alice")),
            ("age".to_string(), ValueContainer::from(30)),
        ]);
        let struct_ref =
            Reference::try_mut_from(ValueContainer::from(struct_val)).unwrap();

        // Set existing property
        struct_ref
            .try_set_text_property("name", ValueContainer::from("Bob"))
            .expect("Failed to set existing property");
        let name = struct_ref.try_get_text_property("name").unwrap();
        assert_eq!(name, "Bob".into());

        // Try to set non-existing property
        let result = struct_ref.try_set_text_property(
            "nonexistent",
            ValueContainer::from("Value"),
        );
        assert_matches!(
            result,
            Err(AccessError::PropertyNotFound(prop)) if prop == "nonexistent"
        );

        // Try to set property on non-struct value
        let int_ref = Reference::from(42);
        let result =
            int_ref.try_set_text_property("name", ValueContainer::from("Bob"));
        assert_matches!(result, Err(AccessError::InvalidOperation(_)));
    }

    #[test]
    fn immutable_reference_fails() {
        let r = Reference::from(42);
        assert_matches!(
            r.try_set_value(43),
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
            r.try_set_value(43),
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
            r.try_set_value(43),
            Err(AssignmentError::ImmutableReference)
        );
    }
}
