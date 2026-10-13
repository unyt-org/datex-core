use crate::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        pointer_address::PointerAddressWithOwnership,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    prelude::*,
    shared_values::{
        ReferenceMutability, SharedContainer, SharedContainerOwnership,
        traits::SharedContainerCommon,
    },
    traits::clone_unsafe::CloneUnsafe,
};
use alloc::format;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::runtime::cache::shared_values_cache::CacheValueRetrievalError;

impl SharedContainer {
    pub fn resolve_with_serde_context(
        pointer_address_with_ownership: PointerAddressWithOwnership,
        ctx: &SerdeContext<'_>,
    ) -> Result<Self, CacheValueRetrievalError> {
        let PointerAddressWithOwnership { address, ownership } =
            pointer_address_with_ownership;
        ctx.shared_container_cache
            .borrow_mut()
            .try_get_shared_container_with_ownership(&address, ownership)
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for SharedContainer {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        d: D,
    ) -> Result<SharedContainer, D::Error> {
        let pointer_address_with_ownership =
            PointerAddressWithOwnership::deserialize(d)?;
        SharedContainer::resolve_with_serde_context(pointer_address_with_ownership, ctx)
            .map_err(|e| {
                serde::de::Error::custom(format!(
                    "Failed to retrieve shared container from cache: {}",
                    e
                ))
            })
    }
}
impl<'ctx> SerdeContext<'ctx> {
    pub unsafe fn pointer_string(&self, value: &SharedContainer) -> String {
        unsafe {
            self.shared_container_cache
                .borrow_mut()
                .store_shared_container(value.clone_unsafe());
        }

        let ownership = match value.ownership() {
            SharedContainerOwnership::Referenced(
                ReferenceMutability::Immutable,
            ) => "'",

            SharedContainerOwnership::Referenced(
                ReferenceMutability::Mutable,
            ) => "'mut",

            SharedContainerOwnership::Owned => "",
        };

        format!("{}{}", ownership, value.pointer_address())
    }
}
impl SerializeWithSerdeContext for SharedContainer {
    /// SAFETY:
    /// The caller of the `serialize` method must either
    /// * guarantee that no direct value (accessible without borrow) is an owned shared value
    ///   (this can be guaranteed by calling clone on the top level value before passing it to [SerializeWithSerdeContext])
    /// * or guarantee that the value is dropped after calling `serialize`, so that the owned shared value
    ///   is not leaked after serialization.
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unsafe { ctx.pointer_string(self).serialize(serializer) }
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        prelude::*,
        runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
        shared_values::{
            OwnedSharedContainer, PointerAddress, ReferenceMutability,
            SharedContainer, SharedContainerMutability,
            SharedContainerOwnership,
            errors::UnexpectedSharedContainerOwnershipError,
        },
        values::value_container::ValueContainer,
    };

    #[test]
    fn serialize_shared_container_reference() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let owned_shared_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                ValueContainer::from(42),
                SharedContainerMutability::Immutable,
                &mut SelfOwnedPointerAddressProvider::default(),
            )
            .derive_immutable_reference();
        let address = owned_shared_container.pointer_address();

        let serialized = context.serialize_to_json(
            &SharedContainer::Referenced(owned_shared_container),
        );
        assert_eq!(serialized, format!(r#""'{}""#, address.to_string()));
    }

    #[test]
    fn serialize_shared_owned_container() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let owned_container =
            SharedContainer::new_owned_with_inferred_allowed_type(
                ValueContainer::from(42),
                SharedContainerMutability::Mutable,
                address_provider,
            );

        let serialized = context.serialize_to_json(&owned_container);
        assert_eq!(
            serialized,
            format!(r#""{}""#, owned_container.pointer_address().to_string())
        );
    }

    use crate::{
        dif::serde_context::SerdeContext,
        runtime::cache::{
            shared_references_cache::SharedReferencesCache,
            shared_values_cache::{
                CacheValueRetrievalError, SharedValuesCache,
            },
        },
    };
    use core::{assert_matches, cell::RefCell};

    #[test]
    fn deserialize_owned_pointer_address_to_shared_container() {
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let owned_shared_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                ValueContainer::from(42),
                SharedContainerMutability::Immutable,
                &mut SelfOwnedPointerAddressProvider::default(),
            );
        let pointer_address = owned_shared_container.pointer_address().clone();
        cache
            .borrow_mut()
            .store_shared_container(SharedContainer::Owned(
                owned_shared_container,
            ));

        let outer = context
            .try_deserialize_from_json(
                format!(r#""{}""#, pointer_address).as_str(),
            )
            .unwrap();
        if let SharedContainer::Owned(retrieved) = &outer {
            assert_eq!(*retrieved.pointer_address(), pointer_address);
        } else {
            panic!("Expected owned shared container");
        }
    }

    #[test]
    fn deserialize_pointer_address_to_shared_container() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();
        let cache = RefCell::new(SharedValuesCache::default());
        let mut context = SerdeContext::new(&cache);

        let owned_container =
            SharedContainer::new_owned_with_inferred_allowed_type(
                ValueContainer::from(42),
                SharedContainerMutability::Mutable,
                address_provider,
            );
        let ptr_address = owned_container.pointer_address();
        let ptr_address_hex = ptr_address.to_string();

        cache.borrow_mut().store_shared_container(owned_container);

        let outer_ref = context
            .try_deserialize_from_json(&format!(r#""'{}""#, ptr_address_hex))
            .unwrap();

        assert_matches!(
            outer_ref,
            SharedContainer::Referenced(reference)
            if reference.reference_mutability() == ReferenceMutability::Immutable &&
                reference.pointer_address() == ptr_address
        );

        let outer_ref_mut = context
            .try_deserialize_from_json(&format!(r#""'mut{}""#, ptr_address_hex))
            .unwrap();

        assert_matches!(
            outer_ref_mut,
            SharedContainer::Referenced(reference)
            if reference.reference_mutability() == ReferenceMutability::Mutable &&
                reference.pointer_address() == ptr_address
        );

        let outer_owned = context
            .try_deserialize_from_json(&format!(r#""{}""#, ptr_address_hex))
            .unwrap();

        assert_matches!(
            outer_owned,
            SharedContainer::Owned(owned)
            if PointerAddress::SelfOwned(owned.pointer_address().clone()) == ptr_address
        );

        // should no longer exist in memory as owned container should have been taken from cache
        assert_matches!(
            cache
                .borrow_mut()
                .try_take_owned_shared_container(&ptr_address),
            Err(
                CacheValueRetrievalError::UnexpectedSharedContainerOwnership(
                    UnexpectedSharedContainerOwnershipError {
                        actual: SharedContainerOwnership::Referenced(
                            ReferenceMutability::Mutable
                        ),
                        expected: SharedContainerOwnership::Owned
                    }
                )
            )
        );

        // should no longer exist in memory at all after explicitly removing the shared container from cache
        cache.borrow_mut().remove_shared_container(&ptr_address);

        assert_matches!(
            cache
                .borrow_mut()
                .try_take_owned_shared_container(&ptr_address),
            Err(CacheValueRetrievalError::ValueNotFoundInCache(_))
        );
    }
}
