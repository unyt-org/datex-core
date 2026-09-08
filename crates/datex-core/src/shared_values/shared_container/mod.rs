//! This module contains the implementation of the shared container, which is the holder of [SharedContainerInner]
//! and the top-level wrapper for any owned or referenced shared container.
use crate::{
    runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
    shared_values::{
        OwnedSharedContainer, PointerAddress, ReferencedSharedContainer,
        SelfOwnedPointerAddress, SharedContainerInner,
        SharedContainerMutability, SharedContainerOwnership,
        errors::{
            UnexpectedImmutableReferenceError,
            UnexpectedSharedContainerOwnershipError,
        },
        traits::_ExposeRcInternal,
    },
    values::value_container::ValueContainer,
};
pub mod identity;
pub(crate) mod to_instructions;
use crate::{
    prelude::*,
    shared_values::{
        ReferenceMutability,
        base_shared_value_container::observers::{
            ObserveOptions, Observer, ObserverCallback, ObserverError,
            ObserverId,
        },
        collapsed_container_value::{
            CollapsedContainerValue, CollapsedContainerValueMut,
        },
        shared_mut::SharedMut,
        traits::SharedContainerCommon,
    },
    traits::convert_value_container::ConvertValueContainer,
    types::type_definition::TypeDefinition,
    value_updates::update_handler::{
        InternalMutabilityUpdateHandler, UpdateCallbackData,
    },
    values::core_value::CoreValue,
};
use alloc::rc::Rc;
use core::{
    cell::{Ref, RefCell, RefMut},
    fmt::{Debug, Display, Formatter},
    mem,
    ops::Deref,
};

pub mod apply;
pub mod serde_dif;
/// Top-level wrapper for any owned or referenced shared container,
/// which can either be an owned shared container or a reference to a shared container.
pub enum SharedContainer {
    /// An owned shared container (`shared X`). This is always points to a [SharedContainerInner::EndpointOwned]
    Owned(OwnedSharedContainer),
    /// A referenced shared container (`'shared X` or `'mut shared X`).
    /// This can point to either a [SharedContainerInner::EndpointOwned] or a [SharedContainerInner::External]
    Referenced(ReferencedSharedContainer),
}

impl Debug for SharedContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // recursive reference
        if self.is_borrowed() {
            f.write_str("(...)")
        } else {
            match self {
                SharedContainer::Owned(owned) => {
                    f.debug_tuple("SharedContainer").field(owned).finish()
                }
                SharedContainer::Referenced(reference) => {
                    f.debug_tuple("SharedContainer").field(reference).finish()
                }
            }
        }
    }
}

