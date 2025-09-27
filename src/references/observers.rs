use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::{
    dif::DIFUpdate,
    references::{reference::Reference, value_reference::ValueReference},
};

#[derive(Debug)]
pub enum ObserverError {
    ObserverNotFound,
    ImmutableReference,
}

impl Display for ObserverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObserverError::ObserverNotFound => {
                write!(f, "Observer not found")
            }
            ObserverError::ImmutableReference => {
                write!(f, "Cannot observe an immutable reference")
            }
        }
    }
}

pub type ReferenceObserver = Rc<dyn Fn(&DIFUpdate)>;

impl Reference {
    /// Adds an observer to this reference that will be notified on value changes.
    /// Returns an error if the reference is immutable or a type reference.
    /// The returned u32 is an observer ID that can be used to remove the observer later.
    pub fn observe<F: Fn(&DIFUpdate) + 'static>(
        &self,
        observer: F,
    ) -> Result<u32, ObserverError> {
        // Add the observer to the list of observers
        Ok(self
            .ensure_mutable_value_reference()?
            .borrow_mut()
            .observers
            .add(Rc::new(observer)))

        // TODO: also set observers on child references if not yet active, keep track of active observers
    }

    /// Removes an observer by its ID.
    /// Returns an error if the observer ID is not found or the reference is immutable.
    pub fn unobserve(&self, observer_id: u32) -> Result<(), ObserverError> {
        let removed = self
            .ensure_mutable_value_reference()?
            .borrow_mut()
            .observers
            .remove(observer_id);
        if removed.is_some() {
            Ok(())
        } else {
            Err(ObserverError::ObserverNotFound)
        }
    }

    /// Returns a list of all observer IDs currently registered to this reference.
    /// A type reference or immutable reference will always return an empty list.
    pub fn observers_ids(&self) -> Vec<u32> {
        match self {
            Reference::TypeReference(_) => vec![],
            Reference::ValueReference(vr) => {
                vr.borrow().observers.keys().cloned().collect()
            }
        }
    }

    /// Removes all observers from this reference.
    /// Returns an error if the reference is immutable.
    pub fn unobserve_all(&self) -> Result<(), ObserverError> {
        self.ensure_mutable_value_reference()?;
        for id in self.observers_ids() {
            let _ = self.unobserve(id);
        }
        Ok(())
    }

    /// Ensures that this reference is a mutable value reference and returns it.
    /// Returns an ObserverError if the reference is immutable or a type reference.
    fn ensure_mutable_value_reference(
        &self,
    ) -> Result<Rc<RefCell<ValueReference>>, ObserverError> {
        self.mutable_value_reference()
            .ok_or(ObserverError::ImmutableReference)
    }

    /// Notifies all observers of a change represented by the given DIFUpdate.
    pub fn notify_observers(&self, dif: &DIFUpdate) {
        match self {
            Reference::TypeReference(_) => {
                // Type references do not have observers
            }
            Reference::ValueReference(vr) => {
                // Notify all observers of the update
                // Clone the observers to avoid borrowing issues if an unobserve is called during observation
                let observers: Vec<_> = vr.borrow().observers
                    .iter()
                    .map(|(_, f)| f.clone()).
                    collect();
                for observer in observers {
                    observer(dif);
                }
            }
        }
    }

    /// Check if there are any observers registered
    pub fn has_observers(&self) -> bool {
        match self {
            Reference::TypeReference(_) => false,
            Reference::ValueReference(vr) => !vr.borrow().observers.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{assert_matches::assert_matches, cell::RefCell, rc::Rc};

    use crate::{
        dif::{
            DIFProperty, DIFUpdate,
            dif_representation::DIFRepresentationValue,
            r#type::DIFTypeContainer,
            value::{DIFValue, DIFValueContainer},
        },
        references::{
            observers::ObserverError,
            reference::{Reference, ReferenceMutability},
        },
        values::{
            core_values::r#struct::Struct, value_container::ValueContainer,
        },
    };

    /// Helper function to record DIF updates observed on a reference
    /// Returns a Rc<RefCell<Vec<DIFUpdate>>> that contains all observed updates
    /// The caller can borrow this to inspect the updates after performing operations on the reference
    fn record_dif_updates(
        reference: &Reference,
    ) -> Rc<RefCell<Vec<DIFUpdate>>> {
        let updates: Rc<RefCell<Vec<DIFUpdate>>> =
            Rc::new(RefCell::new(vec![]));
        let updates_clone = Rc::clone(&updates);
        reference
            .observe(move |update| {
                updates_clone.borrow_mut().push(update.clone());
            })
            .expect("Failed to attach observer");
        updates
    }

    #[test]
    fn immutable_reference_observe_fails() {
        let r = Reference::from(42);
        assert_matches!(
            r.observe(|_| {}),
            Err(ObserverError::ImmutableReference)
        );

        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Final,
        )
        .unwrap();
        assert_matches!(
            r.observe(|_| {}),
            Err(ObserverError::ImmutableReference)
        );

        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Immutable,
        )
        .unwrap();
        assert_matches!(
            r.observe(|_| {}),
            Err(ObserverError::ImmutableReference)
        );
    }

    #[test]
    fn observe_and_unobserve() {
        let r = Reference::try_mut_from(42.into()).unwrap();
        assert!(!r.has_observers());
        let observer_id = r.observe(|_| {}).unwrap();
        assert!(observer_id == 0);
        assert!(r.has_observers());
        assert!(r.unobserve(observer_id).is_ok());
        assert!(!r.has_observers());
        assert_matches!(
            r.unobserve(observer_id),
            Err(ObserverError::ObserverNotFound)
        );
    }

    #[test]
    fn observer_ids_incremental() {
        let r = Reference::try_mut_from(42.into()).unwrap();
        let id1 = r.observe(|_| {}).unwrap();
        let id2 = r.observe(|_| {}).unwrap();
        assert!(id1 == 0);
        assert!(id2 == 1);
        assert!(r.unobserve(id1).is_ok());
        let id3 = r.observe(|_| {}).unwrap();
        assert!(id3 == 0);
        let id4 = r.observe(|_| {}).unwrap();
        assert!(id4 == 2);
    }

    #[test]
    fn observe_replace() {
        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let observed_update = record_dif_updates(&int_ref);

        // Update the value of the reference
        int_ref.try_set_value(43).expect("Failed to set value");

        // Verify the observed update matches the expected change
        let expected_update = DIFUpdate::Replace {
            value: DIFValueContainer::try_from(&ValueContainer::from(43)).unwrap(),
        };
        assert_eq!(*observed_update.borrow(), vec![expected_update]);
    }

    #[test]
    fn observe_update_property() {
        let reference = Reference::try_mut_from(
            Struct::from(vec![
                ("a".to_string(), ValueContainer::from(1)),
                ("b".to_string(), ValueContainer::from(2)),
            ])
            .into(),
        )
        .unwrap();
        let observed_updates = record_dif_updates(&reference);
        // Update a property
        reference
            .try_set_text_property("a", "val".into())
            .expect("Failed to set property");
        // Verify the observed update matches the expected change
        let expected_update = DIFUpdate::UpdateProperty {
            property: DIFProperty::Text("a".to_string()),
            value: DIFValue::new(
                DIFRepresentationValue::String("val".to_string()),
                DIFTypeContainer::none(),
            )
            .as_container(),
        };
        assert_eq!(*observed_updates.borrow(), vec![expected_update]);
    }
}
