//! This module contains the implementation of the [Value] struct, which represents a value in the DATEX type system.
//! A [Value] consists of a [CoreValue] representation and an optional custom type.
use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::convert_value::ConvertValue,
    types::type_definition::{
        TypeDefinition, callable::CallableTypeDefinition,
    },
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        core_values::{
            callable::{Callable, CallableBody},
            native::DatexNative,
        },
        value_container::{ValueContainer},
    },
};
mod apply;
pub mod borrowed_value;
mod classification;
mod convert_parts;
mod convert_value;
mod datex_hash;
mod datex_native;
mod equality;
mod get_core_lib_type_id;
mod get_datex_type;
mod iter_parts;
mod local_child_path_resolver;
mod ops;
mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
mod update_handler;
mod value_access;
pub mod value_classification;

use crate::{
    shared_values::errors::AccessError,
    traits::{
        classification::Classification,
        convert_value_container::ConvertValueContainer,
        datex_native_only_structural::DatexNativeOnlyStructural,
        value_access::ValueAccess,
    },
    utils::impl_display_for_datex_value::impl_display_for_datex_value,
    value_updates::update_handler::InternalMutabilityUpdateHandler,
    values::{
        core_values::{endpoint::Endpoint, text::Text, native::NativeCoreValue},
        value::value_classification::{
            ValueClassification,
            unresolved_value_classification::UnresolvedValueClassification,
        },
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};
use core::{
    fmt::{Debug, Formatter},
    result::Result,
};

mod try_clone;

#[derive(Debug, Clone)]
/// Represents a local DATEX value.
/// This can either be a core value with classification or a
/// native rust value that provides all trait implementations.
pub enum Value {
    /// Core value with classification
    Core(CoreValueWithClassification),
    /// Native rust value with DATEX representation
    Native(NativeCoreValue),
}

impl<T: ConvertValue> From<T> for Value {
    fn from(inner: T) -> Self {
        inner.to_value()
    }
}

impl From<&'static str> for Value {
    fn from(value: &'static str) -> Self {
        Value::core(CoreValue::Text(Text::StaticStr(value)))
    }
}

impl Value {
    /// Creates a new [Value] from a native value that implements the [DatexNative] trait.
    pub fn native(value: impl DatexNative) -> Value {
        Value::Native(NativeCoreValue::new(value))
    }
    pub fn native_boxed(value: Box<dyn DatexNative>) -> Value {
        Value::Native(NativeCoreValue { value })
    }

    pub fn core(inner: impl Into<CoreValueWithClassification>) -> Self {
        Value::Core(inner.into())
    }

    pub fn core_with_classification(
        inner: impl Into<CoreValue>,
        classification: ValueClassification,
    ) -> Self {
        Value::Core(CoreValueWithClassification {
            inner: inner.into(),
            classification,
        })
    }

    pub fn is_collection_value(&self) -> bool {
        match self {
            Value::Core(core_value_with_classification) => {
                core_value_with_classification.is_collection_value()
            }
            Value::Native(_) => false,
        }
    }

    pub fn null() -> Self {
        CoreValue::Null.into()
    }

    pub fn uninitialized() -> Self {
        CoreValue::Uninitialized.into()
    }

    /// Returns the classification of the value, which includes its entity type, impls, and tag.
    pub fn classification(
        &self,
        cache: &mut SharedReferencesCache,
    ) -> ValueClassification {
        match self {
            Value::Core(core_value_with_classification) => {
                core_value_with_classification.classification.clone()
            }
            Value::Native(native) => native.classification(cache),
        }
    }

    /// Returns the unresolved classification of the value, which includes its entity type, impls, and tag.
    /// In contrast to `classification`, this method does not require a mutable reference to a cache
    /// and only returns an address for an entity type if it is present, without resolving it to a full type.
    pub fn unresolved_classification(&self) -> UnresolvedValueClassification {
        match self {
            Value::Core(core_value_with_classification) => {
                core_value_with_classification.classification.clone().into()
            }
            Value::Native(native) => native.unresolved_classification(),
        }
    }

    pub fn new(inner: impl ConvertValue) -> Self {
        inner.to_value()
    }

