//! This module contains the implementation of the [ValueContainer] enum, which represents a container for values in the DATEX type system.
//! A [ValueContainer] can either be a local value, which directly contains a [Value], or a shared value, which contains a reference to a [SharedContainer].
use crate::{
    utils::sheep::Sheep,
};
mod equality;
mod identity;
use core::ops::Deref;
mod serde_dif;
use super::value::Value;
use crate::{
    prelude::*,
    shared_values::SharedContainer,
    types::{
        r#type::Type,
        type_definition::TypeDefinition,
        type_definition_with_metadata::{
            TypeDefinitionWithMetadata, TypeMetadata,
        },
    },
};

pub mod borrowed_value_container;

mod apply;
mod update_handler;
use crate::{
    shared_values::{
        OwnedSharedContainer, ReferencedSharedContainer,
        collapsed_container_value::{
            CollapsedContainerValue, CollapsedContainerValueMut,
        },
        traits::SharedContainerCommon,
    },
    traits::convert_value_container::ConvertValueContainer,
    utils::{
        impl_display_for_datex_value::impl_display_for_datex_value,
        sheep_mut::SheepMut,
    },
    values::core_values::endpoint::Endpoint,
};
use core::{
    fmt::Display,
    hash::{Hash, Hasher},
    ops::FnOnce,
};
use serde::Serialize;
use crate::instruction::regular_instruction::RegularInstruction;
use crate::traits::convert_parts::FromParts;
use crate::traits::convert_value::ConvertValue;
use crate::values::core_value::CoreValue;
use crate::values::core_values::text::Text;

pub mod classification;
pub mod convert_parts;
mod convert_value_container;
mod datex_hash;
pub mod error;
pub mod get_core_lib_type_id;
pub mod get_datex_type;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
#[derive(Debug, Eq, Clone)]
pub enum ValueContainer {
    Local(Value),
    Shared(SharedContainer),
}

impl ValueContainer {
    /// Unwraps an [Option<ValueContainer>] and returns a [ValueContainer].
    /// If the Option is None, returns a ValueContainer containing a null value.
    pub fn new_from_option(value: Option<ValueContainer>) -> ValueContainer {
        match value {
            Some(value) => value,
            None => ValueContainer::Local(Value::null()),
        }
    }

    /// Creates a new [ValueContainer::Local] from a [Value]
    pub fn local(value: impl Into<Value>) -> Self {
        ValueContainer::Local(value.into())
    }

    pub fn owner(&self) -> Endpoint {
        match self {
            ValueContainer::Local(_value) => Endpoint::LOCAL,
            ValueContainer::Shared(shared) => {
                shared.pointer_address().endpoint()
            }
        }
    }

