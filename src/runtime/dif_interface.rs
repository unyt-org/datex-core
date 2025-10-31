use crate::dif::interface::DIFResolveReferenceError;
use crate::dif::reference::DIFReference;
use crate::dif::r#type::DIFTypeContainer;
use crate::dif::update::{DIFProperty, DIFUpdateData};
use crate::references::observers::{ObserveOptions, Observer, TransceiverId};
use crate::references::reference::{AccessError, ReferenceMutability};
use crate::runtime::RuntimeInternal;
use crate::values::value_container::ValueContainer;
use crate::{
    dif::{
        interface::{
            DIFApplyError, DIFCreatePointerError, DIFInterface,
            DIFObserveError, DIFUpdateError,
        },
        value::DIFValueContainer,
    },
    references::reference::Reference,
    values::pointer::PointerAddress,
};
use datex_core::dif::update::DIFUpdate;
use std::rc::Rc;

impl RuntimeInternal {
    fn resolve_in_memory_reference(
        &self,
        address: &PointerAddress,
    ) -> Option<Reference> {
        self.memory.borrow().get_reference(address).cloned()
    }
    // FIXME #398 implement async resolution
    async fn resolve_reference(
        &self,
        address: &PointerAddress,
    ) -> Option<Reference> {
        self.memory.borrow().get_reference(address).cloned()
    }
}

impl DIFInterface for RuntimeInternal {
    fn update(
        &self,
        source_id: TransceiverId,
        address: PointerAddress,
        update: DIFUpdateData,
    ) -> Result<(), DIFUpdateError> {
        let reference = self
            .resolve_in_memory_reference(&address)
            .ok_or(DIFUpdateError::ReferenceNotFound)?;
        match update {
            DIFUpdateData::Set { key, value } => {
                if !reference.supports_property_access() {
                    return Err(DIFUpdateError::AccessError(
                        AccessError::InvalidOperation(
                            "Reference does not support property access"
                                .to_string(),
                        ),
                    ));
                }
                let value_container = value.to_value_container(&self.memory)?;
                match key {
                    DIFProperty::Text(key) => reference.try_set_text_property(
                        source_id,
                        &key,
                        value_container,
                        &self.memory,
                    )?,
                    DIFProperty::Index(key) => reference
                        .try_set_numeric_property(
                            source_id,
                            key as u32,
                            value_container,
                            &self.memory,
                        )?,
                    DIFProperty::Value(key) => {
                        let key = key.to_value_container(&self.memory)?;
                        reference.try_set_property(
                            source_id,
                            key,
                            value_container,
                            &self.memory,
                        )?
                    }
                }
            }
            DIFUpdateData::Replace { value } => reference.try_set_value(
                source_id,
                value.to_value_container(&self.memory)?,
                &self.memory,
            )?,
            DIFUpdateData::Push { value } => {
                if !reference.supports_push() {
                    return Err(DIFUpdateError::AccessError(
                        AccessError::InvalidOperation(
                            "Reference does not support push operation"
                                .to_string(),
                        ),
                    ));
                }
                reference.try_push_value(
                    source_id,
                    value.to_value_container(&self.memory)?,
                    &self.memory,
                )?
            }
            DIFUpdateData::Clear => {
                if !reference.supports_clear() {
                    return Err(DIFUpdateError::AccessError(
                        AccessError::InvalidOperation(
                            "Reference does not support clear operation"
                                .to_string(),
                        ),
                    ));
                }
                reference.try_clear(source_id)?
            }
            DIFUpdateData::Remove { key } => {
                if !reference.supports_property_access() {
                    return Err(DIFUpdateError::AccessError(
                        AccessError::InvalidOperation(
                            "Reference does not support property access"
                                .to_string(),
                        ),
                    ));
                }

                match key {
                    DIFProperty::Text(key) => reference.try_delete_property(
                        source_id,
                        ValueContainer::from(key),
                        &self.memory,
                    )?,
                    DIFProperty::Index(key) => reference.try_delete_property(
                        source_id,
                        ValueContainer::from(key),
                        &self.memory,
                    )?,
                    DIFProperty::Value(key) => {
                        let key = key.to_value_container(&self.memory)?;
                        reference.try_delete_property(
                            source_id,
                            key,
                            &self.memory,
                        )?
                    }
                }
            }
        };

        Ok(())
    }

    async fn resolve_pointer_address_external(
        &self,
        address: PointerAddress,
    ) -> Result<DIFReference, DIFResolveReferenceError> {
        let reference = self.resolve_in_memory_reference(&address);
        match reference {
            Some(ptr) => Ok(DIFReference::from_reference(&ptr, &self.memory)),
            None => todo!("#399 Implement async resolution of references"),
        }
    }

