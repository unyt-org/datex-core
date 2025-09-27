use std::{cell::RefCell, rc::Rc};

use crate::{
    dif::DIFUpdate,
    references::{reference::Reference, value_reference::ValueReference},
};

#[derive(Debug)]
pub enum ObserveError {
    ObserverNotFound,
    ImmutableReference,
}
pub type ReferenceObserver = Box<dyn Fn(&DIFUpdate)>;

impl Reference {
    /// Adds an observer to this reference that will be notified on value changes.
    /// Returns an error if the reference is immutable or a type reference.
    /// The returned u32 is an observer ID that can be used to remove the observer later.
    pub fn observe<F: Fn(&DIFUpdate) + 'static>(
        &self,
        observer: F,
    ) -> Result<u32, ObserveError> {
        // Add the observer to the list of observers
        Ok(self
            .ensure_mutable_value_reference()?
            .borrow_mut()
            .observers
            .add(Box::new(observer)))

        // TODO: also set observers on child references if not yet active, keep track of active observers
    }

    /// Removes an observer by its ID.
    /// Returns an error if the observer ID is not found or the reference is immutable.
    pub fn unobserve(&self, observer_id: u32) -> Result<(), ObserveError> {
        self.ensure_mutable_value_reference()?
            .borrow_mut()
            .observers
            .remove(observer_id)
            .ok_or(ObserveError::ObserverNotFound)?;
        Ok(())
    }

    fn ensure_mutable_value_reference(
        &self,
    ) -> Result<Rc<RefCell<ValueReference>>, ObserveError> {
        self.mutable_value_reference()
            .ok_or(ObserveError::ImmutableReference)
    }

    /// Notifies all observers of a change represented by the given DIFUpdate.
    pub fn notify_observers(&self, dif: &DIFUpdate) {
        match self {
            Reference::TypeReference(_) => {
                // Type references do not have observers
            }
            Reference::ValueReference(vr) => {
                // Notify all observers of the update
                for (_, observer) in &vr.borrow().observers {
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
