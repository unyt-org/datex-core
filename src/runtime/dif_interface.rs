use crate::dif::DIFProperty;
use crate::dif::interface::{DIFFreeError, DIFResolveReferenceError};
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
        &self,
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
                let value_container = value
                    .to_value_container(&self.memory().borrow())?;
                match property {
                    DIFProperty::Text(key) => {
                        ptr.try_set_text_property(&key, value_container)
                            .map_err(DIFUpdateError::AccessError)?;
                    }
                    DIFProperty::Integer(key) => {
                        ptr.try_set_numeric_property(
                            key as u32,
                            value_container,
                        )
                        .map_err(DIFUpdateError::AccessError)?;
                    }
                    DIFProperty::Value(key) => {
                        let key = key
                            .to_value_container(&self.memory().borrow())?;
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
                    new_value.to_value_container(&self.memory().borrow())?,
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
                    new_value.to_value_container(&self.memory().borrow())?,
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
        &self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFValueContainer, DIFApplyError> {
        todo!()
    }

    fn create_pointer(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError> {
        let container = value.to_value_container(&self.memory().borrow())?;
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

    fn observe_pointer<F: Fn(&DIFUpdate) + 'static>(
        &self,
        address: PointerAddress,
        observer: F,
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

    fn free_pointer(&self, address: PointerAddress) -> Result<(), DIFFreeError> {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use datex_core::runtime::RuntimeConfig;
    use crate::dif::dif_representation::DIFRepresentationValue;
    use crate::dif::interface::DIFInterface;
    use crate::dif::value::{DIFValue, DIFValueContainer};
    use crate::references::reference::ReferenceMutability;
    use crate::runtime::Runtime;

    #[test]
    fn test_create_and_observe_pointer() {
        let runtime = Runtime::init_native(RuntimeConfig::default());
        let pointer_address = runtime
            .create_pointer(
                DIFValueContainer::Value(
                    DIFValue::from(
                        DIFRepresentationValue::String("Hello, world!".to_string()),
                    ),
                ),
                None,
                ReferenceMutability::Mutable,
            )
            .expect("Failed to create pointer");

        let observed = Rc::new(RefCell::new(None));
        let observed_clone = observed.clone();

        let observer_id  = Rc::new(RefCell::new(None));
        let observer_id_clone = observer_id.clone();
        let runtime_clone = runtime.clone();
        let pointer_address_clone = pointer_address.clone();

        // Observe the pointer
        observer_id.replace(Some(runtime
            .observe_pointer(pointer_address_clone.clone(), move |update| {
                println!("Observed pointer value: {:?}", update);
                observed_clone.replace(Some(update.clone()));
                // unobserve after first update
                runtime_clone.unobserve_pointer(pointer_address_clone.clone(), observer_id_clone.borrow().unwrap()).unwrap();
            })
            .expect("Failed to observe pointer")
        ));

        // Update the pointer value
        runtime
            .update(
                pointer_address.clone(),
                crate::dif::DIFUpdate::Replace(DIFValueContainer::Value(
                    DIFValue::from(DIFRepresentationValue::String(
                        "Hello, Datex!".to_string(),
                    )),
                )),
            )
            .expect("Failed to update pointer");

        // Check if the observed value matches the update
        let observed_value = observed.borrow();
        assert_eq!(
            *observed_value,
            Some(crate::dif::DIFUpdate::Replace(DIFValueContainer::Value(
                DIFValue::from(DIFRepresentationValue::String(
                    "Hello, Datex!".to_string(),
                )),
            )))
        );

        // try unobserve again, should fail
        assert!(runtime.unobserve_pointer(pointer_address.clone(), observer_id.borrow().unwrap()).is_err());
    }
}