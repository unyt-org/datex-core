//! This module contains the implementation of the [Value] struct, which represents a value in the DATEX type system.
//! A [Value] consists of a [CoreValue] representation and an optional custom type.
use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    types::type_definition::{
        TypeDefinition, callable::CallableTypeDefinition,
    },
    values::{
        core_value::CoreValue,
        core_values::{
            callable::{Callable, CallableBody},
            native::DatexNative,
        },
        value_container::{ValueContainer, value_key::BorrowedValueKey},
    },
};
pub mod apply;
pub mod borrowed_value;
mod child_iterator;
pub mod classification;
pub mod convert_parts;
pub mod convert_value_container;
mod datex_hash;
mod datex_native;
pub mod equality;
pub mod get_core_lib_type_id;
pub mod get_datex_type;
mod local_child_path_resolver;
pub mod ops;
pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
pub mod update_handler;
mod value_access;
pub mod value_classification;

use crate::{
    preludes::derive::{
        ConvertCoreValue, DatexNativeStructural, TaggedTypeDefinition,
    },
    shared_values::errors::AccessError,
    traits::{
        convert_value_container::ConvertValueContainer,
        datex_native_only_structural::DatexNativeOnlyStructural,
        static_classification::StaticClassification, value_access::ValueAccess,
    },
    types::r#type::Type,
    utils::impl_display_for_datex_value::impl_display_for_datex_value,
    value_updates::update_handler::InternalMutabilityUpdateHandler,
    values::{
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
        core_values::{endpoint::Endpoint, native::validate_classification},
        value::value_classification::{ValueClassification, ValueTag},
    },
};
use core::{
    fmt::{Debug, Display, Formatter},
    result::Result,
};

#[derive(Debug)]
pub struct Value {
    /// The inner representation of the value, which is a [CoreValue].
    pub inner: CoreValue,
    /// additional extensions for the value, including its entity type, impls, and tag
    pub classification: ValueClassification,
}

impl Clone for Value {
    fn clone(&self) -> Self {
        Value {
            inner: self.inner.clone(),
            classification: self.classification.clone(),
        }
    }
}

impl<T: Into<CoreValue>> From<T> for Value {
    fn from(inner: T) -> Self {
        let inner = inner.into();
        Value {
            inner,
            classification: ValueClassification::default(),
        }
    }
}

impl Value {
    pub fn null() -> Self {
        CoreValue::Null.into()
    }

    pub fn uninitialized() -> Self {
        CoreValue::Uninitialized.into()
    }

    pub fn new(
        inner: impl Into<CoreValue>,
        classification: impl Into<ValueClassification>,
    ) -> Self {
        Value {
            inner: inner.into(),
            classification: classification.into(),
        }
    }

    /// Creates a new CoreValue from a native value that implements the [DatexNative] trait.
    /// Since types might be needed to get resolved for entity values, the cache is required.
    pub fn native<T: DatexNative>(
        value: T,
        cache: &mut SharedReferencesCache,
    ) -> Value {
        let classification = value.classification(cache);
        Value::new(CoreValue::native(value), classification)
    }

    /// Creates a new CoreValue from a native value that implements the [DatexNative] trait.
    /// Since types might be needed to get resolved for entity values, the cache is required.
    pub fn native_boxed<T: DatexNative>(
        value: Box<T>,
        cache: &mut SharedReferencesCache,
    ) -> Value {
        let classification = value.classification(cache);
        Value::new(CoreValue::native_boxed(value), classification)
    }

    /// Creates a new CoreValue from a native value that implements the [DatexNative] trait.
    /// Since types might be needed to get resolved for entity values, the cache is required.
    pub fn native_dyn(
        value: Box<dyn DatexNative>,
        cache: &mut SharedReferencesCache,
    ) -> Value {
        let classification = value.classification(cache);
        Value::new(CoreValue::native_boxed(value), classification)
    }

    /// Creates a new CoreValue from a native value that implements the [DatexNativeStructural] trait.
    /// Since the type is required to be completely structural without any entity references,
    /// no cache is needed to resolve types.
    pub fn native_structural<T: DatexNativeStructural>(value: T) -> Value {
        let classification = value.classification_without_cache();
        Value::new(CoreValue::native(value), classification)
    }

