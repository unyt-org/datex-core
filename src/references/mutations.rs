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
    /// Runs a closure with the current value of this reference.
    pub fn with_value<R, F: FnOnce(&mut Value) -> R>(&self, f: F) -> Option<R> {
        let reference = self.collapse_reference_chain();

        match reference {
            Reference::ValueReference(vr) => {
                match &mut vr.borrow_mut().value_container {
                    ValueContainer::Value(value) => Some(f(value)),
                    ValueContainer::Reference(_) => {
                        unreachable!(
                            "Expected a ValueContainer::Value, but found a Reference"
                        )
                    }
                }
            }
            Reference::TypeReference(_) => None,
        }
    }

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
    use crate::references::reference::{AssignmentError, ReferenceMutability};
    use crate::{
        dif::DIFUpdate, references::reference::Reference,
        values::value_container::ValueContainer,
    };

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

    #[test]
    fn value_change_observe() {
        let int_ref = Reference::from(42);

        let observed_update: Rc<RefCell<Option<DIFUpdate>>> =
            Rc::new(RefCell::new(None));
        let observed_update_clone = Rc::clone(&observed_update);

        // Attach an observer to the reference
        int_ref
            .observe(move |update| {
                *observed_update_clone.borrow_mut() = Some(update.clone());
            })
            .expect("Failed to attach observer");

        // Update the value of the reference
        int_ref.try_set_value(43).expect("Failed to set value");

        // Verify the observed update matches the expected change
        let expected_update = DIFUpdate::Replace(
            DIFValueContainer::try_from(&ValueContainer::from(43)).unwrap(),
        );
        assert_eq!(*observed_update.borrow(), Some(expected_update));
    }
}
