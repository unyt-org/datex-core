use crate::{
    prelude::*,
    utils::goat_mut::GoatMut,
    values::{
        value::{Value, borrowed_value::BorrowedValueMut},
        value_container::ValueContainer,
    },
};
use core::{
    any::Any,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};
mod datex_native_ops;
pub use datex_native_ops::*;
mod datex_native_trait;
pub mod display;
mod get_datex_type;
pub mod goat_helpers;
mod ops;

use crate::{
    libs::core::type_id::CoreLibTypeId,
    traits::{
        classification::Classification, convert_value::ConvertValue,
        convert_value_container::ConvertValueContainer, try_clone::TryClone,
    },
    utils::goat::Goat,
    values::{
        value::borrowed_value::BorrowedValue,
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};
pub use datex_native_trait::*;
pub use ops::*;

impl<T: DatexNative + ConvertValue + Classification> ConvertValueContainer
    for T
{
    fn to_value_container(self) -> ValueContainer {
        ValueContainer::Local(Value::new(self))
    }

    fn as_borrowed_value_container(&self) -> BorrowedValueContainer<'_> {
        BorrowedValueContainer::Local(BorrowedValue::Native(Goat::Borrowed(
            self,
        )))
    }
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut<'_> {
        BorrowedValueContainerMut::Local(BorrowedValueMut::Native(
            GoatMut::Borrowed(self),
        ))
    }

    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Local(value) => {
                Self::try_from_value(value).map_err(ValueContainer::Local)
            }
            _ => Err(value_container),
        }
    }

    fn try_borrow_from_value_container(
        value_container: &ValueContainer,
    ) -> Result<&Self, ()>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Local(value) => Self::try_borrow_from_value(value),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value_container(
        value_container: &mut ValueContainer,
    ) -> Result<&mut Self, ()>
    where
        Self: Sized,
    {
        match value_container {
            ValueContainer::Local(value) => {
                Self::try_borrow_mut_from_value(value)
            }
            _ => Err(()),
        }
    }
}

pub struct NativeCoreValue {
    pub value: Box<dyn DatexNative + 'static>,
}

impl Deref for NativeCoreValue {
    type Target = dyn DatexNative + 'static;

    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl DerefMut for NativeCoreValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl TryClone for NativeCoreValue {
    fn try_clone(&self) -> Result<Value, ()> {
        self.value.deref().try_clone()
    }
}

impl NativeCoreValue {
    pub fn new<T>(value: T) -> Self
    where
        T: DatexNative + 'static,
    {
        NativeCoreValue {
            value: Box::new(value),
        }
    }

    pub fn as_any(&self) -> &dyn Any {
        self.value.as_ref().as_any()
    }
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self.value.as_mut().as_any_mut()
    }
    pub fn into_any(self) -> Box<dyn Any> {
        self.value
    }

    pub fn core_lib_type_id(&self) -> CoreLibTypeId {
        self.value.core_lib_type_id()
    }

    /// Attempt to downcast the native value to a specific type.
    /// Returns `Some(&T)` if the downcast is successful, or `None` if it fails.
    pub fn try_as<T: 'static>(&self) -> Option<&T> {
        let any = self.value.as_any();
        if let Some(val) = any.downcast_ref::<T>() {
            Some(val)
        } else if let Some(val) = any.downcast_ref::<Box<T>>() {
            Some(&**val)
        } else if let Some(val) = any.downcast_ref::<Option<T>>() {
            val.as_ref()
        } else {
            None
        }
    }

    /// Attempt to downcast the native value to a specific type.
    /// Returns `Some(&mut T)` if the downcast is successful, or `None` if it fails.
    pub fn try_as_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let any_mut = self.value.as_any_mut();
        if let Some(val) = any_mut.downcast_mut::<T>() {
            Some(val)
        } else if let Some(val) = any_mut.downcast_mut::<Box<T>>() {
            Some(&mut **val)
        } else if let Some(val) = any_mut.downcast_mut::<Option<T>>() {
            val.as_mut()
        } else {
            None
        }
    }

    /// Attempt to downcast the native value to a specific type.
    /// Returns `Ok(T)` if the downcast is successful, or `Err(Self)` if it fails.
    pub fn try_into_value<T: 'static>(self) -> Result<T, Self> {
        let any = self.as_any();
        if any.is::<T>() {
            // SAFETY: we just verified the type
            Ok(*self.into_any().downcast::<T>().unwrap())
        } else if any.is::<Box<T>>() {
            // SAFETY: we just verified the type
            Ok(**self.into_any().downcast::<Box<T>>().unwrap())
        } else if any.is::<Option<T>>() {
            // SAFETY: we just verified the type
            if self.as_any().downcast_ref::<Option<T>>().unwrap().is_some() {
                Ok(self.into_any().downcast::<Option<T>>().unwrap().unwrap())
            } else {
                Err(self)
            }
        } else {
            Err(self)
        }
    }
}

impl Clone for NativeCoreValue {
    fn clone(&self) -> Self {
        match self.try_clone().unwrap() {
            Value::Native(n) => n,
            _ => unreachable!(),
        }
    }
}

impl Debug for NativeCoreValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::cache::shared_references_cache::SharedReferencesCache,
        values::{core_value::CoreValue, core_values::native::NativeCoreValue},
    };

    use crate::{
        prelude::*,
        values::value::{Value, value_classification::ValueClassification},
    };

    #[test]
    fn serde() {
        let cache = &mut SharedReferencesCache::default();
        let val = NativeCoreValue::new("xx".to_string());
        let ser = Value::Native(val);
        assert_eq!(
            ser.classification(cache),
            ValueClassification::new_unclassified(),
        );
        assert_eq!(ser, Value::Native(NativeCoreValue::new("xx".to_string())));
    }
}
