use crate::dif::interface::DIFResolveReferenceError;
use crate::dif::reference::DIFReference;
use crate::dif::update::{DIFKey, DIFUpdateData};
use crate::dif::value::DIFReferenceNotFoundError;
use crate::references::observers::{ObserveOptions, Observer, TransceiverId};
use crate::references::reference::ReferenceMutability;
use crate::runtime::RuntimeInternal;
use crate::stdlib::rc::Rc;
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
use core::prelude::rust_2024::*;
use core::result::Result;
use crate::dif::r#type::DIFTypeDefinition;
use crate::stdlib::vec::Vec;

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
        update: &DIFUpdateData,
    ) -> Result<(), DIFUpdateError> {
        let reference = self
            .resolve_in_memory_reference(&address)
            .ok_or(DIFUpdateError::ReferenceNotFound)?;
        match update {
            DIFUpdateData::Set { key, value } => {
                let value_container = value.to_value_container(&self.memory)?;
                match key {
                    DIFKey::Text(key) => reference.try_set_property(
                        source_id,
                        update,
                        key,
                        value_container,
                    )?,
                    DIFKey::Index(key) => reference.try_set_property(
                        source_id,
                        update,
                        *key,
                        value_container,
                    )?,
                    DIFKey::Value(key) => {
                        let key = key.to_value_container(&self.memory)?;
                        reference.try_set_property(
                            source_id,
                            update,
                            &key,
                            value_container,
                        )?
                    }
                }
            }
            DIFUpdateData::Replace { value } => reference.try_replace(
                source_id,
                update,
                value.to_value_container(&self.memory)?,
            )?,
            DIFUpdateData::Append { value } => reference.try_append_value(
                source_id,
                update,
                value.to_value_container(&self.memory)?,
            )?,
            DIFUpdateData::Clear => reference.try_clear(source_id)?,
            DIFUpdateData::Delete { key } => match key {
                DIFKey::Text(key) => {
                    reference.try_delete_property(source_id, update, key)?
                }
                DIFKey::Index(key) => {
                    reference.try_delete_property(source_id, update, *key)?
                }
                DIFKey::Value(key) => {
                    let key = key.to_value_container(&self.memory)?;
                    reference.try_delete_property(source_id, update, &key)?
                }
            },
            DIFUpdateData::ListSplice {
                start,
                delete_count,
                items,
            } => {
                reference.try_list_splice(
                    source_id,
                    update,
                    *start..(start + delete_count),
                    items
                        .iter()
                        .map(|item| item.to_value_container(&self.memory))
                        .collect::<Result<
                            Vec<ValueContainer>,
                            DIFReferenceNotFoundError,
                        >>()?,
                )?
            }
        };

        Ok(())
    }

    fn apply(
        &self,
        callee: DIFValueContainer,
        value: DIFValueContainer,
    ) -> Result<DIFValueContainer, DIFApplyError> {
        core::todo!("#400 Undescribed by author.")
    }

    fn create_pointer(
        &self,
        value: DIFValueContainer,
        allowed_type: Option<DIFTypeDefinition>,
        mutability: ReferenceMutability,
    ) -> Result<PointerAddress, DIFCreatePointerError> {
        let container = value.to_value_container(&self.memory)?;
        let type_container = if let Some(allowed_type) = &allowed_type {
            core::todo!(
                "FIXME: Implement type_container creation from DIFTypeDefinition"
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

    async fn resolve_pointer_address_external(
        &self,
        address: PointerAddress,
    ) -> Result<DIFReference, DIFResolveReferenceError> {
        let reference = self.resolve_in_memory_reference(&address);
        match reference {
            Some(ptr) => Ok(DIFReference::from_reference(&ptr, &self.memory)),
            None => {
                core::todo!("#399 Implement async resolution of references")
            }
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

    fn observe_pointer(
        &self,
        transceiver_id: TransceiverId,
        address: PointerAddress,
        options: ObserveOptions,
        callback: impl Fn(&DIFUpdateData, TransceiverId) + 'static,
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
    use crate::stdlib::rc::Rc;
    use crate::values::core_values::endpoint::Endpoint;
    use crate::values::core_values::map::Map;
    use crate::values::value_container::ValueContainer;
    use core::cell::RefCell;
    use datex_core::runtime::RuntimeConfig;

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
                    move |update, _| {
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
                &DIFUpdateData::replace(DIFValue::from(
                    DIFValueRepresentation::String("Hello, Datex!".to_string()),
                )),
            )
            .expect("Failed to update pointer");

        // Check if the observed value matches the update
        let observed_value = observed.borrow();
        assert_eq!(
            *observed_value,
            Some(DIFUpdateData::replace(DIFValue::from(
                DIFValueRepresentation::String("Hello, Datex!".to_string(),)
            )))
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