impl SharedContainer {
    /// Creates a new owned [SharedContainer] with an initial [ValueContainer],
    /// a [SharedContainerMutability], and a [SelfOwnedPointerAddressProvider].
    ///
    /// The allowed type is inferred from the value_container's allowed type.
    pub fn new_owned_with_inferred_allowed_type<T: Into<ValueContainer>>(
        value_container: T,
        mutability: SharedContainerMutability,
        address_provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Self {
        SharedContainer::Owned(
            OwnedSharedContainer::new_with_inferred_allowed_type(
                value_container,
                mutability,
                address_provider,
            ),
        )
    }

    /// Creates a new owned [SharedContainer] with an initial [ValueContainer],
    /// a [SharedContainerMutability], and a [SelfOwnedPointerAddress].
    ///
    /// The allowed type is inferred from the value_container's allowed type.
    /// # Safety
    /// The caller must ensure that the address is not used anywhere else.
    pub unsafe fn new_owned_with_inferred_allowed_type_unsafe<
        T: Into<ValueContainer>,
    >(
        value_container: T,
        mutability: SharedContainerMutability,
        address: SelfOwnedPointerAddress,
    ) -> Self {
        SharedContainer::Owned(unsafe {
            OwnedSharedContainer::new_with_inferred_allowed_type_unsafe(
                value_container.into(),
                mutability,
                address,
            )
        })
    }

    /// Ensures that the shared container is mutable and returns it.
    /// Returns an ObserverError if the reference is immutable (or a type container).
    fn ensure_mutable_container(&self) -> Result<(), ObserverError> {
        if self.container_mutability() != SharedContainerMutability::Mutable {
            return Err(ObserverError::ImmutableValue);
        }
        Ok(())
    }

    /// Adds an observer to this shared container that will be notified on value changes.
    pub fn observe(
        &self,
        observer: Observer,
    ) -> Result<ObserverId, ObserverError> {
        self.ensure_mutable_container()?;
        let res = self.observer_data_mut().observe(observer)?;
        self.ensure_local_nested_observe_callbacks();

        Ok(res)
    }

    pub fn unobserve(
        &self,
        observer_id: ObserverId,
    ) -> Result<(), ObserverError> {
        self.ensure_mutable_container()?;
        self.observer_data_mut().unobserve(observer_id)?;

        // also disable local nested observe callbacks if there are no more observers registered
        if !self.observer_data().has_observers() {
            self.disable_local_nested_observe_callbacks();
        }

        Ok(())
    }

    /// Updates the options for an existing observer by its ID.
    /// Returns an error if the observer ID is not found or the reference is immutable.
    pub fn update_observer_options(
        &self,
        observer_id: ObserverId,
        options: ObserveOptions,
    ) -> Result<(), ObserverError> {
        self.ensure_mutable_container()?;
        self.observer_data_mut()
            .update_observer_options(observer_id, options)
    }

    // Enables observe callbacks for the inner local value if not yet enabled
    fn ensure_local_nested_observe_callbacks(&self) {
        let enabled = if !self.observer_data().get_local_observers_enabled()
            && let ValueContainer::Local(local_value) =
                self.base_shared_container_mut().value_container_mut()
        {
            let self_clone = self.clone();

            let callback: ObserverCallback = Rc::new(move |update| {
                // TODO: check if not already borrowed?
                self_clone.observer_data_mut().call_observers(update);
            });
            local_value.set_update_callback_data(Some(UpdateCallbackData {
                callback,
                path: vec![],
            }));
            true
        } else {
            false
        };

        if enabled {
            self.observer_data_mut().set_local_observers_enabled(true);
        }
    }

    fn disable_local_nested_observe_callbacks(&self) {
        let mut base = self.base_shared_container_mut();
        if self.observer_data().get_local_observers_enabled()
            && let ValueContainer::Local(local_value) =
                base.value_container_mut()
        {
            local_value.set_update_callback_data(None);
        }
        self.observer_data_mut().set_local_observers_enabled(false);
    }

    /// Gets the current actual [TypeDefinition] of the collapsed inner [Value]
    pub fn actual_type(&self) -> TypeDefinition {
        let value = self.collapsed_value();
        value.borrow().actual_type()
    }

    pub fn collapsed_value(&self) -> CollapsedContainerValue<'_> {
        CollapsedContainerValue::new_shared(self.get_collapsed_rc())
    }

