use crate::references::observers::ReferenceObserver;
use crate::references::reference::ReferenceMutability;
use crate::{
    dif::{
        DIFUpdate,
        interface::{
            DIFApplyError, DIFCreatePointerError, DIFInterface,
            DIFObserveError, DIFUpdateError,
        },
        value::DIFValueContainer,
    },
    references::reference::Reference,
    runtime::{Runtime, execution::ExecutionError},
    values::{pointer::PointerAddress, value_container::ValueContainer},
};

impl Runtime {
    fn resolve_reference(&self, address: &PointerAddress) -> Option<Reference> {
        self.memory().borrow().get_reference(&address).cloned()
    }
}

impl DIFInterface for Runtime {
    fn update(
        &mut self,
        address: PointerAddress,
        update: DIFUpdate,
    ) -> Result<(), DIFUpdateError> {
        todo!()
    }

    fn apply(
        &mut self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFApplyError, ExecutionError> {
        todo!()
    }

    fn create_pointer(
        &self,
        value: DIFValueContainer,
    ) -> Result<PointerAddress, DIFCreatePointerError> {
        let container = match value {
            DIFValueContainer::Reference(address) => ValueContainer::Reference(
                self.resolve_reference(&address)
                    .ok_or(DIFCreatePointerError::ReferenceNotFound)?,
            ),
            DIFValueContainer::Value(v) => ValueContainer::from(v),
        };
        let reference = Reference::try_new_from_value_container(
            container,                      // TODO
            None,                           // TODO
            None,                           // TODO
            ReferenceMutability::Immutable, // TODO
        )?;
        let address = self.memory().borrow_mut().register_reference(reference);
        Ok(address)
    }

    fn observe_pointer(
        &self,
        address: PointerAddress,
        observer: ReferenceObserver,
    ) -> Result<u32, DIFObserveError> {
        let ptr = self
            .resolve_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        Ok(ptr.observe(observer)?)
    }

    fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: u32,
    ) -> Result<(), DIFObserveError> {
        let ptr = self
            .resolve_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        ptr.unobserve(observer_id)
            .map_err(DIFObserveError::ObserveError)
    }
}