    /// Checks if the inner [CoreValue] of the [Value] is a [CoreValue::Native].
    pub fn is_native(&self) -> bool {
        matches!(&self, Value::Native(_))
    }

    pub fn is_uninitialized(&self) -> bool {
        match self {
            Value::Core(core_value_with_classification) => {
                core_value_with_classification.is_uninitialized()
            }
            Value::Native(_) => false,
        }
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
        match self {
            Value::Core(core_value_with_classification) => {
                match core_value_with_classification.inner {
                    CoreValue::Box(boxed) => Ok(*boxed),
                    _ => Err(Value::Core(core_value_with_classification)),
                }
            }
            _ => Err(self),
        }
    }

    /// Tries to get a borrow of the current value as the specified type.
    /// Does not perform any type conversion.
    pub fn try_as<T: ?Sized>(&self) -> Option<&T>
    where
        T: ConvertValue,
    {
        T::try_borrow_from_value(self).ok()
    }

    pub fn try_as_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ConvertValue,
    {
        T::try_borrow_mut_from_value(self).ok()
    }

    /// Tries to convert the current value into the specific specified type.
    /// Does not perform any type conversion.
    pub fn try_into_value<T>(self) -> Result<T, Value>
    where
        T: ConvertValue,
    {
        T::try_from_value(self)
    }

    /// Tries to get a reference to the inner [CoreValue] if the [Value] is a [Value::Core] variant.
    pub fn try_as_core_value(&self) -> Option<&CoreValue> {
        match self {
            Value::Core(core_value_with_classification) => {
                Some(&core_value_with_classification.inner)
            }
            Value::Native(_native) => None,
        }
    }

    /// Tries to get a mutable reference to the inner [CoreValue] if the [Value] is a [Value::Core] variant.
    pub fn try_as_core_value_mut(&mut self) -> Option<&mut CoreValue> {
        match self {
            Value::Core(core_value_with_classification) => {
                Some(&mut core_value_with_classification.inner)
            }
            Value::Native(_native) => None,
        }
    }

    /// Tries to convert the current value into a [CoreValue] if the [Value] is a [Value::Core] variant.
    pub fn try_into_core_value(self) -> Result<CoreValue, Value> {
        match self {
            Value::Core(core_value_with_classification) => {
                Ok(core_value_with_classification.inner)
            }
            Value::Native(native) => Err(Value::Native(native)),
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
        Value::new(CoreValue::Callable(Callable {
            name,
            signature,
            body,
            creator,
        }))
    }

    pub fn is_null(&self) -> bool {
        match self {
            Value::Core(core_value_with_classification) => {
                core_value_with_classification.is_null()
            }
            Value::Native(_native_core_value) => false, // TODO: handle native null values
        }
    }

    /// Tries to downcast the CoreValue to a native value of type T.
    /// This method will return Some(Box<T>) if the CoreValue is a Native variant and the underlying value can be downcast to T.
    pub fn downcast_native<T: DatexNative>(self) -> Option<Box<T>> {
        if let Value::Native(native_value) = self {
            native_value.into_any().downcast::<T>().ok()
        } else {
            None
        }
    }

    /// Tries to downcast the CoreValue to a reference of a native value of type T.
    /// This method will return Some(&T) if the CoreValue is a Native variant and the underlying value can be downcast to T.
    pub fn downcast_native_ref<T: DatexNative>(&self) -> Option<&T> {
        if let Value::Native(native_value) = self {
            native_value.as_any().downcast_ref::<T>()
        } else {
            None
        }
    }

    /// Tries to downcast the CoreValue to a mutable reference of a native value of type T.
    /// This method will return Some(&mut T) if the CoreValue is a Native variant and the underlying value can be downcast to T.
    pub fn downcast_native_mut<T: DatexNative>(&mut self) -> Option<&mut T> {
        if let Value::Native(native_value) = self {
            native_value.as_any_mut().downcast_mut::<T>()
        } else {
            None
        }
    }

    /// Gets a property on the value if applicable (e.g. for map and structs)
    pub fn try_get_property<'a>(
        &self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        <Self as ValueAccess>::try_get_property(self, key)
    }