    /// Creates a new CoreValue from a native value that implements the [DatexNativeStructural] trait.
    /// Since the type is required to be completely structural without any entity references,
    /// no cache is needed to resolve types.
    pub fn native_structural_boxed<T: DatexNativeStructural>(
        value: Box<T>,
    ) -> Value {
        let classification = value.classification_without_cache();
        Value::new(CoreValue::native_boxed(value), classification)
    }

    pub fn classification(&self) -> &ValueClassification {
        &self.classification
    }
    pub fn into_inner(self) -> CoreValue {
        self.inner
    }
    pub fn is_uninitialized(&self) -> bool {
        matches!(&self.inner, CoreValue::Uninitialized)
    }

    /// Returns a reference to the inner [CoreValue] of the [Value].
    /// If the inner [CoreValue] is a [CoreValue::Native], it is first collapsed to a DATEX value.
    pub fn inner_non_native(
        &self,
        _cache: &mut SharedReferencesCache,
    ) -> Result<Cow<'_, CoreValue>, ()> {
        Ok(Cow::Borrowed(&self.inner)) // workaround
        // TODO: implement try_borrowed_boxed_to_value
        // match &self.inner {
        //     CoreValue::Native(native) => Ok(
        //         Cow::Owned(native.value.try_boxed_to_value(cache)?.inner)
        //     ),
        //     _ => Ok(Cow::Borrowed(&self.inner)),
        // }
    }

    /// Strips any local observers from the given value container.
    /// This method should be called when a value is moved from its [SharedContainer] parent.
    pub fn without_local_observers(mut self) -> Value {
        self.set_update_callback_data(None);
        self
    }

    /// Creates a new Value representing a boxed value.
    /// This can be used to wrap a [ValueContainer] directly into a local [Value],
    /// e.g. for #Tagged(shared X) or (X | null) | null
    pub fn boxed(value: impl Into<ValueContainer>) -> Self {
        Value::from(CoreValue::Box(Box::new(value.into())))
    }
    pub fn unbox(self) -> Result<ValueContainer, Value> {
        match self.inner {
            CoreValue::Box(boxed) => Ok(*boxed),
            _ => Err(self),
        }
    }
}

impl Value {
    pub fn callable(
        name: Option<String>,
        signature: CallableTypeDefinition,
        body: CallableBody,
        creator: Endpoint,
    ) -> Self {
        Value::new(
            CoreValue::Callable(Callable {
                name,
                signature,
                body,
                creator,
            }),
            ValueClassification::None,
        )
    }

    pub fn is_null(&self) -> bool {
        core::matches!(self.inner, CoreValue::Null)
    }

    /// Tries to get a borrow of the current value as the specified type.
    /// Does not perform any type conversion.
    pub fn try_as<T>(&self) -> Option<&T>
    where
        T: ConvertCoreValue + StaticClassification,
    {
        match validate_classification::<T>(self) {
            Ok(_) => T::try_borrow_from_core_value(&self.inner).ok(),
            Err(_) => None,
        }
    }

    pub fn try_as_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ConvertCoreValue + StaticClassification,
    {
        match validate_classification::<T>(self) {
            Ok(_) => T::try_borrow_mut_from_core_value(&mut self.inner).ok(),
            Err(_) => None,
        }
    }

    /// Tries to convert the current value into the specific specified type.
    /// Does not perform any type conversion.
    pub fn try_into_value<T>(self) -> Result<T, Value>
    where
        T: ConvertCoreValue + StaticClassification,
    {
        match validate_classification::<T>(&self) {
            Ok(_) => {
                T::try_from_core_value(self.inner).map_err(|inner| Value {
                    inner,
                    classification: self.classification,
                })
            }
            Err(_) => Err(self),
        }
    }

    /// Returns the actual current [TypeDefinition] of the value
    pub fn actual_type(&self) -> TypeDefinition {
        match &self.classification {
            ValueClassification::Entity(entity_type) => {
                TypeDefinition::Box(Box::new(Type::Entity(entity_type.clone())))
            }
            ValueClassification::Tag(ValueTag { tag, is_empty }) => {
                TypeDefinition::TaggedType(TaggedTypeDefinition {
                    tag: tag.clone(),
                    ty: if *is_empty {
                        None
                    } else {
                        Some(Box::new(Type::core(self.default_core_type())))
                    },
                })
            }
            ValueClassification::Impls(_impls) => todo!(),
            ValueClassification::None => {
                TypeDefinition::CoreType(self.default_core_type())
            }
        }
    }

