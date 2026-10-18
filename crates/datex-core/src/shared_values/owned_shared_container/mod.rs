mod clone_unsafe;
mod common;
mod convert_value_container;
pub mod get_datex_type;
pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
pub mod convert_parts;

use crate::{
    prelude::*,
    runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
    shared_values::{
        ReferencedSharedContainer, RemotePointerAddress,
        SelfOwnedPointerAddress, SelfOwnedSharedContainer,
        SharedContainerInner, SharedContainerMutability,
        base_shared_value_container::{
            BaseSharedValueContainer, observers::ObserverData,
        },
        errors::UnexpectedImmutableSharedContainerError,
        traits::{_ExposeRcInternal, SharedContainerCommon},
    },
    traits::{
        identity::Identity, structural_eq::StructuralEq, value_eq::ValueEq,
    },
    utils::impl_display_for_datex_value::impl_display_for_datex_value,
    values::value_container::ValueContainer,
};
use alloc::rc::Rc;
use core::{
    cell::{Ref, RefCell, RefMut},
    fmt::Display,
    hash::{Hash, Hasher},
    mem,
};
use crate::runtime::cache::shared_references_cache::SharedReferencesCache;
use crate::shared_values::SharedContainer;
use crate::values::core_values::native::DatexNative;

/// Wrapper struct for an owned shared value (i.e. `shared X`)
/// It is guaranteed that the inner value is a [SharedContainerInner::EndpointOwned].
///
/// ([OwnedSharedContainer] implies [SharedContainerInner::EndpointOwned], but not vice versa,
/// since a [SharedContainerInner::EndpointOwned] can be wrapped in a [ReferencedSharedContainer])
///
/// When holding an [OwnedSharedContainer], it is guaranteed that the contained [SharedContainerInner] is
/// not moved and changed to [SharedContainerInner::External].
/// Only an [OwnedSharedContainer] can be moved to another endpoint or location.
#[derive(Debug)]
pub struct OwnedSharedContainer {
    /// It is guaranteed that the inner value is a [SharedContainerInner::EndpointOwned].
    inner: Rc<RefCell<SharedContainerInner>>,
    /// This reflects the container mutability of the inner container, which is guaranteed to stay the same
    container_mutability: SharedContainerMutability,
    /// Observer data (e.g. observer list) for this shared container. Can be borrowed separately from the [SharedContainerInner]
    observer_data: Rc<RefCell<ObserverData>>,
    /// Flag indicating that the inner value is uninitialized. This must get reset to false when the inner value is updated.
    /// When set to true, an update to the value is allowed even if the reference is immutable, since the value is not yet initialized.
    /// TODO: better way to handle uninitialized values, maybe don't store on top level, but this is more efficient since we don't have to deref the inner value
    pub(super) is_uninitialized: bool,
}

impl OwnedSharedContainer {
    /// Creates a new owned container from an [SelfOwnedSharedContainer]
    pub fn new_from_self_owned_container(
        container: SelfOwnedSharedContainer,
    ) -> Self {
        let container_mutability = *container.value().mutability();
        OwnedSharedContainer {
            inner: Rc::new(RefCell::new(SharedContainerInner::EndpointOwned(
                container,
            ))),
            container_mutability,
            observer_data: Rc::new(RefCell::new(ObserverData::default())),
            is_uninitialized: false,
        }
    }

