use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    values::{value::Value, value_container::ValueContainer},
};
use core::{
    any::Any,
    fmt::{Debug, Formatter},
    ops::Deref,
};
mod datex_hash;
mod datex_native_trait;
pub mod display;
mod get_core_lib_type_id;
mod get_datex_type;
mod ops;
mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
mod value_access;
use crate::{
    libs::core::type_id::CoreLibTypeId,
    preludes::derive::{BorrowedValueContainer, StaticClassification},
    traits::{
        convert_core_value::ConvertCoreValue,
        convert_value_container::ConvertValueContainer, try_clone::TryClone,
    },
    values::{core_value::CoreValue, value::borrowed_value::BorrowedValue},
};
pub use datex_native_trait::*;

impl<T: DatexNative + ConvertCoreValue + StaticClassification>
    ConvertValueContainer for T
{
    fn to_value_container(
        self,
        cache: &mut SharedReferencesCache,
    ) -> ValueContainer {
        ValueContainer::Local(Value::native(self, cache))
    }

    fn as_borrowed_value_container(
        &self,
        cache: &mut SharedReferencesCache,
    ) -> BorrowedValueContainer<'_> {
        BorrowedValueContainer::Local(BorrowedValue::native_borrowed(
            self, cache,
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
                match validate_classification::<T>(&value) {
                    Ok(_) => Self::try_from_core_value(value.inner).map_err(
                        |inner| {
                            ValueContainer::Local(Value {
                                inner,
                                classification: value.classification,
                            })
                        },
                    ),
                    Err(_) => Err(ValueContainer::Local(value)),
                }
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
            ValueContainer::Local(value) => {
                match validate_classification::<T>(value) {
                    Ok(_) => {
                        Ok(Self::try_borrow_from_core_value(&value.inner)?)
                    }
                    Err(_) => Err(()),
                }
            }
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
                match validate_classification::<T>(value) {
                    Ok(_) => Ok(Self::try_borrow_mut_from_core_value(
                        &mut value.inner,
                    )?),
                    Err(_) => Err(()),
                }
            }
            _ => Err(()),
        }
    }
}

// if the target type has no classification, i.e. is structural
// but the value has a classification, we cannot convert it into the target type
pub fn validate_classification<T>(value: &Value) -> Result<(), ()>
where
    T: StaticClassification,
{
    if !T::has_classification() && !value.classification().is_none() {
        Err(())
    } else {
        Ok(())
    }
}

pub struct NativeCoreValue {
    pub value: Box<dyn DatexNative + 'static>,
}

impl TryClone for NativeCoreValue {
    fn try_clone(&self) -> Result<CoreValue, ()> {
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

    pub fn to_datex_native_value(
        self,
        cache: &mut SharedReferencesCache,
    ) -> Value {
        Value::native_dyn(self.value, cache)
    }

    pub fn core_lib_type_id(&self) -> CoreLibTypeId {
        self.value.core_lib_type_id()
    }

    /// Attempt to downcast the native value to a specific type.
    /// Returns `Some(&T)` if the downcast is successful, or `None` if it fails.
    pub fn try_as<T: 'static>(&self) -> Option<&T> {
        self.value.as_any().downcast_ref::<T>()
    }

    /// Attempt to downcast the native value to a specific type.
    /// Returns `Some(&mut T)` if the downcast is successful, or `None` if it fails.
    pub fn try_as_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.value.as_any_mut().downcast_mut::<T>()
    }

    /// Attempt to downcast the native value to a specific type.
    /// Returns `Ok(T)` if the downcast is successful, or `Err(Self)` if it fails.
    pub fn try_into_value<T: 'static>(self) -> Result<T, Self> {
        if self.as_any().is::<T>() {
            // SAFETY: we just verified the type
            Ok(*self.into_any().downcast::<T>().unwrap())
        } else {
            Err(self)
        }
    }
}

impl Clone for NativeCoreValue {
    fn clone(&self) -> Self {
        match self.try_clone().unwrap() {
            CoreValue::Native(n) => n,
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
        prelude::*, values::value::value_classification::ValueClassification,
    };

    #[test]
    fn serde() {
        let val = NativeCoreValue::new("xx".to_string());
        let ser =
            val.to_datex_native_value(&mut SharedReferencesCache::default());
        assert_eq!(ser.classification(), &ValueClassification::None,);
        assert_eq!(
            ser.inner,
            CoreValue::Native(NativeCoreValue::new("xx".to_string()))
        );
    }
}
