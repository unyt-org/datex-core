use crate::dif::update::DIFUpdate;
use crate::references::{
    reference::Reference, value_reference::ValueReference,
};
use serde::{Deserialize, Serialize};
use crate::stdlib::{cell::RefCell, fmt::Display, rc::Rc};

#[derive(Debug)]
pub enum ObserverError {
    ObserverNotFound,
    ImmutableReference,
}

impl Display for ObserverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ObserverError::ObserverNotFound => {
                core::write!(f, "Observer not found")
            }
            ObserverError::ImmutableReference => {
                core::write!(f, "Cannot observe an immutable reference")
            }
        }
    }
}

pub type ObserverCallback = Rc<dyn Fn(&DIFUpdate)>;

/// unique identifier for a transceiver (source of updates)
/// 0-255 are reserved for DIF clients
pub type TransceiverId = u32;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ObserveOptions {
    /// If true, the transceiver will be notified of changes that originated from itself
    pub relay_own_updates: bool,
}

#[derive(Clone)]
pub struct Observer {
    pub transceiver_id: TransceiverId,
    pub options: ObserveOptions,
    pub callback: ObserverCallback,
}

impl Observer {
    /// Creates a new observer with the given callback function,
    /// using default options and a transceiver ID of 0.
    pub fn new<F: Fn(&DIFUpdate) + 'static>(callback: F) -> Self {
        Observer {
            transceiver_id: 0,
            options: ObserveOptions::default(),
            callback: Rc::new(callback),
        }
    }
}

