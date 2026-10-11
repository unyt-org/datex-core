use crate::{
    dif::error::{DIFObserveError, DIFUpdateError},
    prelude::*,
    runtime::{
        Runtime,
        cache::shared_values_cache::{
            SharedValuesCache, ValueNotFoundInCacheError,
        },
        pointer_address_provider::SelfOwnedPointerAddressProvider,
    },
    shared_values::{
        OwnedSharedContainer, PointerAddress, SelfOwnedPointerAddress,
        SelfOwnedSharedContainer, SharedContainer, SharedContainerMutability,
        SharedContainerOwnership,
        base_shared_value_container::{
            BaseSharedValueContainer,
            observers::{
                ObserveOptions, Observer, ObserverCallback, ObserverId,
                TransceiverId,
            },
        },
        traits::SharedContainerCommon,
    },
    traits::apply::{Apply, ApplyArgument, ApplyError},
    types::type_definition::{
        TypeDefinition, callable::CallableTypeDefinition,
    },
    value_updates::{UpdateReturn, update_data::Update},
    values::{
        core_values::callable::{Callable, CallableBody, NativeCallable},
        value_container::ValueContainer,
    },
};
use alloc::rc::Rc;
use core::{cell::RefCell, result::Result};

pub type DIFUpdateResult = Result<UpdateReturn, DIFUpdateError>;

pub struct DIFInterface {
    pub cache: RefCell<SharedValuesCache>,
    address_provider: Rc<RefCell<SelfOwnedPointerAddressProvider>>,
    transceiver_id: TransceiverId,
}

impl DIFInterface {
    pub fn new(
        transceiver_id: TransceiverId,
        address_provider: Rc<RefCell<SelfOwnedPointerAddressProvider>>,
    ) -> Self {
        DIFInterface {
            cache: RefCell::new(SharedValuesCache::default()),
            address_provider,
            transceiver_id,
        }
    }
}
impl DIFInterface {
    /// Returns a list of all [ObserverCallback]s that are currently active for the pointer
    pub fn get_current_observers(
        &self,
        address: &PointerAddress,
        source_id: &TransceiverId,
    ) -> Result<Vec<ObserverCallback>, ValueNotFoundInCacheError> {
        let shared_container = self
            .cache
            .borrow()
            .try_get_shared_container_immutable_reference(address)?;
        Ok(shared_container.get_current_observers(source_id))
    }

    /// Registers a native callable function in the DIFInterface as a shared value and returns its pointer address.
    pub fn register_callable(
        &mut self,
        callable: NativeCallable,
        name: Option<String>,
        signature: CallableTypeDefinition,
    ) -> SelfOwnedPointerAddress {
        let callable = Callable {
            name,
            signature: signature.clone(),
            body: CallableBody::Native(callable),
            creator: Default::default(),
        };
        let shared_base = BaseSharedValueContainer::try_new(
            ValueContainer::from(callable),
            TypeDefinition::Callable(signature),
            SharedContainerMutability::Immutable,
        )
        .unwrap();
        self.create_pointer(shared_base)
    }

    /// Executes an apply operation, applying the `value` to the `callee`.
    pub fn apply_sync(
        &self,
        runtime: &Runtime,
        callee: ValueContainer,
        value: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        callee.try_apply_sync_checked(runtime, value)
    }

    /// Executes an async apply operation, applying the `value` to the `callee`.
    pub async fn apply_async(
        &self,
        runtime: &Runtime,
        callee: ValueContainer,
        value: Vec<ApplyArgument>,
    ) -> Result<(Option<ValueContainer>, Vec<ValueContainer>), ApplyError> {
        callee.try_apply_async_checked(runtime, value).await
    }

    /// Creates a new owned local pointer and stores it in memory.
    /// Returns the [SelfOwnedPointerAddress] of the newly created pointer.
    pub fn create_pointer(
        &mut self,
        value: BaseSharedValueContainer,
    ) -> SelfOwnedPointerAddress {
        let container = OwnedSharedContainer::new_from_self_owned_container(
            SelfOwnedSharedContainer::new(
                value,
                &mut self.address_provider.borrow_mut(),
            ),
        );

        let pointer_address = container.pointer_address().clone();
        self.cache
            .borrow_mut()
            .store_shared_container(SharedContainer::Owned(container));
        pointer_address
    }

    /// Resolves a pointer address of a pointer that is currently in memory to its [SharedContainer].
    /// Returns an error if the pointer is not found in memory.
    pub fn resolve_pointer_address(
        &mut self,
        address: PointerAddress,
    ) -> Result<SharedContainer, ValueNotFoundInCacheError> {
        self.cache
            .borrow()
            .try_get_shared_container(&address)
            .cloned()
    }

    pub fn has_address_with_ownership(
        &self,
        pointer_address: &PointerAddress,
        ownership: SharedContainerOwnership,
    ) -> bool {
        self.cache
            .borrow()
            .has_address_with_ownership(pointer_address, ownership)
    }

    /// Starts observing changes to the pointer at the given address.
    /// As long as the pointer is observed, it will not be garbage collected.
    pub fn observe_pointer(
        &self,
        address: PointerAddress,
        options: ObserveOptions,
        callback: impl Fn(&Update) + 'static,
    ) -> Result<ObserverId, DIFObserveError> {
        let cache = self.cache.borrow_mut();
        let shared_container_ref = cache
            .try_get_shared_container(&address)
            .map_err(|_| DIFObserveError::ReferenceNotFound)?;
        Ok(shared_container_ref.observe(Observer {
            transceiver_id: self.transceiver_id.clone(),
            options,
            callback: Rc::new(callback),
        })?)
    }

    /// Updates the options for an existing observer on the pointer at the given address.
    /// If the observer does not exist, an error is returned.
    pub fn update_observer_options(
        &self,
        address: PointerAddress,
        observer_id: ObserverId,
        options: ObserveOptions,
    ) -> Result<(), DIFObserveError> {
        let cache = self.cache.borrow_mut();
        let shared_container_ref = cache
            .try_get_shared_container(&address)
            .map_err(|_| DIFObserveError::ReferenceNotFound)?;
        shared_container_ref.update_observer_options(observer_id, options)?;
        Ok(())
    }

    /// Stops observing changes to the pointer at the given address.
    /// If no other references to the pointer exist, it may be garbage collected after this call.
    pub fn unobserve_pointer(
        &self,
        address: PointerAddress,
        observer_id: ObserverId,
    ) -> Result<(), DIFObserveError> {
        let cache = self.cache.borrow_mut();
        let shared_container_ref = cache
            .try_get_shared_container(&address)
            .map_err(|_| DIFObserveError::ReferenceNotFound)?;
        shared_container_ref.unobserve(observer_id)?;
        Ok(())
    }

    // TODO: lock/unlock pointers
}