    /// Creates a new [OwnedSharedContainer] with an initial [ValueContainer],
    /// a [SharedContainerMutability], and an [SelfOwnedPointerAddress].
    ///
    /// The allowed type is inferred from the value_container's allowed type.
    pub fn new_with_inferred_allowed_type<T: Into<ValueContainer>>(
        value_container: T,
        mutability: SharedContainerMutability,
        address_provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Self {
        // Note: address provider guarantees new unique address
        unsafe {
            Self::new_with_inferred_allowed_type_unsafe(
                value_container.into(),
                mutability,
                address_provider.get_new_self_owned_address(),
            )
        }
    }


    /// Creates a new [OwnedSharedContainer] containing an initial [DatexNative] value.
    /// The allowed type is the type of the [DatexNative].
    pub fn new_native<T: DatexNative>(
        value: T,
        mutability: SharedContainerMutability,
        address_provider: &mut SelfOwnedPointerAddressProvider,
        cache: &mut SharedReferencesCache
    ) -> Self {
        let ty = value.allowed_type(cache);

        let value_container = value.to_value_container();
        let is_uninitialized = value_container.is_uninitialized();

        // SAFETY: address provider guarantees new unique address
        unsafe {
            Self::new_from_base_shared_container_unsafe(
                BaseSharedValueContainer::try_new(
                    value_container,
                    ty,
                    mutability,
                ).unwrap(), // the unwrap is safe since the type of the native value is guaranteed to match the value
                address_provider.get_new_self_owned_address(),
                is_uninitialized,
            )
        }
    }


    /// Creates a new [OwnedSharedContainer] with an initial [ValueContainer],
    /// a [SharedContainerMutability], and an [SelfOwnedPointerAddress].
    ///
    /// The allowed type is inferred from the value_container's allowed type.
    /// # Safety
    /// The caller must ensure that the address is not used anywhere else.
    pub unsafe fn new_with_inferred_allowed_type_unsafe(
        value_container: ValueContainer,
        mutability: SharedContainerMutability,
        address: SelfOwnedPointerAddress,
    ) -> Self {
        let is_uninitialized = value_container.is_uninitialized();
        unsafe {
            Self::new_from_base_shared_container_unsafe(
                BaseSharedValueContainer::new_with_inferred_allowed_type(
                    value_container,
                    mutability,
                ),
                address,
                is_uninitialized,
            )
        }
    }

    /// Creates a new [OwnedSharedContainer] from a [BaseSharedValueContainer] and a [SelfOwnedPointerAddress].
    /// # Safety
    /// The caller must ensure that the address is not used anywhere else.
    unsafe fn new_from_base_shared_container_unsafe(
        base_shared_container: BaseSharedValueContainer,
        address: SelfOwnedPointerAddress,
        is_uninitialized: bool,
    ) -> Self {
        let mut container =
            OwnedSharedContainer::new_from_self_owned_container(unsafe {
                SelfOwnedSharedContainer::new_with_address(
                    base_shared_container,
                    address,
                )
            });

        if is_uninitialized {
            container.mark_uninitialized();
        }

        container
    }

    /// Creates a new [OwnedSharedContainer] from parts.
    ///
    /// # Safety
    /// This function should only be called if you can guarantee that
    /// no other [OwnedSharedContainer] exist that are using the same inner value.
    pub unsafe fn new_unchecked(
        container_mutability: SharedContainerMutability,
        inner: Rc<RefCell<SharedContainerInner>>,
        observer_data: Rc<RefCell<ObserverData>>,
    ) -> Self {
        OwnedSharedContainer {
            inner,
            container_mutability,
            observer_data,
            is_uninitialized: false,
        }
    }

    /// Get a [Ref] to the inner [SelfOwnedSharedContainer].
    /// It is guaranteed that the contained [SharedContainerInner] is always a [SharedContainerInner::EndpointOwned].
    pub fn as_self_owned_shared_container(
        &self,
    ) -> Ref<'_, SelfOwnedSharedContainer> {
        Ref::map(self.inner(), |inner| match inner {
            SharedContainerInner::EndpointOwned(inner) => inner,
            _ => unreachable!(
                "OwnedSharedContainer must contain an EndpointOwned inner value"
            ),
        })
    }

