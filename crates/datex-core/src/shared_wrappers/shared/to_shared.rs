use crate::{
    runtime::{
        cache::shared_references_cache::SharedReferencesCache,
        pointer_address_provider::SelfOwnedPointerAddressProvider,
    },
    shared_values::{SharedContainer, SharedContainerMutability},
    shared_wrappers::shared::Shared,
    values::core_values::native::DatexNative,
};

pub trait ToShared: DatexNative + Sized {
    fn shared(
        self,
        _cache: &mut SharedReferencesCache,
        provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Shared<Self> {
        Shared::try_from(SharedContainer::new_owned_with_inferred_allowed_type(
            self.to_value_container(),
            SharedContainerMutability::Mutable,
            provider,
        ))
        .unwrap() // TODO: can we always unwrap safely here?
    }
}

/// Auto implementation of ToShared for all types that implement DatexNative and are Sized.
impl<T> ToShared for T where T: DatexNative + Sized {}