    /// Gets a property on the value if applicable (e.g. for map and structs)
    pub fn try_get_property<'a>(
        &self,
        key: impl Into<BorrowedValueKey<'a>>,
        cache: &mut SharedReferencesCache,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        <Self as ValueAccess>::try_get_property(self, key.into(), cache)
    }

    pub fn try_get_property_mut<'a>(
        &mut self,
        key: impl Into<BorrowedValueKey<'a>>,
        cache: &mut SharedReferencesCache,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        <Self as ValueAccess>::try_get_property_mut(self, key.into(), cache)
    }

    /// Takes (removes) a property from the value if applicable (e.g. for map and structs)
    pub fn try_take_property<'a>(
        &mut self,
        key: impl Into<BorrowedValueKey<'a>>,
        _cache: &mut SharedReferencesCache,
    ) -> Result<ValueContainer, AccessError> {
        // TODO
        match self.inner {
            CoreValue::Map(ref mut map) => {
                // If the value is a map, get the property
                Ok(map.try_delete(key)?)
            }
            CoreValue::List(ref mut list) => {
                if let Some(index) = key.into().try_as_index() {
                    Ok(list.try_delete(index)?)
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            CoreValue::Text(ref text) => {
                if let Some(index) = key.into().try_as_index() {
                    let char = text.char_at(index)?;
                    Ok(ValueContainer::from(char.to_string()))
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            _ => {
                // If the value is not an map, we cannot get a property
                Err(AccessError::InvalidOperation(
                    "Cannot get property".to_string(),
                ))
            }
        }
    }

    pub fn try_delete_property<'a>(
        &mut self,
        key: impl Into<BorrowedValueKey<'a>>,
    ) -> Result<(), AccessError> {
        match self.inner {
            CoreValue::Map(ref mut map) => {
                // If the value is a map, delete the property
                map.try_delete(key)?;
                Ok(())
            }
            CoreValue::List(ref mut list) => {
                if let Some(index) = key.into().try_as_index() {
                    list.try_delete(index)?;
                    Ok(())
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            CoreValue::Text(_) => Err(AccessError::InvalidOperation(
                "Cannot delete property on text".to_string(),
            )),
            _ => {
                // If the value is not a map, we cannot delete a property
                Err(AccessError::InvalidOperation(
                    "Cannot delete property".to_string(),
                ))
            }
        }
    }

    /// Sets a property on the value if applicable (e.g. for maps)
    pub fn try_set_property<'a>(
        &mut self,
        key: impl Into<BorrowedValueKey<'a>>,
        val: ValueContainer,
    ) -> Result<(), AccessError> {
        let key = key.into();

        match self.inner {
            CoreValue::Map(ref mut map) => {
                // If the value is an map, set the property
                map.try_set(key, val)?;
            }
            CoreValue::List(ref mut list) => {
                if let Some(index) = key.try_as_index() {
                    list.try_set(index, val)
                        .map_err(AccessError::IndexOutOfBounds)?;
                } else {
                    return Err(AccessError::InvalidIndexKey);
                }
            }
            CoreValue::Text(ref mut text) => {
                if let Some(index) = key.try_as_index() {
                    if let ValueContainer::Local(v) = &val
                        && let CoreValue::Text(new_char) = &v.inner
                        && new_char.0.len() == 1
                    {
                        let char = new_char.0.chars().next().unwrap_or('\0');
                        text.set_char_at(index, char).map_err(|err| {
                            AccessError::IndexOutOfBounds(err)
                        })?;
                    } else {
                        return Err(AccessError::InvalidOperation(
                            "Can only set char character in text".to_string(),
                        ));
                    }
                } else {
                    return Err(AccessError::InvalidIndexKey);
                }
            }
            _ => {
                // If the value is not a map, we cannot set a property
                return Err(AccessError::InvalidOperation(format!(
                    "Cannot set property '{}' on non-map value: {:?}",
                    key, self
                )));
            }
        }

        Ok(())
    }
}

impl_display_for_datex_value!(
    Value,
    impl core::fmt::Display for Value {
        fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
            core::write!(f, "{}", self.inner)
        }
    }
);

#[cfg(test)]
/// Tests for the Value struct and its methods.
/// This module contains unit tests for the Value struct, including its methods and operations.
/// The value is a holder for a combination of a CoreValue representation and its actual type.
mod tests {
    use super::*;
    use crate::{
        libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
        prelude::*,
        traits::structural_eq::assert_structural_eq,
        types::{r#type::Type, type_definition::impl_type::ImplTypeDefinition},
        values::core_values::{
            endpoint::Endpoint,
            integer::{Integer, typed_integer::TypedInteger},
            list::{List, datex_list},
        },
    };
    use core::{assert_matches, str::FromStr};
    use log::info;

    #[test]
    fn endpoint() {
        let endpoint = Value::from(Endpoint::from_str("@test").unwrap());
        assert_eq!(endpoint.to_string(), "@test");
    }

    #[test]
    fn new_addition_assignments() {
        let mut x = Value::from(42i8);
        let y = Value::from(27i8);

        x += y.clone();
        assert_eq!(x, Value::from(69i8));
    }

    #[test]
    fn new_additions() {
        let x = Value::from(42i8);
        let y = Value::from(27i8);

        let z = (x.clone() + y.clone()).unwrap();
        assert_eq!(z, Value::from(69i8));
    }

    #[test]
    fn list() {
        let mut a = List::from(vec![
            Value::from("42"),
            Value::from(42),
            Value::from(true),
        ]);

        a.push(Value::from(42));
        a.push(4);

        assert_eq!(a.len(), 5);

        let b = List::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.len(), 11);

        let c = datex_list![1, "test", 3, true, false];
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".into());
        assert_eq!(c[2], 3.into());
    }

    #[test]
    fn boolean() {
        let a = Value::from(true);
        let b = Value::from(false);
        let c = Value::from(false);
        assert_ne!(a, b);
        assert_eq!(b, c);

        let d = (!b.clone()).unwrap();
        assert_eq!(a, d);

        // We can't add two booleans together, so this should return None
        let a_plus_b = a.clone() + b.clone();
        assert!(a_plus_b.is_err());
    }

    #[test]
    fn equality_same_type() {
        let a = Value::from(42i8);
        let b = Value::from(42i8);
        let c = Value::from(27i8);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);

        info!("{} === {}", a.clone(), b.clone());
        info!("{} !== {}", a.clone(), c.clone());
    }

    #[test]
    fn decimal() {
        let a = Value::from(42.1f32);
        let b = Value::from(27f32);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b, Value::from(69.1f32));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn null() {
        let null_value = Value::null();
        assert_eq!(null_value.to_string(), "null");

        let maybe_value: Option<i8> = None;
        let null_value = Value::from(maybe_value);
        assert_eq!(null_value.to_string(), "null");
        assert!(null_value.is_null());
    }

    #[test]
    fn addition() {
        let a = Value::from(42i8);
        let b = Value::from(27i8);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b, Value::from(69i8));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
    }

    #[test]
    fn string_concatenation() {
        let a = Value::from("Hello ");
        let b = Value::from(TypedInteger::I8(42i8));
        assert!(matches!(a.inner, CoreValue::Text(_)));
        assert!(matches!(
            b.try_as::<TypedInteger>().unwrap(),
            TypedInteger::I8(_)
        ));

        let a_plus_b = (&a + &b).unwrap();
        let b_plus_a = (&b + &a).unwrap();

        assert!(matches!(a_plus_b.inner, CoreValue::Text(_)));
        assert!(matches!(b_plus_a.inner, CoreValue::Text(_)));

        assert_eq!(a_plus_b, Value::from("Hello 42"));
        assert_eq!(b_plus_a, Value::from("42Hello "));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }

    #[test]
    fn structural_equality() {
        let a = Value::from(42_i8);
        let b = Value::from(42_i32);
        assert_matches!(a.inner, CoreValue::TypedInteger(TypedInteger::I8(_)));
        assert_matches!(b.inner, CoreValue::TypedInteger(TypedInteger::I32(_)));
        assert_ne!(a, b);

        assert_structural_eq!(a, b);

        assert_structural_eq!(
            Value::from(TypedInteger::I8(42)),
            Value::from(TypedInteger::U32(42)),
        );

        assert_structural_eq!(
            Value::from(42_i8),
            Value::from(Integer::from(42_i8))
        );
    }
}
