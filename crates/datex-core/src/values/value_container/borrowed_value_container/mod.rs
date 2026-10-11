use crate::{
    shared_values::{
        OwnedSharedContainer, ReferencedSharedContainer, SharedContainer,
    },
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
        core_values::native::DatexNative,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
        value_container::ValueContainer,
    },
};
mod classification;
mod datex_hash;
mod equality;
mod get_core_lib_type_id;
mod get_datex_type;
pub mod hash;
mod identity;
mod ops;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
pub mod to_instructions;

use core::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum BorrowedValueContainer<'a> {
    Local(BorrowedValue<'a>),
    Shared(SharedContainer),
}

#[cfg(feature = "value_display")]
impl Display for BorrowedValueContainer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        use crate::decompiler::ast_to_source_code::value_to_source_code_default;

        write!(f, "{}", value_to_source_code_default(self))
    }
}
#[cfg(not(feature = "value_display"))]
impl Display for BorrowedValueContainer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "[[ BorrowedValueContainer ]]")
    }
}

impl<'a> BorrowedValueContainer<'a> {
    /// Creates a new `BorrowedValueContainer` from a reference to a native value.
    pub fn native_borrowed<T: DatexNative>(
        val: impl Into<Goat<'a, T>>,
    ) -> Self {
        BorrowedValueContainer::Local(BorrowedValue::native_borrowed(val))
    }

    /// Tries to get an immutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as<T: ?Sized>(self) -> Result<Goat<'a, T>, Self>
    where
        Goat<'a, T>: TryFrom<BorrowedValue<'a>, Error = BorrowedValue<'a>>,
    {
        match self {
            BorrowedValueContainer::Local(value) => {
                value.try_as().map_err(BorrowedValueContainer::Local)
            }
            BorrowedValueContainer::Shared(_) => Err(self),
        }
    }

    pub fn try_clone_to_value_container(self) -> Result<ValueContainer, ()>
    where
        CoreValue: Clone,
    {
        match self {
            BorrowedValueContainer::Local(value) => {
                Ok(ValueContainer::Local(value.try_clone_to_value()?))
            }
            BorrowedValueContainer::Shared(shared) => {
                Ok(ValueContainer::Shared(shared.clone()))
            }
        }
    }
}

pub enum BorrowedValueContainerMut<'a> {
    Local(BorrowedValueMut<'a>),
    Shared(SharedContainer),
}

impl<'a> BorrowedValueContainerMut<'a> {
    /// Creates a new `BorrowedValueContainer` from a reference to a native value.
    pub fn native_borrowed<T: DatexNative>(
        val: impl Into<GoatMut<'a, T>>,
    ) -> Self {
        BorrowedValueContainerMut::Local(BorrowedValueMut::native_borrowed(val))
    }

    /// Tries to get an immutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as<T: ?Sized>(self) -> Option<Goat<'a, T>>
    where
        Goat<'a, T>: TryFrom<BorrowedValueMut<'a>>,
    {
        match self {
            BorrowedValueContainerMut::Local(value) => value.try_as(),
            BorrowedValueContainerMut::Shared(_) => None,
        }
    }

    /// Tries to get a mutable reference to the value as a specified type.
    /// Does not perform any type conversion.
    /// This only works for local values, not for shared values.
    pub fn try_as_mut<T>(self) -> Option<GoatMut<'a, T>>
    where
        GoatMut<'a, T>: TryFrom<BorrowedValueMut<'a>>,
    {
        match self {
            BorrowedValueContainerMut::Local(value) => value.try_as_mut(),
            BorrowedValueContainerMut::Shared(_) => None,
        }
    }
}

impl<'a> From<BorrowedValue<'a>> for BorrowedValueContainer<'a> {
    fn from(borrowed_value: BorrowedValue<'a>) -> Self {
        BorrowedValueContainer::Local(borrowed_value)
    }
}

impl<'a> From<&'a ValueContainer> for BorrowedValueContainer<'a> {
    fn from(value_container: &'a ValueContainer) -> Self {
        match value_container {
            ValueContainer::Shared(shared_container) => {
                BorrowedValueContainer::Shared(shared_container.clone())
            }
            ValueContainer::Local(local_value) => {
                BorrowedValueContainer::Local(BorrowedValue::from(local_value))
            }
        }
    }
}

impl<'a> From<&'a mut ValueContainer> for BorrowedValueContainer<'a> {
    fn from(value_container: &'a mut ValueContainer) -> Self {
        match value_container {
            ValueContainer::Shared(shared_container) => {
                BorrowedValueContainer::Shared(shared_container.clone())
            }
            ValueContainer::Local(local_value) => {
                BorrowedValueContainer::Local(BorrowedValue::from(local_value))
            }
        }
    }
}

impl<'a> From<&'a mut ValueContainer> for BorrowedValueContainerMut<'a> {
    fn from(value_container: &'a mut ValueContainer) -> Self {
        match value_container {
            ValueContainer::Shared(shared_container) => {
                BorrowedValueContainerMut::Shared(shared_container.clone())
            }
            ValueContainer::Local(local_value) => {
                BorrowedValueContainerMut::Local(BorrowedValueMut::from(
                    local_value,
                ))
            }
        }
    }
}

impl<'a> From<BorrowedValueMut<'a>> for BorrowedValueContainerMut<'a> {
    fn from(borrowed_value: BorrowedValueMut<'a>) -> Self {
        BorrowedValueContainerMut::Local(borrowed_value)
    }
}

impl From<SharedContainer> for BorrowedValueContainer<'_> {
    fn from(shared_container: SharedContainer) -> Self {
        BorrowedValueContainer::Shared(shared_container)
    }
}

impl From<OwnedSharedContainer> for BorrowedValueContainer<'_> {
    fn from(owned_shared_container: OwnedSharedContainer) -> Self {
        BorrowedValueContainer::Shared(SharedContainer::Referenced(
            owned_shared_container.derive_with_max_mutability(),
        ))
    }
}
impl From<ReferencedSharedContainer> for BorrowedValueContainer<'_> {
    fn from(referenced_shared_container: ReferencedSharedContainer) -> Self {
        BorrowedValueContainer::Shared(SharedContainer::Referenced(
            referenced_shared_container,
        ))
    }
}

impl<'a> From<&'a Value> for BorrowedValueContainer<'a> {
    fn from(value: &'a Value) -> Self {
        BorrowedValueContainer::Local(BorrowedValue::from(value))
    }
}
