use crate::dif::DIFProperty;
use crate::references::observers::ReferenceObserver;
use crate::references::reference::{AccessError, ReferenceMutability};
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
        self.memory().borrow().get_reference(address).cloned()
    }
    fn as_value_container(
        &self,
        val: &DIFValueContainer,
    ) -> Option<ValueContainer> {
        match val {
            DIFValueContainer::Value(value) => {
                Some(ValueContainer::from(value.clone()))
            }
            DIFValueContainer::Reference(address) => self
                .resolve_reference(&address)
                .map(ValueContainer::Reference),
        }
    }
}

impl DIFInterface for Runtime {
    fn update(
        &mut self,
        address: PointerAddress,
        update: DIFUpdate,
    ) -> Result<(), DIFUpdateError> {
        let ptr = self
            .resolve_reference(&address)
            .ok_or(DIFUpdateError::ReferenceNotFound)?;
        match update {
            DIFUpdate::UpdateProperty { property, value } => {
                if !ptr.supports_property_access() {
                    return Err(DIFUpdateError::AccessError(
                        AccessError::InvalidOperation(
                            "Reference does not support property access"
                                .to_string(),
                        ),
                    ));
                }
                let value_container = self
                    .as_value_container(&value)
                    .ok_or(DIFUpdateError::ReferenceNotFound)?;
                match &property {
                    DIFProperty::Text(key) => {
                        ptr.try_set_text_property(&key, value_container)
                            .map_err(DIFUpdateError::AccessError)?;
                    }
                    DIFProperty::Integer(key) => {
                        ptr.try_set_numeric_property(
                            *key as u32,
                            value_container,
                        )
                        .map_err(DIFUpdateError::AccessError)?;
                    }
                    DIFProperty::Value(key) => {
                        let key = self
                            .as_value_container(key)
                            .ok_or(DIFUpdateError::ReferenceNotFound)?;
                        ptr.try_set_property(key, value_container)
                            .map_err(DIFUpdateError::AccessError)?;
                    }
                }

                // ptr.notify_observers(&DIFUpdate::UpdateProperty {
                //     property,
                //     value,
                // });
                Ok(())
            }
            DIFUpdate::Replace(new_value) => {
                ptr.try_set_value(
                    self.as_value_container(&new_value)
                        .ok_or(DIFUpdateError::ReferenceNotFound)?,
                )
                .map_err(DIFUpdateError::AssignmentError)?;

                // ptr.notify_observers(&DIFUpdate::Replace(new_value));
                Ok(())
            }
            DIFUpdate::Push(new_value) => {
                if !ptr.supports_push() {
                    return Err(DIFUpdateError::AccessError(
                        AccessError::InvalidOperation(
                            "Reference does not support push operation"
                                .to_string(),
                        ),
                    ));
                }
                ptr.try_push_value(
                    self.as_value_container(&new_value)
                        .ok_or(DIFUpdateError::ReferenceNotFound)?,
                )
                .map_err(DIFUpdateError::AccessError)?;

                // ptr.notify_observers(&DIFUpdate::Push(new_value));
                Ok(())
            }
        }
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