    pub fn collapsed_value_mut(&self) -> CollapsedContainerValueMut<'_> {
        CollapsedContainerValueMut::new_shared(self.get_collapsed_rc())
    }

    /// Returns the Rc<RefCell<SharedContainerInner>> of the most inner local value of the shared container.
    fn get_collapsed_rc(&self) -> Rc<RefCell<SharedContainerInner>> {
        // TODO: no clone of RC?
        // collapse nested direct shared children until most inner local value is found
        let mut inner_rc = self.get_rc_internal().clone();

        loop {
            let next_rc = {
                let borrowed = inner_rc.borrow();

                match borrowed.base_shared_container().value_container() {
                    ValueContainer::Shared(shared) => {
                        Some(shared.get_rc_internal().clone())
                    }
                    _ => None,
                }
            };

            match next_rc {
                Some(rc) => inner_rc = rc,
                None => break,
            }
        }

        inner_rc
    }

    pub fn pointer_address(&self) -> PointerAddress {
        match self {
            SharedContainer::Owned(owned) => {
                PointerAddress::SelfOwned(owned.pointer_address().clone())
            }
            SharedContainer::Referenced(referenced) => {
                referenced.pointer_address()
            }
        }
    }

    /// Returns true if the shared container has a self owned pointer address
    pub fn is_self_owned(&self) -> bool {
        match self {
            SharedContainer::Owned(_owned) => true,
            SharedContainer::Referenced(referenced) => {
                matches!(
                    referenced.inner().deref(),
                    SharedContainerInner::EndpointOwned(_)
                )
            }
        }
    }

    /// Creates a new immutable [ReferencedSharedContainer] pointing to the same inner value as self.
    pub fn derive_immutable_reference(&self) -> ReferencedSharedContainer {
        match self {
            SharedContainer::Owned(owned) => owned.derive_immutable_reference(),
            SharedContainer::Referenced(referenced) => {
                referenced.derive_immutable_reference()
            }
        }
    }
    /// Tries to create a new mutable or immutable [ReferencedSharedContainer] pointing to the same inner value as this [OwnedSharedContainer].
    /// Returns an [Err] if the requested mutability is [ReferenceMutability::Mutable],
    /// but the current reference_mutability is [ReferenceMutability::Immutable] or the container itself is not mutable
    pub fn try_derive_reference_with_mutability(
        &self,
        mutability: ReferenceMutability,
    ) -> Result<ReferencedSharedContainer, UnexpectedImmutableReferenceError>
    {
        match mutability {
            ReferenceMutability::Immutable => {
                Ok(self.derive_immutable_reference())
            }
            ReferenceMutability::Mutable => self.try_derive_mutable_reference(),
        }
    }

    /// Tries to create a new mutable [ReferencedSharedContainer] pointing to the same inner value as this [OwnedSharedContainer].
    /// Returns an [Err] if the current reference_mutability is [ReferenceMutability::Immutable] or the container itself is not mutable
    pub fn try_derive_mutable_reference(
        &self,
    ) -> Result<ReferencedSharedContainer, UnexpectedImmutableReferenceError>
    {
        match self {
            SharedContainer::Owned(owned) => owned
                .try_derive_mutable_reference()
                .map_err(|_| UnexpectedImmutableReferenceError),
            SharedContainer::Referenced(referenced) => {
                referenced.try_derive_mutable_reference()
            }
        }
    }

    /// Returns the owned shared container if it is owned, otherwise returns an error.
    pub fn try_get_owned(
        &self,
    ) -> Result<&OwnedSharedContainer, UnexpectedSharedContainerOwnershipError>
    {
        match self {
            SharedContainer::Owned(owned) => Ok(owned),
            SharedContainer::Referenced(reference) => {
                Err(UnexpectedSharedContainerOwnershipError {
                    actual: SharedContainerOwnership::Referenced(
                        reference.reference_mutability(),
                    ),
                    expected: SharedContainerOwnership::Owned,
                })
            }
        }
    }

    /// Clones the shared container as a mutable reference if possible, otherwise as an immutable reference
    pub fn derive_reference_with_max_mutability(
        &self,
    ) -> ReferencedSharedContainer {
        self.try_derive_mutable_reference()
            .unwrap_or_else(|_| self.derive_immutable_reference())
    }

    /// Downgrades an owned shared container to a referenced shared container.
    /// If the shared container is already a referenced shared container, it will just be returned.
    /// If the shared container is owned, it will be replaced with a new referenced shared container pointing to the same inner value
    /// and the original owned shared container will be returned.
    /// Also sets the move indicator on the original owned shared container, so that we know it should be treated as moved.
    pub fn downgrade_to_reference(&mut self) -> SharedContainer {
        match self {
            SharedContainer::Owned(_) => {
                // replace previous with null value
                // FIXME: find a more efficient way to do this enum variant swap
                let previous = mem::replace(&mut *self, unsafe {
                    SharedContainer::new_owned_with_inferred_allowed_type_unsafe(
                        CoreValue::Null,
                        SharedContainerMutability::Immutable,
                        SelfOwnedPointerAddress::new([0; 5]),
                    )
                });

                // create a new referenced shared container and assign to self
                *self = previous.clone_with_move_indicator_if_owned();

                // return original, potentially owned shared container
                previous
            }
            SharedContainer::Referenced(referenced) => {
                SharedContainer::Referenced(referenced.clone())
            }
        }
    }

    /// Tries to upgrade an inner [ReferencedSharedContainer] to an [OwnedSharedContainer].
    /// If the container is already an [OwnedSharedContainer], it will be returned as is.
    /// If the move indicator is set, the inner value is moved into a new [OwnedSharedContainer] and returned.
    /// If the move indicator is not set, an [Err] is returned with the original [ReferencedSharedContainer].
    /// Panics if the move indicator is set but the address is not a [SelfOwnedPointerAddress].
    ///
    /// # Safety
    /// The caller must ensure that no other [OwnedSharedContainer] for the same inner value exists
    /// if move_indicator of the [ReferencedSharedContainer] is set.
    pub unsafe fn try_upgrade_to_owned(
        self,
    ) -> Result<OwnedSharedContainer, ReferencedSharedContainer> {
        match self {
            SharedContainer::Referenced(referenced_container) => unsafe {
                referenced_container.try_upgrade_to_owned()
            },
            SharedContainer::Owned(owned_container) => Ok(owned_container),
        }
    }

    /// Clones the shared container with the move indicator if it is an [OwnedSharedContainer],
    /// otherwise as a normal reference
    pub fn clone_with_move_indicator_if_owned(&self) -> SharedContainer {
        SharedContainer::Referenced(match self {
            SharedContainer::Owned(owned) => owned.clone_with_move_indicator(),
            SharedContainer::Referenced(referenced) => referenced.clone(),
        })
    }

    /// Returns true if the shared container is an [OwnedSharedContainer] or a [ReferencedSharedContainer] that is marked as moved.
    pub fn treat_as_move(&self) -> bool {
        match self {
            SharedContainer::Owned(_owned) => true,
            SharedContainer::Referenced(referenced) => {
                referenced.treat_as_move()
            }
        }
    }

    pub fn to_string_omit_content(&self) -> String {
        match self {
            SharedContainer::Owned(owned) => owned.to_string_omit_content(),
            SharedContainer::Referenced(referenced) => {
                referenced.to_string_omit_content()
            }
        }
    }

    /// Tries to get an immutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as<T>(&self) -> Option<Ref<'_, T>>
    where
        T: ConvertValueContainer,
    {
        Ref::filter_map(self.value_container(), |value| value.try_as::<T>())
            .ok()
    }

    /// Tries to get a mutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as_mut<T>(&self) -> Option<SharedMut<'_, T>>
    where
        T: ConvertValueContainer,
    {
        RefMut::filter_map(self.value_container_mut(), |value| {
            value.try_as_mut::<T>()
        })
        .ok()
        .map(|v| SharedMut::new(v, self))
    }

    /// This method is called when a borrow of the inner shared container is dropped.
    pub fn notify_borrow_dropped(&self) {
        // TODO: this could be used in the future to trigger queued updates only after a borrow was dropped
    }

    /// Marks the shared container as uninitialized.
    pub(crate) fn mark_uninitialized(&mut self) {
        match self {
            SharedContainer::Owned(owned) => owned.mark_uninitialized(),
            SharedContainer::Referenced(referenced) => {
                referenced.mark_uninitialized()
            }
        }
    }

    /// Unmarks the shared container as uninitialized.
    pub(crate) fn unset_uninitialized(&mut self) {
        match self {
            SharedContainer::Owned(owned) => owned.unset_uninitialized(),
            SharedContainer::Referenced(referenced) => {
                referenced.unset_uninitialized()
            }
        }
    }
}

