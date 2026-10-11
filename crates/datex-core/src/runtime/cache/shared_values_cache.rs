use crate::{
    collections::HashMap,
    prelude::*,
    shared_values::{
        OwnedSharedContainer, PointerAddress, ReferenceMutability,
        ReferencedSharedContainer, SharedContainer, SharedContainerOwnership,
        errors::{
            UnexpectedImmutableReferenceError,
            UnexpectedSharedContainerOwnershipError,
        },
        traits::SharedContainerCommon,
    },
    values::core_values::callable::Callable,
};
use core::{
    fmt::Display,
    hash::{Hash, Hasher},
};

/// Cache layer that stores references or owned and referenced shared containers
#[derive(Debug, Default)]
pub struct SharedValuesCache {
    values: HashMap<PointerAddress, SharedContainer>,
    callables: HashMap<u64, Callable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueNotFoundInCacheError(pub PointerAddress);

impl Display for ValueNotFoundInCacheError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Value not found in cache: {}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheValueRetrievalError {
    UnexpectedImmutableReference(UnexpectedImmutableReferenceError),
    UnexpectedSharedContainerOwnership(UnexpectedSharedContainerOwnershipError),
    ValueNotFoundInCache(ValueNotFoundInCacheError),
}

impl Display for CacheValueRetrievalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CacheValueRetrievalError::UnexpectedImmutableReference(err) => {
                write!(f, "Unexpected immutable reference: {}", err)
            }
            CacheValueRetrievalError::UnexpectedSharedContainerOwnership(
                err,
            ) => {
                write!(f, "Unexpected shared container ownership: {}", err)
            }
            CacheValueRetrievalError::ValueNotFoundInCache(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

impl From<UnexpectedImmutableReferenceError> for CacheValueRetrievalError {
    fn from(error: UnexpectedImmutableReferenceError) -> Self {
        CacheValueRetrievalError::UnexpectedImmutableReference(error)
    }
}

impl From<ValueNotFoundInCacheError> for CacheValueRetrievalError {
    fn from(error: ValueNotFoundInCacheError) -> Self {
        CacheValueRetrievalError::ValueNotFoundInCache(error)
    }
}

impl From<UnexpectedSharedContainerOwnershipError>
    for CacheValueRetrievalError
{
    fn from(error: UnexpectedSharedContainerOwnershipError) -> Self {
        CacheValueRetrievalError::UnexpectedSharedContainerOwnership(error)
    }
}

impl SharedValuesCache {
    /// Creates a new SharedValuesCache and populates it with the provided shared containers.
    /// Assumes that all provided containers have unique pointer addresses.
    pub fn new(values: Vec<SharedContainer>) -> Self {
        SharedValuesCache {
            values: values
                .into_iter()
                .map(|container| (container.pointer_address(), container))
                .collect(),
            callables: HashMap::new(),
        }
    }

    /// Stores a shared container in the cache, indexed by its pointer address.
    /// If a container already exists at the address, the container with the maximum ownership is kept.
    pub fn store_shared_container(&mut self, container: SharedContainer) {
        if let Some(existing) = self.values.get(&container.pointer_address())
            && existing.ownership() > container.ownership()
        {
            return;
        }
        self.values.insert(container.pointer_address(), container);
    }

    /// Stores a callable in the cache and returns its hash value.
    /// TODO: use different hash and HashMap<hash, callable> to reduce collisions?
    pub fn store_callable(&mut self, callable: Callable) -> u64 {
        let mut hasher = default_hasher();
        callable.hash(&mut hasher);
        let hash = hasher.finish();
        self.callables.insert(hash, callable);
        hash
    }

    /// Retrieves a callable from the cache based on its hash value.
    pub fn get_callable(&self, hash: u64) -> Option<Callable> {
        self.callables.get(&hash).cloned()
    }

    /// Removes a callable from the cache based on its hash value.
    pub fn remove_callable_with_hash(&mut self, hash: u64) {
        self.callables.remove(&hash);
    }

    /// Removes the shared container for the given pointer address from the cache, if it exists.
    pub fn remove_shared_container(
        &mut self,
        pointer_address: &PointerAddress,
    ) {
        self.values.remove(pointer_address);
    }

    /// Tries to take an owned shared container from the cache for the given pointer address.
    /// If the container for the address is not an owned shared container, or if there is no container for the address, None is returned and the cache is not modified.
    /// If an owned shared container is found for the address, it is replaced with a reference in the cache and the owned container is returned
    pub fn try_take_owned_shared_container(
        &mut self,
        pointer_address: &PointerAddress,
    ) -> Result<OwnedSharedContainer, CacheValueRetrievalError> {
        match self.values.get(pointer_address) {
            Some(container) => {
                match container {
                    SharedContainer::Owned(container) => {
                        // replace owned with reference in cache and return owned
                        match self.values.insert(
                            pointer_address.clone(),
                            SharedContainer::Referenced(
                                container.derive_with_max_mutability(),
                            ),
                        ) {
                            Some(SharedContainer::Owned(owned_container)) => {
                                Ok(owned_container)
                            }
                            _ => unreachable!(),
                        }
                    }
                    SharedContainer::Referenced(reference) => {
                        Err(UnexpectedSharedContainerOwnershipError {
                            actual: SharedContainerOwnership::Referenced(
                                reference.reference_mutability(),
                            ),
                            expected: SharedContainerOwnership::Owned,
                        }
                        .into())
                    }
                }
            }
            _ => Err(ValueNotFoundInCacheError(pointer_address.clone()).into()),
        }
    }

    pub fn try_get_shared_container(
        &self,
        pointer_address: &PointerAddress,
    ) -> Result<&SharedContainer, ValueNotFoundInCacheError> {
        self.values
            .get(pointer_address)
            .ok_or(ValueNotFoundInCacheError(pointer_address.clone()))
    }

    /// Tries to get a mutable reference to a shared container from the cache at the given pointer address.
    /// If the container for the address is not in the cache, a [ValueNotFoundInCacheError] is returned.
    pub fn try_get_shared_container_reference(
        &self,
        pointer_address: &PointerAddress,
        mutability: ReferenceMutability,
    ) -> Result<ReferencedSharedContainer, CacheValueRetrievalError> {
        match mutability {
            ReferenceMutability::Mutable => {
                self.try_get_shared_container_mutable_reference(pointer_address)
            }
            ReferenceMutability::Immutable => Ok(self
                .try_get_shared_container_immutable_reference(
                    pointer_address,
                )?),
        }
    }

    /// Tries to get a mutable reference to a shared container from the cache at the given pointer address.
    /// If the container for the address is not in the cache, a [ValueNotFoundInCacheError] is returned.
    /// If the container cannot be accessed as a mutable reference, an [UnexpectedImmutableReferenceError] error is returned.
    pub fn try_get_shared_container_mutable_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Result<ReferencedSharedContainer, CacheValueRetrievalError> {
        Ok(self
            .values
            .get(pointer_address)
            .ok_or(ValueNotFoundInCacheError(pointer_address.clone()))?
            .try_derive_mutable_reference()?)
    }

    /// Tries to get an immutable reference to a shared container from the cache at the given pointer address.
    /// If the container for the address is not in the cache, a [ValueNotFoundInCacheError] is returned.
    pub fn try_get_shared_container_immutable_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Result<ReferencedSharedContainer, ValueNotFoundInCacheError> {
        Ok(self
            .values
            .get(pointer_address)
            .ok_or(ValueNotFoundInCacheError(pointer_address.clone()))?
            .derive_immutable_reference())
    }

    /// Tries to get a shared container from the cache at the given pointer address with the specified ownership.
    /// If the container for the address is not in the cache, or if the container cannot be accessed with the specified ownership, a [CacheValueRetrievalError] is returned.
    pub fn try_get_shared_container_with_ownership(
        &mut self,
        pointer_address: &PointerAddress,
        ownership: SharedContainerOwnership,
    ) -> Result<SharedContainer, CacheValueRetrievalError> {
        match ownership {
            SharedContainerOwnership::Owned => self
                .try_take_owned_shared_container(pointer_address)
                .map(SharedContainer::Owned),
            SharedContainerOwnership::Referenced(
                ReferenceMutability::Mutable,
            ) => self
                .try_get_shared_container_mutable_reference(pointer_address)
                .map(SharedContainer::Referenced),
            SharedContainerOwnership::Referenced(
                ReferenceMutability::Immutable,
            ) => Ok(self
                .try_get_shared_container_immutable_reference(pointer_address)
                .map(SharedContainer::Referenced)?),
        }
    }
    pub fn has_address_with_ownership(
        &self,
        pointer_address: &PointerAddress,
        ownership: SharedContainerOwnership,
    ) -> bool {
        self.values
            .get(pointer_address)
            .is_some_and(|container| container.ownership() >= ownership)
    }
}