    /// Get a [RefMut] to the inner [SelfOwnedSharedContainer].
    /// It is guaranteed that the contained [SharedContainerInner] is always a [SharedContainerInner::EndpointOwned].
    pub fn as_self_owned_shared_container_mut(
        &self,
    ) -> RefMut<'_, SelfOwnedSharedContainer> {
        RefMut::map(self.inner_mut(), |inner| match inner {
            SharedContainerInner::EndpointOwned(inner) => inner,
            _ => unreachable!(
                "OwnedSharedContainer must contain an EndpointOwned inner value"
            ),
        })
    }

    /// Get a [Ref] to the inner [SelfOwnedPointerAddress].
    /// It is guaranteed that the pointer address is always a [SelfOwnedPointerAddress].
    pub fn pointer_address(&self) -> Ref<'_, SelfOwnedPointerAddress> {
        Ref::map(self.as_self_owned_shared_container(), |inner| {
            inner.address()
        })
    }

    /// Creates a new immutable [ReferencedSharedContainer] pointing to the same inner value as this [OwnedSharedContainer].
    pub fn derive_immutable_reference(&self) -> ReferencedSharedContainer {
        ReferencedSharedContainer::new_immutable(
            self.inner.clone(),
            self.observer_data.clone(),
        )
    }

    /// Tries to create a new mutable [ReferencedSharedContainer] pointing to the same inner value as this [OwnedSharedContainer].
    /// Returns an [Err] if the container itself is not mutable
    pub fn try_derive_mutable_reference(
        &self,
    ) -> Result<
        ReferencedSharedContainer,
        UnexpectedImmutableSharedContainerError,
    > {
        if self.container_mutability() != SharedContainerMutability::Mutable {
            return Err(UnexpectedImmutableSharedContainerError);
        }

        // new_mutable_unchecked is safe to call here since we checked the container mutability before
        Ok(ReferencedSharedContainer::new_mutable_unchecked(
            self.inner.clone(),
            self.observer_data.clone(),
        ))
    }

    /// Clones the shared container as a mutable reference if possible, otherwise as an immutable reference
    pub fn derive_with_max_mutability(&self) -> ReferencedSharedContainer {
        self.try_derive_mutable_reference()
            .unwrap_or_else(|_| self.derive_immutable_reference())
    }

    /// Clones the shared container as a mutable reference if possible,
    /// otherwise as an immutable reference, and sets the move indicator on the cloned reference
    pub fn clone_with_move_indicator(&self) -> ReferencedSharedContainer {
        let mut reference = self.derive_with_max_mutability();
        unsafe {
            // SAFETY: since this is an OwnedSharedContainer, it always has an owned address
            reference.set_move_indicator();
        }
        reference
    }

    /// Moves an owned shared container by converting it to a [ReferencedSharedContainer] with a [RemotePointerAddress] pointing to the given remote address.
    /// Drops the original owned shared container
    /// # Safety
    /// The caller must ensure that the [RemotePointerAddress] does not yet exist in the [SharedReferencesCache]
    pub unsafe fn move_to_remote(self, remote_address: RemotePointerAddress) {
        let mut inner = self.inner_mut();
        // replace previous with null value
        // FIXME: find a more efficient way to do this enum variant swap
        let previous = mem::replace(
            &mut *inner,
            SharedContainerInner::EndpointOwned(unsafe {
                SelfOwnedSharedContainer::new_with_address(
                    BaseSharedValueContainer::null(),
                    SelfOwnedPointerAddress([0; 5]),
                )
            }),
        );

        *inner = match previous {
            SharedContainerInner::EndpointOwned(owned) => {
                SharedContainerInner::External(unsafe {
                    owned.convert_to_external_container(remote_address)
                })
            }
            _ => unreachable!(
                "OwnedSharedContainer must contain an EndpointOwned inner value"
            ),
        };
    }

    pub fn to_string_omit_content(&self) -> String {
        "(...)".to_string()
    }

    /// Marks the shared container as uninitialized.
    pub(crate) fn mark_uninitialized(&mut self) {
        self.is_uninitialized = true;
    }

    /// Unmarks the shared container as uninitialized.
    pub(crate) fn unset_uninitialized(&mut self) {
        self.is_uninitialized = false;
    }
}

impl_display_for_datex_value!(
    OwnedSharedContainer,
    impl core::fmt::Display for OwnedSharedContainer {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}", self.as_self_owned_shared_container().value())
        }
    }
);

impl _ExposeRcInternal for OwnedSharedContainer {
    type Shared = SharedContainerInner;
    fn get_rc_internal(&self) -> &Rc<RefCell<Self::Shared>> {
        &self.inner
    }
}

impl Identity for OwnedSharedContainer {
    fn identical(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for OwnedSharedContainer {}

/// PartialEq corresponds to pointer equality / identity for `Reference`.
impl PartialEq for OwnedSharedContainer {
    fn eq(&self, other: &Self) -> bool {
        self.identical(other)
    }
}

impl Hash for OwnedSharedContainer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(self.get_rc_internal());
        ptr.hash(state); // hash the address
    }
}

impl StructuralEq for OwnedSharedContainer {
    fn structural_eq(&self, other: &Self) -> bool {
        self.inner()
            .base_shared_container()
            .value_container()
            .structural_eq(
                other.inner().base_shared_container().value_container(),
            )
    }
}

impl ValueEq for OwnedSharedContainer {
    fn value_eq(&self, other: &Self) -> bool {
        self.inner()
            .base_shared_container()
            .value_container()
            .value_eq(other.inner().base_shared_container().value_container())
    }
}