/// Custom clone implementation for [SharedContainer].
/// A [SharedContainer::Owned] cannot be cloned as is, only a new reference can be created
/// A [SharedContainer::Referenced] can be cloned normally
impl Clone for SharedContainer {
    fn clone(&self) -> Self {
        match self {
            // An owned container cannot be cloned, only a new reference can be created
            SharedContainer::Owned(owned) => {
                SharedContainer::Referenced(owned.derive_with_max_mutability())
            }
            // A referenced container can be cloned
            SharedContainer::Referenced(referenced) => {
                SharedContainer::Referenced(referenced.clone())
            }
        }
    }
}

// TODO: use impl_display_for_datex_value
impl Display for SharedContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            SharedContainer::Owned(owned) => write!(f, "{}", owned),
            SharedContainer::Referenced(referenced) => {
                write!(f, "{}", referenced)
            }
        }
    }
}

pub mod clone_unsafe;
mod common;
pub mod equality;
pub mod get_datex_type;
pub mod update_handler;

impl From<OwnedSharedContainer> for SharedContainer {
    fn from(value: OwnedSharedContainer) -> Self {
        SharedContainer::Owned(value)
    }
}

impl From<ReferencedSharedContainer> for SharedContainer {
    fn from(value: ReferencedSharedContainer) -> Self {
        SharedContainer::Referenced(value)
    }
}

impl _ExposeRcInternal for SharedContainer {
    type Shared = SharedContainerInner;
    fn get_rc_internal(&self) -> &Rc<RefCell<Self::Shared>> {
        match self {
            SharedContainer::Owned(owned) => owned.get_rc_internal(),
            SharedContainer::Referenced(referenced) => {
                referenced.get_rc_internal()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_matches;

    #[test]
    fn immutable_reference_observe_fails() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();
        let shared = SharedContainer::new_owned_with_inferred_allowed_type(
            42,
            SharedContainerMutability::Immutable,
            address_provider,
        );
        assert_matches!(
            shared.observe(Observer::new(|_| {})),
            Err(ObserverError::ImmutableValue)
        );

        let mut r = SharedContainer::new_owned_with_inferred_allowed_type(
            42,
            SharedContainerMutability::Mutable,
            address_provider,
        );
        assert_matches!(r.observe(Observer::new(|_| {})), Ok(_));
    }
}