    pub fn try_get_property_mut<'a>(
        &mut self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        <Self as ValueAccess>::try_get_property_mut(self, key)
    }

    /// Returns the actual current [TypeDefinition] of the value
    pub fn actual_type(&self) -> TypeDefinition {
        match self {
            Value::Core(core_value_with_classification) => {
                core_value_with_classification.actual_type()
            }
            Value::Native(_native) => todo!(),
        }
    }
}

impl_display_for_datex_value!(
    Value,
    impl core::fmt::Display for Value {
        fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
            match self {
                Value::Core(core_value_with_classification) => {
                    core::write!(f, "{}", core_value_with_classification)
                }
                Value::Native(native) => {
                    core::write!(f, "{}", native)
                }
            }
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
        types::{r#type::Type, type_definition::impl_type::ImplMarkers},
        values::core_values::{
            endpoint::Endpoint,
            integer::{Integer, typed_integer::TypedInteger},
            list::{List, datex_list},
        },
    };
    use core::assert_matches;
    use log::info;

    #[test]
    fn endpoint() {
        let endpoint = Value::from(Endpoint::new("@test"));
        assert!(!endpoint.is_native());

        let endpoint_value = endpoint.try_into_value::<Endpoint>().unwrap();
        assert_eq!(endpoint_value.to_string(), "@test");

        let endpoint = Value::from(CoreValue::Endpoint(Endpoint::new("@test")));
        assert!(!endpoint.is_native());
        assert_eq!(
            endpoint.try_into_value::<Endpoint>().unwrap().to_string(),
            "@test"
        );
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
            Value::from("42".to_string()),
            Value::from(42),
            Value::from(true),
        ]);

        a.push(Value::from(42));
        a.push(4);

        assert_eq!(a.len(), 5);

        let b = List::from(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(b.len(), 11);

        let c = datex_list![1, "test".to_string(), 3, true, false];
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], 1.into());
        assert_eq!(c[1], "test".to_string().into());
        assert_eq!(c[2], 3.into());
    }

    #[test]
    fn boolean() {
        let a = Value::from(CoreValue::Boolean(true.into()));
        let b = Value::from(CoreValue::Boolean(false.into()));
        let c = Value::from(CoreValue::Boolean(false.into()));
        assert_ne!(a, b);
        assert_eq!(b, c);

        let d = (!&b).unwrap();
        assert_eq!(a, d);

        // We can't add two booleans together, so this should return None
        let a_plus_b = &a + &b;
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
        assert_eq!(*null_value.try_as::<Option<i8>>().unwrap(), None);
        assert_eq!(null_value.try_into_value::<Option<i8>>().unwrap(), None);
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
        let a = Value::from("Hello ".to_string());
        let b = Value::from(TypedInteger::I8(42i8));
        assert!(matches!(a.try_as_core_value().unwrap(), CoreValue::Text(_)));
        assert!(matches!(
            b.try_as::<TypedInteger>().unwrap(),
            TypedInteger::I8(_)
        ));

        let a_plus_b = (&a + &b).unwrap();
        let b_plus_a = (&b + &a).unwrap();

        assert!(matches!(
            a_plus_b.try_as_core_value().unwrap(),
            CoreValue::Text(_)
        ));
        assert!(matches!(
            b_plus_a.try_as_core_value().unwrap(),
            CoreValue::Text(_)
        ));

        assert_eq!(a_plus_b, Value::from("Hello 42".to_string()));
        assert_eq!(b_plus_a, Value::from("42Hello ".to_string()));

        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b);
        info!("{} + {} = {}", b.clone(), a.clone(), b_plus_a);
    }

    #[test]
    fn structural_equality() {
        let a = Value::from(42_i8);
        let b = Value::from(42_i32);
        assert_matches!(
            a.try_as_core_value().unwrap(),
            CoreValue::TypedInteger(TypedInteger::I8(_))
        );
        assert_matches!(
            b.try_as_core_value().unwrap(),
            CoreValue::TypedInteger(TypedInteger::I32(_))
        );
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
