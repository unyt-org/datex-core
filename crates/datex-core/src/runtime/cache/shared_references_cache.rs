use crate::{
    collections::{HashMap, HashSet},
    shared_values::{
        PointerAddress, ReferencedSharedContainer, SelfOwnedPointerAddress,
        SharedContainer, SharedContainerMutability,
        traits::SharedContainerCommon,
        weak_shared_container::WeakSharedContainer,
    },
    types::{
        entities::entity_type_definition::EntityTypeDefinition,
        entity_type::EntityType, r#type::Type,
    },
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::Value, value_container::ValueContainer,
    },
};
use core::{any::TypeId, fmt::Display, ops::Deref};

pub enum SharedTypeReservation {
    Existing(EntityType),
    New(EntityType),
}

#[derive(Debug, Default)]
pub struct SharedReferencesCache {
    /// References to owned values that are currently referenced and required by remote endpoints
    owned_values: HashMap<PointerAddress, ReferencedSharedContainer>,
    /// Weak references to remote values that this endpoint is currently subscribed to and receives updates for
    remote_values: HashMap<PointerAddress, WeakSharedContainer>,
    /// Set of TypeIds of structural types that are currently being resolved to prevent recursion
    resolving_structural_types: HashSet<TypeId>,
}

impl SharedReferencesCache {
    /// Registers a remote shared container in the cache.
    /// This method should be called for shared containers that this endpoint is currently subscribed to and receives updates for.
    /// If the reference is already registered (has a PointerAddress), the existing address is returned and no new registration is done.
    /// The reference is stored as a weak reference, so it will be dropped when there are no more strong references to it.
    /// TODO: call remove on Rc drop!
    pub fn register_remote_shared_container(
        &mut self,
        container: &ReferencedSharedContainer,
    ) {
        let pointer_address = container.pointer_address();
        // check if reference is already registered (if it has an address, we assume it is registered)
        self.remote_values
            .entry(pointer_address)
            .or_insert_with(|| container.downgrade());
    }

    /// Registers an owned shared container in the cache.
    /// This method should be called for shared containers that this endpoint owns and is responsible for.
    /// If the reference is already registered (has a PointerAddress), the existing address is returned and no new registration is done.
    /// The reference is stored as a strong reference, so it will not be dropped until it is explicitly removed from the cache.
    pub fn register_owned_shared_container(
        &mut self,
        container: &ReferencedSharedContainer,
    ) {
        let pointer_address = container.pointer_address();
        // check if reference is already registered (if it has an address, we assume it is registered)
        self.owned_values
            .entry(pointer_address)
            .or_insert_with(|| container.clone());
    }

    /// Removes a remote shared container from the cache.
    pub fn remove_remote_shared_container(
        &mut self,
        pointer_address: &PointerAddress,
    ) {
        self.remote_values.remove(pointer_address);
    }

    /// Removes an owned shared container from the cache.
    pub fn remove_owned_shared_container(
        &mut self,
        pointer_address: &PointerAddress,
    ) {
        self.owned_values.remove(pointer_address);
    }

    /// Returns a reference stored at the given PointerAddress, if it exists.
    pub fn get_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Option<ReferencedSharedContainer> {
        self.get_owned_reference(pointer_address)
            .cloned()
            .or_else(|| self.get_remote_reference(pointer_address))
    }

    /// Returns a reference to an owned shared container stored at the given PointerAddress, if it exists.
    pub fn get_owned_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Option<&ReferencedSharedContainer> {
        self.owned_values.get(pointer_address)
    }

    /// Returns a reference to a remote shared container stored at the given PointerAddress, if it exists and is still alive.
    pub fn get_remote_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Option<ReferencedSharedContainer> {
        self.remote_values
            .get(pointer_address)
            .and_then(|weak_ref| weak_ref.upgrade())
    }

    /// Checks if a reference with the given PointerAddress exists in memory.
    pub fn has_reference(&self, pointer_address: &PointerAddress) -> bool {
        self.owned_values.contains_key(pointer_address)
            || self
                .remote_values
                .get(pointer_address)
                .is_some_and(|weak_ref| weak_ref.upgrade().is_some())
    }

    /// Returns an iterator over all currently stored references in the cache, both owned and remote.
    pub fn values(&self) -> impl Iterator<Item = ReferencedSharedContainer> {
        gen move {
            for value in self.owned_values() {
                yield value.clone();
            }
            for value in self.remote_values() {
                yield value;
            }
        }
    }

    /// Returns an iterator over all currently stored owned references in the cache.
    pub fn owned_values(
        &self,
    ) -> impl Iterator<Item = &ReferencedSharedContainer> {
        self.owned_values.values()
    }

    /// Returns an iterator over all currently stored remote references in the cache.
    pub fn remote_values(
        &self,
    ) -> impl Iterator<Item = ReferencedSharedContainer> {
        self.remote_values
            .values()
            .filter_map(|weak_ref| weak_ref.upgrade())
    }

    /// Returns an iterator over all currently stored remote weak references in the cache.
    pub fn remote_values_weak(
        &self,
    ) -> impl Iterator<Item = &WeakSharedContainer> {
        self.remote_values.values()
    }

