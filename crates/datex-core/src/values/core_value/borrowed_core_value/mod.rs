use crate::{
    prelude::*,
    types::{
        entities::entity_type_definition::EntityTypeDefinition, r#type::Type,
    },
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
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
        },
        value::{
            borrowed_value::BorrowedValue,
            value_classification::ValueClassification,
        },
        value_container::ValueContainer,
    },
};
pub mod equality;
pub mod hash;
#[cfg(feature = "ast")]
mod to_datex_expression_data;

use core::ops::Deref;
use crate::values::core_value_with_classification::borrowed_core_value_with_classification::BorrowedCoreValueWithClassification;

/// Similar to [CoreValue], but it is a potentially borrowed reference to a [CoreValue] variant instead of owning it.
#[derive(Debug, Default)]
pub enum BorrowedCoreValue<'a> {
    #[default]
    Uninitialized,
    Null,
    Boolean(Goat<'a, Boolean>),
    Integer(Goat<'a, Integer>),
    TypedInteger(Goat<'a, TypedInteger>),
    Decimal(Goat<'a, Decimal>),
    TypedDecimal(Goat<'a, TypedDecimal>),
    Text(Goat<'a, str>),
    Endpoint(Goat<'a, Endpoint>),
    List(Goat<'a, List>),
    Map(Goat<'a, Map>),
    Type(Goat<'a, Type>),
    EntityTypeDefinition(Goat<'a, EntityTypeDefinition>),
    Callable(Goat<'a, Callable>),
    Range(Goat<'a, Range>),
    Box(Goat<'a, Box<ValueContainer>>),
    Instant(Goat<'a, Instant>),
}

impl<'a> BorrowedCoreValue<'a> {
    pub(crate) fn clone_to_core_value(&self) -> CoreValue {
        match self {
            BorrowedCoreValue::Uninitialized => CoreValue::Uninitialized,
            BorrowedCoreValue::Null => CoreValue::Null,
            BorrowedCoreValue::Boolean(boolean) => {
                CoreValue::Boolean(boolean.deref().clone())
            }
            BorrowedCoreValue::Integer(integer) => {
                CoreValue::Integer(integer.deref().clone())
            }
            BorrowedCoreValue::TypedInteger(typed_integer) => {
                CoreValue::TypedInteger(typed_integer.deref().clone())
            }
            BorrowedCoreValue::Decimal(decimal) => {
                CoreValue::Decimal(decimal.deref().clone())
            }
            BorrowedCoreValue::TypedDecimal(typed_decimal) => {
                CoreValue::TypedDecimal(typed_decimal.deref().clone())
            }
            BorrowedCoreValue::Text(text) => {
                CoreValue::Text(text.to_string().into())
            }
            BorrowedCoreValue::Endpoint(endpoint) => {
                CoreValue::Endpoint(endpoint.deref().clone())
            }
            BorrowedCoreValue::List(list) => {
                CoreValue::List(list.deref().clone())
            }
            BorrowedCoreValue::Map(map) => CoreValue::Map(map.deref().clone()),
            BorrowedCoreValue::Type(type_value) => {
                CoreValue::Type(type_value.deref().clone())
            }
            BorrowedCoreValue::EntityTypeDefinition(entity_type_definition) => {
                CoreValue::EntityTypeDefinition(
                    entity_type_definition.deref().clone(),
                )
            }
            BorrowedCoreValue::Callable(callable) => {
                CoreValue::Callable(callable.deref().clone())
            }
            BorrowedCoreValue::Range(range) => {
                CoreValue::Range(range.deref().clone())
            }
            BorrowedCoreValue::Box(boxed_value) => {
                CoreValue::Box(boxed_value.deref().clone())
            }
            BorrowedCoreValue::Instant(instant) => {
                CoreValue::Instant(*instant.deref())
            }
        }
    }
}

impl<'a> From<&'a CoreValue> for BorrowedCoreValue<'a> {
    fn from(core_value: &'a CoreValue) -> Self {
        match core_value {
            CoreValue::Callable(callable) => {
                BorrowedCoreValue::Callable(Goat::Borrowed(callable))
            }
            CoreValue::Uninitialized => BorrowedCoreValue::Uninitialized,
            CoreValue::Null => BorrowedCoreValue::Null,
            CoreValue::Boolean(boolean) => {
                BorrowedCoreValue::Boolean(Goat::Borrowed(boolean))
            }
            CoreValue::Integer(integer) => {
                BorrowedCoreValue::Integer(Goat::Borrowed(integer))
            }
            CoreValue::TypedInteger(typed_integer) => {
                BorrowedCoreValue::TypedInteger(Goat::Borrowed(typed_integer))
            }
            CoreValue::Decimal(decimal) => {
                BorrowedCoreValue::Decimal(Goat::Borrowed(decimal))
            }
            CoreValue::TypedDecimal(typed_decimal) => {
                BorrowedCoreValue::TypedDecimal(Goat::Borrowed(typed_decimal))
            }
            CoreValue::Text(text) => {
                BorrowedCoreValue::Text(Goat::Borrowed(&text.as_str()))
            }
            CoreValue::Endpoint(endpoint) => {
                BorrowedCoreValue::Endpoint(Goat::Borrowed(endpoint))
            }
            CoreValue::List(list) => {
                BorrowedCoreValue::List(Goat::Borrowed(list))
            }
            CoreValue::Map(map) => BorrowedCoreValue::Map(Goat::Borrowed(map)),
            CoreValue::Type(type_value) => {
                BorrowedCoreValue::Type(Goat::Borrowed(type_value))
            }
            CoreValue::EntityTypeDefinition(entity_type_definition) => {
                BorrowedCoreValue::EntityTypeDefinition(Goat::Borrowed(
                    entity_type_definition,
                ))
            }
            CoreValue::Range(range) => {
                BorrowedCoreValue::Range(Goat::Borrowed(range))
            }
            CoreValue::Box(boxed_value) => {
                BorrowedCoreValue::Box(Goat::Borrowed(boxed_value))
            }
            CoreValue::Instant(instant) => {
                BorrowedCoreValue::Instant(Goat::Borrowed(instant))
            }
        }
    }
}

impl<'a> From<BorrowedCoreValue<'a>> for BorrowedValue<'a> {
    fn from(borrowed_core_value: BorrowedCoreValue<'a>) -> Self {
        BorrowedValue::Core(BorrowedCoreValueWithClassification {
            inner: borrowed_core_value,
            classification: ValueClassification::new_unclassified(),
        })
    }
}

#[derive(Default, Debug)]
pub enum BorrowedCoreValueMut<'a> {
    #[default]
    Uninitialized,
    Null,
    Boolean(GoatMut<'a, Boolean>),
    Integer(GoatMut<'a, Integer>),
    TypedInteger(GoatMut<'a, TypedInteger>),
    Decimal(GoatMut<'a, Decimal>),
    TypedDecimal(GoatMut<'a, TypedDecimal>),
    Text(GoatMut<'a, str>),
    Endpoint(GoatMut<'a, Endpoint>),
    List(GoatMut<'a, List>),
    Map(GoatMut<'a, Map>),
    Type(GoatMut<'a, Type>),
    EntityTypeDefinition(GoatMut<'a, EntityTypeDefinition>),
    Callable(GoatMut<'a, Callable>),
    Range(GoatMut<'a, Range>),
    Box(GoatMut<'a, Box<ValueContainer>>),
    Instant(GoatMut<'a, Instant>),
}

impl<'a> BorrowedCoreValueMut<'a> {
    pub fn try_clone_to_core_value(self) -> Result<CoreValue, ()> {
        match self {
            BorrowedCoreValueMut::Uninitialized => Ok(CoreValue::Uninitialized),
            BorrowedCoreValueMut::Null => Ok(CoreValue::Null),
            BorrowedCoreValueMut::Boolean(boolean) => {
                Ok(CoreValue::Boolean(boolean.deref().clone()))
            }
            BorrowedCoreValueMut::Integer(integer) => {
                Ok(CoreValue::Integer(integer.deref().clone()))
            }
            BorrowedCoreValueMut::TypedInteger(typed_integer) => {
                Ok(CoreValue::TypedInteger(typed_integer.deref().clone()))
            }
            BorrowedCoreValueMut::Decimal(decimal) => {
                Ok(CoreValue::Decimal(decimal.deref().clone()))
            }
            BorrowedCoreValueMut::TypedDecimal(typed_decimal) => {
                Ok(CoreValue::TypedDecimal(typed_decimal.deref().clone()))
            }
            BorrowedCoreValueMut::Text(text) => {
                Ok(CoreValue::Text(text.to_string().into()))
            }
            BorrowedCoreValueMut::Endpoint(endpoint) => {
                Ok(CoreValue::Endpoint(endpoint.deref().clone()))
            }
            BorrowedCoreValueMut::List(list) => {
                Ok(CoreValue::List(list.deref().clone()))
            }
            BorrowedCoreValueMut::Map(map) => {
                Ok(CoreValue::Map(map.deref().clone()))
            }
            BorrowedCoreValueMut::Type(type_value) => {
                Ok(CoreValue::Type(type_value.deref().clone()))
            }
            BorrowedCoreValueMut::EntityTypeDefinition(
                entity_type_definition,
            ) => Ok(CoreValue::EntityTypeDefinition(
                entity_type_definition.deref().clone(),
            )),
            BorrowedCoreValueMut::Callable(callable) => {
                Ok(CoreValue::Callable(callable.deref().clone()))
            }
            BorrowedCoreValueMut::Range(range) => {
                Ok(CoreValue::Range(range.deref().clone()))
            }
            BorrowedCoreValueMut::Box(boxed_value) => {
                Ok(CoreValue::Box(boxed_value.deref().clone()))
            }
            BorrowedCoreValueMut::Instant(instant) => {
                Ok(CoreValue::Instant(*instant.deref()))
            }
        }
    }
}

impl<'a> From<&'a mut CoreValue> for BorrowedCoreValueMut<'a> {
    fn from(core_value: &'a mut CoreValue) -> Self {
        match core_value {
            CoreValue::Callable(callable) => {
                BorrowedCoreValueMut::Callable(GoatMut::Borrowed(callable))
            }
            CoreValue::Uninitialized => BorrowedCoreValueMut::Uninitialized,
            CoreValue::Null => BorrowedCoreValueMut::Null,
            CoreValue::Boolean(boolean) => {
                BorrowedCoreValueMut::Boolean(GoatMut::Borrowed(boolean))
            }
            CoreValue::Integer(integer) => {
                BorrowedCoreValueMut::Integer(GoatMut::Borrowed(integer))
            }
            CoreValue::TypedInteger(typed_integer) => {
                BorrowedCoreValueMut::TypedInteger(GoatMut::Borrowed(
                    typed_integer,
                ))
            }
            CoreValue::Decimal(decimal) => {
                BorrowedCoreValueMut::Decimal(GoatMut::Borrowed(decimal))
            }
            CoreValue::TypedDecimal(typed_decimal) => {
                BorrowedCoreValueMut::TypedDecimal(GoatMut::Borrowed(
                    typed_decimal,
                ))
            }
            CoreValue::Text(text) => {
                BorrowedCoreValueMut::Text(GoatMut::Borrowed(text.try_as_mut_str().expect("fixme")))
            }
            CoreValue::Endpoint(endpoint) => {
                BorrowedCoreValueMut::Endpoint(GoatMut::Borrowed(endpoint))
            }
            CoreValue::List(list) => {
                BorrowedCoreValueMut::List(GoatMut::Borrowed(list))
            }
            CoreValue::Map(map) => {
                BorrowedCoreValueMut::Map(GoatMut::Borrowed(map))
            }
            CoreValue::Type(type_value) => {
                BorrowedCoreValueMut::Type(GoatMut::Borrowed(type_value))
            }
            CoreValue::EntityTypeDefinition(entity_type_definition) => {
                BorrowedCoreValueMut::EntityTypeDefinition(GoatMut::Borrowed(
                    entity_type_definition,
                ))
            }
            CoreValue::Range(range) => {
                BorrowedCoreValueMut::Range(GoatMut::Borrowed(range))
            }
            CoreValue::Box(boxed_value) => {
                BorrowedCoreValueMut::Box(GoatMut::Borrowed(boxed_value))
            }
            CoreValue::Instant(instant) => {
                BorrowedCoreValueMut::Instant(GoatMut::Borrowed(instant))
            }
        }
    }
}

impl<'a> From<&'a mut str> for BorrowedCoreValueMut<'a> {
    fn from(s: &'a mut str) -> Self {
        BorrowedCoreValueMut::Text(GoatMut::Borrowed(s))
    }
}

impl<'a> From<&'a str> for BorrowedCoreValue<'a> {
    fn from(s: &'a str) -> Self {
        BorrowedCoreValue::Text(Goat::Borrowed(s))
    }
}
