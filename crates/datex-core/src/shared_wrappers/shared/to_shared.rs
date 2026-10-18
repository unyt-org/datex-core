use crate::{
    runtime::{
        cache::shared_references_cache::SharedReferencesCache,
        pointer_address_provider::SelfOwnedPointerAddressProvider,
    },
    shared_values::{SharedContainer, SharedContainerMutability},
    shared_wrappers::shared::shared_owned::Shared,
    values::core_values::native::DatexNative,
};
use crate::runtime::Runtime;
use crate::shared_values::OwnedSharedContainer;
use crate::shared_wrappers::shared::shared_ref::SharedRef;

pub trait ToShared: DatexNative + Sized {
    fn shared_with_cache_and_address_provider(
        self,
        cache: &mut SharedReferencesCache,
        provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Shared<Self> {
        Shared::try_from(OwnedSharedContainer::new_native(
            self,
            SharedContainerMutability::Mutable,
            provider,
            cache,
        ))
        .unwrap() // TODO: can we always unwrap safely here?
    }

    /// Creates a [Shared] value containing [Self]
    fn shared(
        self,
        runtime: &Runtime
    ) -> Shared<Self> {
        let mut cache = runtime.shared_references_cache_refcell().borrow_mut();
        let mut provider = runtime.pointer_address_provider_mut();
        self.shared_with_cache_and_address_provider(&mut cache, &mut provider)
    }
    
    fn shared_ref_with_cache_and_address_provider(
        self,
        cache: &mut SharedReferencesCache,
        provider: &mut SelfOwnedPointerAddressProvider,
    ) -> SharedRef<Self> {
        SharedRef::try_from(OwnedSharedContainer::new_native(
            self,
            SharedContainerMutability::Mutable,
            provider,
            cache,
        ))
        .unwrap() // TODO: can we always unwrap safely here?
    }
    
    /// Creates a [SharedRef] value containing [Self]
    fn shared_ref(
        self,
        runtime: &Runtime
    ) -> SharedRef<Self> {
        let mut cache = runtime.shared_references_cache_refcell().borrow_mut();
        let mut provider = runtime.pointer_address_provider_mut();
        self.shared_ref_with_cache_and_address_provider(&mut cache, &mut provider)
    }
}

/// Auto implementation of ToShared for all types that implement DatexNative and are Sized.
impl<T> ToShared for T where T: DatexNative + Sized {}