    /// Tries to get a shared container containing an EntityTypeDefinition from the cache for the given SelfOwnedPointerAddress.
    /// Returns the [EntityType] if found, or None if not found
    pub fn try_get_shared_type(
        &mut self,
        address: SelfOwnedPointerAddress,
    ) -> Option<EntityType> {
        // return existing type container stored in cache
        if let Some(value) = self
            .get_owned_reference(&PointerAddress::SelfOwned(address.clone()))
        {
            match value.value_container().deref() {
                ValueContainer::Local(Value::Core(
                    CoreValueWithClassification {
                        inner: CoreValue::EntityTypeDefinition(_),
                        ..
                    },
                )) => {}
                _ => {
                    panic!(
                        "Expected a shared container containing an EntityTypeDefinition, but found a different type for address: {}",
                        address
                    );
                }
            }
            // SAFETY: We have checked that the value is a SharedContainer containing an EntityTypeDefinition.
            unsafe {
                Some(EntityType::new_unchecked(SharedContainer::Referenced(
                    value.clone(),
                )))
            }
        } else {
            None
        }
    }

    /// Registers a new shared container type in the cache for the given address and returns the registered container as a [EntityType].
    ///
    /// # Safety
    /// The caller must ensure that the address is not used anywhere else.
    pub unsafe fn register_shared_type(
        &mut self,
        address: SelfOwnedPointerAddress,
        entity_type_definition: EntityTypeDefinition,
    ) -> EntityType {
        // create new shared container
        let shared_type_container = unsafe {
            EntityType::new_unchecked(
                SharedContainer::new_owned_with_inferred_allowed_type_unsafe(
                    CoreValue::EntityTypeDefinition(entity_type_definition),
                    SharedContainerMutability::Immutable,
                    address,
                ),
            )
        };

        // register shared container in cache
        self.register_owned_shared_container(
            &shared_type_container
                .clone()
                .to_shared_container()
                .derive_reference_with_max_mutability(),
        );
        shared_type_container
    }

    /// Reserves a shared container type in the cache for the given address and returns a [SharedTypeReservation].
    ///
    /// # Safety
    /// The caller must ensure that the address is unique.
    pub unsafe fn reserve_shared_type(
        &mut self,
        address: SelfOwnedPointerAddress,
    ) -> SharedTypeReservation {
        if let Some(existing) = self
            .get_owned_reference(&PointerAddress::SelfOwned(address.clone()))
        {
            return SharedTypeReservation::Existing(unsafe {
                EntityType::new_unchecked(SharedContainer::Referenced(
                    existing.clone(),
                ))
            });
        }
        let shared_container = unsafe {
            SharedContainer::new_owned_with_inferred_allowed_type_unsafe(
                CoreValue::Uninitialized,
                SharedContainerMutability::Immutable,
                address.clone(),
            )
        };
        self.register_owned_shared_container(
            &shared_container.derive_immutable_reference(),
        );
        // NOTE: this treats shared_container as if it already contains a type value.
        // So accessing it as a type before finish_shared_type is called will panic.
        SharedTypeReservation::New(unsafe {
            EntityType::new_unchecked(shared_container)
        })
    }

    /// Finishes the registration of a shared container type in the cache for the given address and definition.
    /// This method should be called after reserve_shared_type to set the actual EntityTypeDefinition for
    /// the reserved shared container. It will panic if the type is not in the cache or if it is already initialized.
    pub fn finish_shared_type(
        &mut self,
        address: SelfOwnedPointerAddress,
        definition: EntityTypeDefinition,
    ) {
        let ty = self
            .get_owned_reference(&PointerAddress::SelfOwned(address.clone()))
            .unwrap_or_else(|| panic!("Type is not in cache: {}", address));
        if !ty.value_container().is_uninitialized() {
            panic!("Type is already initialized: {}", address);
        }
        *ty.value_container_mut() = ValueContainer::Local(
            CoreValue::EntityTypeDefinition(definition).into(),
        );
    }

    /// Resolves a structural type for the given type T, ensuring that recursive types are not allowed.
    pub fn resolve_structural_type<T, F>(&mut self, f: F) -> Type
    where
        T: 'static,
        F: FnOnce(&mut Self) -> Type,
    {
        let id = TypeId::of::<T>();
        if !self.resolving_structural_types.insert(id) {
            // FIXME: allo recursive structural types
            panic!(
                "Can not use recursive structural DATEX types: {}",
                core::any::type_name::<T>(),
            );
        }
        let ty = f(self);
        self.resolving_structural_types.remove(&id);
        ty
    }

    /// Executes the given closure with a temporary entity boundary,
    /// which allows resolving structural types within the closure without affecting the outer context.
    pub fn with_entity_boundary<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let previous = core::mem::take(&mut self.resolving_structural_types);
        let result = f(self);
        self.resolving_structural_types = previous;
        result
    }
}

#[cfg(feature = "decompiler")]
impl Display for SharedReferencesCache {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use crate::decompiler::{ast_to_source_code::value_to_source_code, *};

        // print owned values
        writeln!(f, "Owned Values:")?;
        for (address, value) in &self.owned_values {
            writeln!(
                f,
                "  {}: {}",
                address,
                value_to_source_code(
                    &ValueContainer::Shared(SharedContainer::Referenced(
                        value.clone()
                    )),
                    DecompileOptions::default()
                )
            )?;
        }
        // print remote values
        writeln!(f, "Remote Values:")?;
        for (address, weak_value) in &self.remote_values {
            let value = weak_value.upgrade();
            if let Some(value) = value {
                writeln!(
                    f,
                    "  {}: {}",
                    address,
                    value_to_source_code(
                        &ValueContainer::Shared(SharedContainer::Referenced(
                            value
                        )),
                        DecompileOptions::default()
                    )
                )?;
            } else {
                writeln!(f, "  {}: <dropped>", address)?;
            }
        }
        Ok(())
    }
}