    fn resolve_pointer_address_in_memory(
        &self,
        address: PointerAddress,
    ) -> Result<DIFReference, DIFResolveReferenceError> {
        let reference = self.resolve_in_memory_reference(&address);
        match reference {
            Some(ptr) => Ok(DIFReference::from_reference(&ptr, &self.memory)),
            None => Err(DIFResolveReferenceError::ReferenceNotFound),
        }
    }

    fn apply(
        &self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFValueContainer, DIFApplyError> {
        todo!("#400 Undescribed by author.")
    }

    fn create_pointer(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeContainer>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError> {
        let container = value.to_value_container(&self.memory)?;
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
        let address = self.memory.borrow_mut().register_reference(&reference);
        Ok(address)
    }

    fn observe_pointer<F: Fn(&DIFUpdate) + 'static>(
        &self,
        transceiver_id: TransceiverId,
        address: PointerAddress,
        options: ObserveOptions,
        callback: F,
    ) -> Result<u32, DIFObserveError> {
        let reference = self
            .resolve_in_memory_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        Ok(reference.observe(Observer {
            transceiver_id,
            options,
            callback: Rc::new(callback),
        })?)
    }

    fn update_observer_options(
        &self,
        address: PointerAddress,
        observer_id: u32,
        options: ObserveOptions,
    ) -> Result<(), DIFObserveError> {
        let reference = self
            .resolve_in_memory_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        reference
            .update_observer_options(observer_id, options)
            .map_err(DIFObserveError::ObserveError)
    }

    fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: u32,
    ) -> Result<(), DIFObserveError> {
        let reference = self
            .resolve_in_memory_reference(&address)
            .ok_or(DIFObserveError::ReferenceNotFound)?;
        reference
            .unobserve(observer_id)
            .map_err(DIFObserveError::ObserveError)
    }
}

#[cfg(test)]
mod tests {
    use crate::dif::interface::DIFInterface;
    use crate::dif::representation::DIFValueRepresentation;
    use crate::dif::update::{DIFUpdate, DIFUpdateData};
    use crate::dif::value::{DIFValue, DIFValueContainer};
    use crate::references::observers::ObserveOptions;
    use crate::references::reference::ReferenceMutability;
    use crate::runtime::Runtime;
    use crate::runtime::memory::Memory;
    use crate::values::core_values::endpoint::Endpoint;
    use crate::values::core_values::map::Map;
    use crate::values::value_container::ValueContainer;
    use datex_core::runtime::RuntimeConfig;
    use core::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn struct_serde() {
        let memory = RefCell::new(Memory::new(Endpoint::default()));
        let map = ValueContainer::from(Map::from(vec![
            ("a".to_string(), 1.into()),
            ("b".to_string(), "text".into()),
        ]));
        let dif_value = DIFValueContainer::from_value_container(&map, &memory);
        let _ = serde_json::to_string(&dif_value).unwrap();
    }

    #[test]
    fn test_create_and_observe_pointer() {
        let runtime = Runtime::init_native(RuntimeConfig::default()).internal;
        let pointer_address = runtime
            .create_pointer(
                DIFValueContainer::Value(DIFValue::from(
                    DIFValueRepresentation::String("Hello, world!".to_string()),
                )),
                None,
                ReferenceMutability::Mutable,
            )
            .expect("Failed to create pointer");

        let observed = Rc::new(RefCell::new(None));
        let observed_clone = observed.clone();

        let observer_id = Rc::new(RefCell::new(None));
        let observer_id_clone = observer_id.clone();
        let runtime_clone = runtime.clone();
        let pointer_address_clone = pointer_address.clone();

        // Observe the pointer
        observer_id.replace(Some(
            runtime
                .observe_pointer(
                    0,
                    pointer_address_clone.clone(),
                    ObserveOptions::default(),
                    move |update| {
                        println!("Observed pointer value: {:?}", update);
                        observed_clone.replace(Some(update.clone()));
                        // unobserve after first update
                        runtime_clone
                            .unobserve_pointer(
                                pointer_address_clone.clone(),
                                observer_id_clone.borrow().unwrap(),
                            )
                            .unwrap();
                    },
                )
                .expect("Failed to observe pointer"),
        ));

        // Update the pointer value
        runtime
            .update(
                1,
                pointer_address.clone(),
                DIFUpdateData::replace(DIFValue::from(
                    DIFValueRepresentation::String("Hello, Datex!".to_string()),
                )),
            )
            .expect("Failed to update pointer");

        // Check if the observed value matches the update
        let observed_value = observed.borrow();
        assert_eq!(
            *observed_value,
            Some(DIFUpdate {
                source_id: 1,
                data: DIFUpdateData::replace(DIFValue::from(
                    DIFValueRepresentation::String("Hello, Datex!".to_string(),)
                ))
            })
        );

        // try unobserve again, should fail
        assert!(
            runtime
                .unobserve_pointer(
                    pointer_address.clone(),
                    observer_id.borrow().unwrap()
                )
                .is_err()
        );
    }
}