    /// Gets a reference to the inner [ValueContainer], regardless of whether it is local or shared.
    pub fn value_container(&self) -> Sheep<'_, ValueContainer> {
        match self {
            ValueContainer::Local(_) => Sheep::Borrowed(self),
            ValueContainer::Shared(shared) => {
                Sheep::Ref(shared.value_container())
            }
        }
    }

    /// Gets a mutable reference to the inner [ValueContainer], regardless of whether it is local or shared.
    pub fn value_container_mut(&mut self) -> SheepMut<'_, ValueContainer> {
        match self {
            ValueContainer::Local(_) => SheepMut::Borrowed(self),
            ValueContainer::Shared(shared) => {
                SheepMut::Ref(shared.value_container_mut())
            }
        }
    }

    pub fn collapsed_value(&self) -> CollapsedContainerValue<'_> {
        match self {
            ValueContainer::Local(value) => {
                CollapsedContainerValue::new_local(value)
            }
            ValueContainer::Shared(shared) => shared.collapsed_value(),
        }
    }

    pub fn collapsed_value_mut(&mut self) -> CollapsedContainerValueMut<'_> {
        match self {
            ValueContainer::Local(value) => {
                CollapsedContainerValueMut::new_local(value)
            }
            ValueContainer::Shared(shared) => shared.collapsed_value_mut(),
        }
    }

    /// Gets a cloned, collapsed inner value.
    /// Use [ValueContainer::with_collapsed_value] instead whenever possible
    /// or match the [ValueContainer]
    pub fn get_cloned_value(&self) -> Value {
        let val = self.collapsed_value();
        val.borrow().clone()
    }

    /// Tries to get an immutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as<T>(&self) -> Option<&T>
    where
        T: ConvertValueContainer,
    {
        T::try_borrow_from_value_container(self).ok()
    }

    /// Tries to get a mutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ConvertValueContainer,
    {
        T::try_borrow_mut_from_value_container(self).ok()
    }

    /// Tries to get an immutable reference to the value as a string slice.
    /// Does not perform any type conversion.
    /// In contrast to [ValueContainer::try_as::<String>()], this method also works for [Text] values containing a static string slice.
    pub fn try_as_str(&self) -> Option<&str> {
        match self {
            ValueContainer::Local(value) => value.try_as_str(),
            ValueContainer::Shared(shared) => None,
        }
    }
    
    /// Tries to get the current collapsed value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_into_value<T>(self) -> Result<T, ValueContainer>
    where
        T: ConvertValueContainer,
    {
        T::try_from_value_container(self)
    }

    /// Tries to convert the current value into the specific specified type.
    /// Performs a cast between native and core values for local values if necessary, e.g. converting a [CoreValue::Map] into a native rust struct.
    pub fn try_cast_into_value<T>(self) -> Result<T, ()>
    where
        T: ConvertValueContainer + FromParts,
    {
        T::try_cast_from_value_container(self)
    }

    /// Strips any local observers from the given value container.
    /// This method should be called when a value is moved from its [SharedContainer] parent.
    pub fn without_local_observers(self) -> ValueContainer {
        match self {
            ValueContainer::Local(value) => {
                ValueContainer::Local(value.without_local_observers())
            }
            val => val,
        }
    }

    /// Performs a clone used by the "clone" command
    /// Local values are just cloned normally
    /// For shared value, the inner value container is cloned (shared x -> x)
    pub fn get_cloned(&self) -> ValueContainer {
        match self {
            ValueContainer::Local(value) => {
                ValueContainer::Local(value.clone())
            }
            ValueContainer::Shared(shared) => shared.value_container().clone(),
        }
    }

    /// Returns the actual type of the contained value, resolving shared values if necessary.
    pub fn actual_type(&self) -> TypeDefinition {
        match self {
            ValueContainer::Local(local) => local.actual_type(),
            ValueContainer::Shared(shared) => shared.actual_type(),
        }
    }

    /// Returns the actual type that describes the value container (e.g. integer or 'mut shared mut integer).
    pub fn actual_container_type(&self) -> TypeDefinitionWithMetadata {
        match self {
            ValueContainer::Local(value) => TypeDefinitionWithMetadata::new(
                value.actual_type(),
                TypeMetadata::default(),
            ),
            ValueContainer::Shared(shared) => {
                let inner_type =
                    shared.value_container().actual_container_type();
                TypeDefinitionWithMetadata::new(
                    TypeDefinition::Box(Box::new(Type::from(inner_type))),
                    TypeMetadata::Shared {
                        mutability: shared.container_mutability(),
                        ownership: shared.ownership(),
                    },
                )
            }
        }
    }

    /// For local values, returns the actual type of the value container
    /// For shared values, returns the allowed type of the value container
    pub fn allowed_or_actual_type(&self) -> Sheep<'_, TypeDefinition> {
        match self {
            ValueContainer::Local(value) => Sheep::Owned(value.actual_type()),
            ValueContainer::Shared(shared) => Sheep::Ref(shared.allowed_type()),
        }
    }

    /// Returns the contained SharedContainer if it is a SharedContainer, otherwise returns None.
    pub fn maybe_shared(&self) -> Option<&SharedContainer> {
        if let ValueContainer::Shared(shared) = self {
            Some(shared)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the contained SharedContainer if it is a SharedContainer, otherwise returns None.
    pub fn maybe_shared_mut(&mut self) -> Option<&mut SharedContainer> {
        if let ValueContainer::Shared(shared) = self {
            Some(shared)
        } else {
            None
        }
    }

    /// Runs a closure with the contained SharedContainer if it is a SharedContainer, otherwise returns None.
    pub fn with_maybe_shared<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&SharedContainer) -> R,
    {
        if let ValueContainer::Shared(shared) = self {
            Some(f(shared))
        } else {
            None
        }
    }

    /// Returns a reference to the contained SharedContainer, panics if it is not a SharedContainer.
    pub fn shared_unchecked(&self) -> &SharedContainer {
        match self {
            ValueContainer::Shared(shared) => shared,
            _ => {
                core::panic!("Cannot convert ValueContainer to SharedContainer")
            }
        }
    }
    pub fn shared_unchecked_mut(&mut self) -> &mut SharedContainer {
        match self {
            ValueContainer::Shared(shared) => shared,
            _ => {
                core::panic!("Cannot convert ValueContainer to SharedContainer")
            }
        }
    }

    /// Returns true if the underlying value is uninitialized (recursive).
    pub fn is_uninitialized(&self) -> bool {
        match self {
            ValueContainer::Local(value) => value.is_uninitialized(),
            ValueContainer::Shared(shared) => shared.is_uninitialized(),
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            ValueContainer::Local(value) => value.is_null(),
            ValueContainer::Shared(shared) => {
                shared.value_container().is_null()
            }
        }
    }
}

impl<T: Into<Value>> From<T> for ValueContainer {
    fn from(value: T) -> Self {
        ValueContainer::Local(value.into())
    }
}

impl From<SharedContainer> for ValueContainer {
    fn from(shared: SharedContainer) -> Self {
        ValueContainer::Shared(shared)
    }
}

impl From<OwnedSharedContainer> for ValueContainer {
    fn from(shared: OwnedSharedContainer) -> Self {
        ValueContainer::Shared(shared.into())
    }
}

impl From<ReferencedSharedContainer> for ValueContainer {
    fn from(shared: ReferencedSharedContainer) -> Self {
        ValueContainer::Shared(shared.into())
    }
}

impl Hash for ValueContainer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ValueContainer::Local(value) => value.hash(state),
            ValueContainer::Shared(pointer) => pointer.hash(state),
        }
    }
}

impl_display_for_datex_value!(
    ValueContainer,
    impl core::fmt::Display for ValueContainer {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                ValueContainer::Local(value) => core::write!(f, "{value}"),
                // TODO #118: only simple temporary way to distinguish between Value and Pointer
                ValueContainer::Shared(shared) => {
                    if shared.is_borrowed() {
                        write!(f, "{}", shared.to_string_omit_content())
                    } else {
                        let value = shared.collapsed_value();
                        write!(f, "shared ({})", value.borrow().as_ref())
                    }
                }
            }
        }
    }
);
