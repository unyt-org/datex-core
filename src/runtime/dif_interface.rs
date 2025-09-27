use crate::dif::DIFProperty;
use crate::dif::interface::DIFResolveReferenceError;
use crate::dif::r#type::{DIFType, DIFTypeContainer};
use crate::dif::value::DIFValue;
use crate::references::observers::ReferenceObserver;
use crate::references::reference::{AccessError, ReferenceMutability};
use crate::types::type_container;
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
    fn resolve_in_memory_reference(
        &self,
        address: &PointerAddress,
    ) -> Option<Reference> {
        self.memory().borrow().get_reference(address).cloned()
    }
    // FIXME TODO
    async fn resolve_reference(
        &self,
        address: &PointerAddress,
    ) -> Option<Reference> {
        self.memory().borrow().get_reference(address).cloned()
    }
    pub fn as_value_container(
        &self,
        val: &DIFValueContainer,
    ) -> Option<ValueContainer> {
        match val {
            DIFValueContainer::Value(value) => {
                Some(ValueContainer::from(value.clone()))
            }
            DIFValueContainer::Reference(address) => self
                .resolve_in_memory_reference(address)
                .map(ValueContainer::Reference),
        }
    }
    // pub fn as_dif_value_container(
    //     &self,
    //     val: &ValueContainer,
    // ) -> Option<DIFValueContainer> {
    //     match val {
    //         ValueContainer::Value(value) => {
    //             DIFValue::try_from(value).ok().map(DIFValueContainer::Value)
    //         }
    //         ValueContainer::Reference(address) => Some(DIFValueContainer::Reference(
    //             address
    //                 .pointer_address()
    //                 .expect("Reference in ValueContainer must have a pointer address")
    //                 .clone(),
    //         )),
    //     }
    // }
}

impl DIFInterface for Runtime {
    fn update(
        &mut self,
        address: PointerAddress,
        update: DIFUpdate,
    ) -> Result<(), DIFUpdateError> {
        let ptr = self
            .resolve_in_memory_reference(&address)
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
                        ptr.try_set_text_property(key, value_container)
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

    async fn resolve_pointer_address_external(
        &self,
        address: PointerAddress,
    ) -> Result<DIFValueContainer, DIFResolveReferenceError> {
        let ptr = self.resolve_in_memory_reference(&address);
        match ptr {
            Some(ptr) => {
                Ok(DIFValueContainer::try_from(&ptr.value_container()).unwrap())
            }
            None => todo!("Implement async resolution of references"),
        }
    }

    fn resolve_pointer_address_in_memory(
        &self,
        address: PointerAddress,
    ) -> Result<DIFValueContainer, DIFResolveReferenceError> {
        let ptr = self.resolve_in_memory_reference(&address);
        match ptr {
            Some(ptr) => {
                Ok(DIFValueContainer::try_from(&ptr.value_container()).unwrap())
            }
            None => Err(DIFResolveReferenceError::ReferenceNotFound),
        }
    }

    fn apply(
        &mut self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFValueContainer, DIFApplyError> {
        todo!()
    }

    async fn create_pointer(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError> {
        let container = match value {
            DIFValueContainer::Reference(address) => ValueContainer::Reference(
                self.resolve_in_memory_reference(&address)
                    .ok_or(DIFCreatePointerError::ReferenceNotFound)?,
            ),
            DIFValueContainer::Value(v) => ValueContainer::from(v),
        };
        let type_container = if let Some(allowed_type) = &allowed_type {
            todo!(
                "FIXME: Implement type_container creation from DIFTypeContainer"
            )
        } else {
            None
        };
        let reference = Reference::try_new_from_value_container(
            container,
            type_container,
            None,
            mutability,
        )?;
        let address = self.memory().borrow_mut().register_reference(reference);
        Ok(address)
    }

    fn create_pointer_sync(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError> {
        let container = match value {
            DIFValueContainer::Reference(address) => ValueContainer::Reference(
                self.resolve_in_memory_reference(&address)
                    .ok_or(DIFCreatePointerError::ReferenceNotFound)?,
            ),
            DIFValueContainer::Value(v) => ValueContainer::from(v),
        };
        let type_container = if let Some(allowed_type) = &allowed_type {
            todo!(
                "FIXME: Implement type_container creation from DIFTypeContainer"
            )
        } else {
            None
        };
        let reference = Reference::try_new_from_value_container(
            container,
            type_container,
            None,
            mutability,
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
            .resolve_in_memory_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        Ok(ptr.observe(observer)?)
    }

    fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: u32,
    ) -> Result<(), DIFObserveError> {
        let ptr = self
            .resolve_in_memory_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        ptr.unobserve(observer_id)
            .map_err(DIFObserveError::ObserveError)
    }
}
