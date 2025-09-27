use crate::{dif::DIFUpdate, references::reference::Reference};

#[derive(Debug)]
pub enum ObserveError {
    ImmutableReference,
}
pub type ReferenceObserver = Box<dyn Fn(&DIFUpdate)>;

impl Reference {
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
                return Ok(brw.observers.add(Box::new(observer)));
            }
        }
        // TODO: also set observers on child references if not yet active, keep track of active observers
    }

    pub fn notify_observers(&self, dif: &DIFUpdate) {
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

    pub fn has_observers(&self) -> bool {
        // Check if there are any observers registered
        match self {
            Reference::TypeReference(_) => false,
            Reference::ValueReference(vr) => !vr.borrow().observers.is_empty(),
        }
    }
}