impl Reference {
    /// Adds an observer to this reference that will be notified on value changes.
    /// Returns an error if the reference is immutable or a type reference.
    /// The returned u32 is an observer ID that can be used to remove the observer later.
    pub fn observe(&self, observer: Observer) -> Result<u32, ObserverError> {
        // Add the observer to the list of observers
        Ok(self
            .ensure_mutable_value_reference()?
            .borrow_mut()
            .observers
            .add(observer))

        // TODO #299: also set observers on child references if not yet active, keep track of active observers
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

    /// Updates the options for an existing observer by its ID.
    /// Returns an error if the observer ID is not found or the reference is immutable.
    pub fn update_observer_options(
        &self,
        observer_id: u32,
        options: ObserveOptions,
    ) -> Result<(), ObserverError> {
        let vr = self.ensure_mutable_value_reference()?;
        let mut vr_borrow = vr.borrow_mut();
        if let Some(observer) = vr_borrow.observers.get_mut(&observer_id) {
            observer.options = options;
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
        self.mutable_reference()
            .ok_or(ObserverError::ImmutableReference)
    }

    /// Notifies all observers of a change represented by the given DIFUpdate.
    pub fn notify_observers(&self, dif: &DIFUpdate) {
        let observer_callbacks: Vec<ObserverCallback> = match self {
            Reference::TypeReference(_) => return, // no observers
            Reference::ValueReference(vr) => {
                // Clone observers while holding borrow
                let vr_ref = vr.borrow();
                vr_ref
                    .observers
                    .iter()
                    .filter(|(_, f)| {
                        // Filter out bounced back transceiver updates if relay_own_updates not enabled
                        f.options.relay_own_updates
                            || f.transceiver_id != dif.source_id
                    })
                    .map(|(_, f)| f.callback.clone())
                    .collect()
            }
        };

        // Call each observer synchronously
        for callback in observer_callbacks {
            callback(dif);
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
    use crate::dif::update::{DIFUpdate, DIFUpdateData};
    use crate::references::observers::{ObserveOptions, TransceiverId};
    use crate::runtime::memory::Memory;
    use crate::values::core_values::map::Map;
    use crate::{
        dif::{
            representation::DIFValueRepresentation,
            r#type::DIFTypeContainer,
            value::{DIFValue, DIFValueContainer},
        },
        references::{
            observers::ObserverError,
            reference::{Reference, ReferenceMutability},
        },
        values::value_container::ValueContainer,
    };
    use datex_core::references::observers::Observer;
    use crate::stdlib::{assert_matches::assert_matches, cell::RefCell, rc::Rc};

    /// Helper function to record DIF updates observed on a reference
    /// Returns a Rc<RefCell<Vec<DIFUpdate>>> that contains all observed updates
    /// The caller can borrow this to inspect the updates after performing operations on the reference
    fn record_dif_updates(
        reference: &Reference,
        transceiver_id: TransceiverId,
        observe_options: ObserveOptions,
    ) -> Rc<RefCell<Vec<DIFUpdate>>> {
        let updates: Rc<RefCell<Vec<DIFUpdate>>> =
            Rc::new(RefCell::new(vec![]));
        let updates_clone = Rc::clone(&updates);
        reference
            .observe(Observer {
                transceiver_id,
                options: observe_options,
                callback: Rc::new(move |update| {
                    updates_clone.borrow_mut().push(update.clone());
                }),
            })
            .expect("Failed to attach observer");
        updates
    }

    #[test]
    fn immutable_reference_observe_fails() {
        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Immutable,
        )
        .unwrap();
        assert_matches!(
            r.observe(Observer::new(|_| {})),
            Err(ObserverError::ImmutableReference)
        );

        let r = Reference::try_new_from_value_container(
            42.into(),
            None,
            None,
            ReferenceMutability::Mutable,
        )
        .unwrap();
        assert_matches!(r.observe(Observer::new(|_| {})), Ok(_));
    }

    #[test]
    fn observe_and_unobserve() {
        let r = Reference::try_mut_from(42.into()).unwrap();
        assert!(!r.has_observers());
        let observer_id = r.observe(Observer::new(|_| {})).unwrap();
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
        let id1 = r.observe(Observer::new(|_| {})).unwrap();
        let id2 = r.observe(Observer::new(|_| {})).unwrap();
        assert!(id1 == 0);
        assert!(id2 == 1);
        assert!(r.unobserve(id1).is_ok());
        let id3 = r.observe(Observer::new(|_| {})).unwrap();
        assert!(id3 == 0);
        let id4 = r.observe(Observer::new(|_| {})).unwrap();
        assert!(id4 == 2);
    }

    #[test]
    fn observe_replace() {
        let memory = &RefCell::new(Memory::default());

        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let observed_update =
            record_dif_updates(&int_ref, 0, ObserveOptions::default());

        // Update the value of the reference
        int_ref
            .try_set_value(1, 43, memory)
            .expect("Failed to set value");

        // Verify the observed update matches the expected change
        let expected_update = DIFUpdate {
            source_id: 1,
            data: DIFUpdateData::replace(
                DIFValueContainer::from_value_container(
                    &ValueContainer::from(43),
                    memory,
                ),
            ),
        };

        assert_eq!(*observed_update.borrow(), vec![expected_update]);
    }

    #[test]
    fn observe_replace_same_transceiver() {
        let memory = &RefCell::new(Memory::default());

        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let observed_update =
            record_dif_updates(&int_ref, 0, ObserveOptions::default());

        // Update the value of the reference
        int_ref
            .try_set_value(0, 43, memory)
            .expect("Failed to set value");

        // No update triggered, same transceiver id
        assert_eq!(*observed_update.borrow(), vec![]);
    }

    #[test]
    fn observe_replace_same_transceiver_relay_own_updates() {
        let memory = &RefCell::new(Memory::default());

        let int_ref = Reference::try_mut_from(42.into()).unwrap();
        let observed_update = record_dif_updates(
            &int_ref,
            0,
            ObserveOptions {
                relay_own_updates: true,
            },
        );

        // Update the value of the reference
        int_ref
            .try_set_value(0, 43, memory)
            .expect("Failed to set value");

        // update triggered, same transceiver id but relay_own_updates enabled
        let expected_update = DIFUpdate {
            source_id: 0,
            data: DIFUpdateData::replace(
                DIFValueContainer::from_value_container(
                    &ValueContainer::from(43),
                    memory,
                ),
            ),
        };

        assert_eq!(*observed_update.borrow(), vec![expected_update]);
    }

    #[test]
    fn observe_update_property() {
        let memory = &RefCell::new(Memory::default());

        let reference = Reference::try_mut_from(
            Map::from(vec![
                ("a".to_string(), ValueContainer::from(1)),
                ("b".to_string(), ValueContainer::from(2)),
            ])
            .into(),
        )
        .unwrap();
        let observed_updates =
            record_dif_updates(&reference, 0, ObserveOptions::default());
        // Update a property
        reference
            .try_set_text_property(1, "a", "val".into(), memory)
            .expect("Failed to set property");
        // Verify the observed update matches the expected change
        let expected_update = DIFUpdate {
            source_id: 1,
            data: DIFUpdateData::set(
                "a",
                DIFValue::new(
                    DIFValueRepresentation::String("val".to_string()),
                    None as Option<DIFTypeContainer>,
                ),
            ),
        };
        assert_eq!(*observed_updates.borrow(), vec![expected_update]);
    }
}
