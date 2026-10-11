//! Implements [TryFrom] for DATEX [CoreValue] and [Value] types. This allows to convert e.g. [CoreValue::Integer] to [Integer].
use crate::{
    traits::convert_value::ConvertValue,
    types::{
        entities::entity_type_definition::EntityTypeDefinition, r#type::Type,
    },
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::{
            CoreValue,
            borrowed_core_value::{BorrowedCoreValue, BorrowedCoreValueMut},
        },
        core_value_with_classification::{
            CoreValueWithClassification,
            borrowed_core_value_with_classification::{
                BorrowedCoreValueWithClassification,
                BorrowedCoreValueWithClassificationMut,
            },
        },
        core_values::{
            boolean::Boolean,
            callable::Callable,
            decimal::{Decimal, typed_decimal::TypedDecimal},
            endpoint::Endpoint,
            instant::Instant,
            integer::{Integer, typed_integer::TypedInteger},
            list::List,
            map::Map,
            range::Range,
            text::Text,
        },
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    },
};

/// Implements [TryFrom] for each [CoreValue] variant to its corresponding type.
/// This allows to convert e.g. [CoreValue::Integer] to [Integer].
/// This also allows to borrow or mutably borrow the corresponding type from a [CoreValue].
/// Implementation for the try from core value methods is also done for [CoreValue::Native] branch.
macro impl_convert_value($($variant:ident => $type:ty),* $(,)?) {
    $(
        impl ConvertValue for $type {
            fn to_value(self) -> Value {
                Value::Core(CoreValueWithClassification::new(CoreValue::$variant(self)))
            }

            fn try_from_value(value: Value) -> Result<Self, Value> {
                match value {
                    Value::Native(native) => {
                        match native.try_into_value() {
                            Ok(v) => Ok(v),
                            Err(native) => Err(Value::Native(native)),
                        }
                    },
                    Value::Core(CoreValueWithClassification {inner: CoreValue::$variant(v), ..}) => Ok(v),
                    _ => Err(value),
                }
            }

            fn try_borrow_from_value(value: &Value) -> Result<&$type, ()> {
                match value {
                    Value::Core(CoreValueWithClassification {inner: CoreValue::$variant(v), ..}) => Ok(v),
                    Value::Native(native) => {
                        native.try_as::<$type>().ok_or(())
                    },
                    _ => Err(()),
                }
            }

            fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut $type, ()> {
                match value {
                    Value::Core(CoreValueWithClassification {inner: CoreValue::$variant(v), ..}) => Ok(v),
                    Value::Native(native) => {
                        native.try_as_mut::<$type>().ok_or(())
                    },
                    _ => Err(()),
                }
            }
        }
    )*
}

macro impl_try_from_core_value($($variant:ident => $type:ty),* $(,)?) {
    $(
        impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, $type> {
            type Error = BorrowedValue<'a>;
            fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    // BorrowedCoreValue::$variant(v) => Ok(v),
                    BorrowedValue::Core(BorrowedCoreValueWithClassification {
                        inner: BorrowedCoreValue::$variant(v),
                        ..
                    }) => Ok(v),
                    _ => Err(value),
                }
            }
        }

        impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, $type> {
            type Error = BorrowedValueMut<'a>;
            fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
                match value {
                    BorrowedValueMut::Core(
                        BorrowedCoreValueWithClassificationMut {
                            inner: BorrowedCoreValueMut::$variant(v),
                            ..
                        },
                    ) => Ok(v),
                    _ => Err(value),
                }
            }
        }
    )*
}

impl_try_from_core_value! {
    Integer             => Integer,
    TypedInteger        => TypedInteger,
    Decimal             => Decimal,
    TypedDecimal        => TypedDecimal,
    Boolean             => Boolean,
    Endpoint            => Endpoint,
    List                => List,
    Map                 => Map,
    Type                => Type,
    EntityTypeDefinition => EntityTypeDefinition,
    Range               => Range,
    Callable            => Callable,
    Instant              => Instant,
}

