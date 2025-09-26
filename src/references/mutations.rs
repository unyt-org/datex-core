use crate::{
    dif::{DIFUpdate, value::DIFValue},
    references::reference::{AccessError, ObserveError, Reference},
    values::{
        core_value::CoreValue, value::Value, value_container::ValueContainer,
    },
};
use crate::dif::value::DIFValueContainer;

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

    /// Sets a text property on the value if applicable (e.g. for objects)
    pub fn try_set_text_property(
        &self,
        key: &str,
        mut val: ValueContainer,
    ) -> Result<(), AccessError> {
        // Ensure the value is a reference if it is a combined value (e.g. an object)
        val = val.upgrade_combined_value_to_reference();

        self.with_value(|value| {
            match value.inner {
                CoreValue::Map(ref mut obj) => {
                    // If the value is an object, set the property
                    obj.set(key, self.bind_child(val));
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

    /// Adds an observer to this reference that will be notified on value changes.
    /// Returns an error if the reference is immutable
    pub fn observe<F: Fn(&DIFUpdate) + 'static>(
        &self,
        observer: F,
    ) -> Result<u32, ObserveError> {
        // Add the observer to the list of observers
        match self {
            Reference::TypeReference(_) => {
                // Type references do not have observers
                Err(ObserveError::ImmutableReference)
            }
            Reference::ValueReference(vr) => {
                let mut brw = vr.borrow_mut();
                if !brw.is_mutable() {
                    return Err(ObserveError::ImmutableReference);
                }

                let next_id = brw.next_observer_id;
                brw.observers.insert(next_id, Box::new(observer));
                brw.next_observer_id += 1;
                Ok(vr.borrow().observers.len() as u32)
            }
        }
        // TODO: also set observers on child references if not yet active, keep track of active observers
    }

    fn notify_observers(&self, dif: &DIFUpdate) {
        match self {
            Reference::TypeReference(_) => {
                // Type references do not have observers
            }
            Reference::ValueReference(vr) => {
                /// Notify all observers of the update
                for (_, observer) in &vr.borrow().observers {
                    observer(dif);
                }
            }
        }
    }

    fn has_observers(&self) -> bool {
        // Check if there are any observers registered
        match self {
            Reference::TypeReference(_) => false,
            Reference::ValueReference(vr) => !vr.borrow().observers.is_empty(),
        }
    }

    pub fn try_set_value<T: Into<ValueContainer>>(
        &self,
        value: T,
    ) -> Result<(), String> {
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
            let dif = DIFUpdate::Replace(DIFValueContainer::try_from(value_container).unwrap());
            self.notify_observers(&dif);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        dif::{DIFUpdate, value::DIFValue},
        references::reference::Reference,
        values::value_container::ValueContainer,
    };
    use crate::dif::value::DIFValueContainer;

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
        let expected_update =
            DIFUpdate::Replace(DIFValueContainer::try_from(&ValueContainer::from(43)).unwrap());
        assert_eq!(*observed_update.borrow(), Some(expected_update));
    }
}