impl_convert_value! {
    Integer             => Integer,
    TypedInteger        => TypedInteger,
    Decimal             => Decimal,
    TypedDecimal        => TypedDecimal,
    Boolean             => Boolean,
    Endpoint            => Endpoint,
    List                => List,
    Map                 => Map,
    Text                => Text,
    Type                => Type,
    EntityTypeDefinition => EntityTypeDefinition,
    Range               => Range,
    Callable            => Callable,
    Instant              => Instant,
}

#[cfg(test)]
mod tests {
    use core::assert_matches;

    use crate::{
        prelude::*,
        traits::convert_value::ConvertValue,
        values::{
            core_value::CoreValue,
            core_values::{
                endpoint::Endpoint, integer::Integer, native::NativeCoreValue,
                text::Text,
            },
            value::Value,
        },
    };

    #[test]
    fn try_from_value() {
        let int_value = CoreValue::Integer(Integer::new(42)).to_value();
        let int: Integer = int_value.try_into_value().unwrap();
        assert_eq!(int, Integer::new(42));

        let text_value = CoreValue::Text(Text::new("Hello, DATEX!")).to_value();
        let text: Text = text_value.try_into_value().unwrap();
        assert_eq!(text, Text::new("Hello, DATEX!"));
    }

    #[test]
    fn try_from_core_value_wrong_type() {
        let int_value = CoreValue::Integer(Integer::new(42)).to_value();
        let result = int_value.try_into_value::<Text>();
        assert_matches!(result, Err(_));
    }

    #[test]
    fn try_from_core_value_ref() {
        let int_value = CoreValue::Integer(Integer::new(42)).to_value();
        let int_ref = int_value.try_as::<Integer>().unwrap();
        assert_eq!(*int_ref, Integer::new(42));

        let text_value = CoreValue::Text(Text::new("Hello, DATEX!")).to_value();
        let text_ref: &Text = text_value.try_as().unwrap();
        assert_eq!(*text_ref, Text::new("Hello, DATEX!"));
    }

    #[test]
    fn try_from_core_value_mut_ref() {
        let mut int_value = CoreValue::Integer(Integer::new(42)).to_value();
        let int_mut_ref = int_value.try_as_mut::<Integer>().unwrap();
        *int_mut_ref = Integer::new(100);
        assert_eq!(*int_mut_ref, Integer::new(100));
        assert_eq!(int_value, CoreValue::Integer(Integer::new(100)).to_value());
    }

    #[test]
    fn try_into_value_value() {
        let value = Value::from(CoreValue::Integer(Integer::new(42)));
        let int: Integer = value.try_into_value().unwrap();
        assert_eq!(int, Integer::new(42));
    }

    #[test]
    fn try_into_value_native() {
        let native_endpoint = Value::from(Endpoint::new("@test"));
        let endpoint: Endpoint = native_endpoint.try_into_value().unwrap();
        assert_eq!(endpoint.to_string(), "@test");
    }
    #[test]
    fn try_borrow_from_core_value_native() {
        let mut native_endpoint =
            Value::Native(NativeCoreValue::new(Endpoint::new("@test")));
        let endpoint_ref: &Endpoint =
            Endpoint::try_borrow_from_value(&native_endpoint).unwrap();
        assert_eq!(*endpoint_ref, Endpoint::new("@test"));

        let endpoint_mut_ref: &mut Endpoint =
            Endpoint::try_borrow_mut_from_value(&mut native_endpoint).unwrap();
        *endpoint_mut_ref = Endpoint::new("@test2");
        assert_eq!(*endpoint_mut_ref, Endpoint::new("@test2"));

        assert_eq!(
            native_endpoint,
            Value::Native(NativeCoreValue::new(Endpoint::new("@test2")))
        );
    }
}
