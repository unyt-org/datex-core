use core::fmt::Display;

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    traits::convert_value::ConvertValue,
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::Value, value_container::ValueContainer,
    },
};
use serde::{
    Serialize,
    de::Visitor,
    ser::{SerializeMap, SerializeSeq},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValueKey {
    Text(String),
    Index(i64), // FIXME shall we use something else, what is the maximum req. we have for DIF to have here? Do we want unsigned int instead?
    Value(ValueContainer),
}

impl ValueKey {
    pub fn into_value_container(self) -> ValueContainer {
        match self {
            ValueKey::Text(text) => ValueContainer::Local(text.to_value()),
            ValueKey::Index(index) => ValueContainer::Local(index.to_value()),
            ValueKey::Value(value_container) => value_container,
        }
    }
}

impl Display for ValueKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValueKey::Text(text) => core::write!(f, "{}", text),
            ValueKey::Index(index) => core::write!(f, "{}", index),
            ValueKey::Value(value_container) => {
                core::write!(f, "{}", value_container)
            }
        }
    }
}

impl SerializeWithSerdeContext for ValueKey {
    fn serialize_with_ctx<S: serde::Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            ValueKey::Text(text) => text.serialize(serializer),
            ValueKey::Index(index) => index.serialize(serializer),
            ValueKey::Value(value_container) => {
                let mut map_serializer = serializer.serialize_map(Some(1))?;
                map_serializer.serialize_entry(
                    "value",
                    &ValueWithSerdeContext::new(value_container, ctx),
                )?;

                map_serializer.end()
            }
        }
    }
}

impl SerializeWithSerdeContext for &[ValueKey] {
    fn serialize_with_ctx<S: serde::Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq_serializer = serializer.serialize_seq(Some(self.len()))?;
        for key in *self {
            seq_serializer
                .serialize_element(&ValueWithSerdeContext::new(key, ctx))?;
        }
        seq_serializer.end()
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for ValueKey {
    fn deserialize_with_ctx<D: serde::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer
            .deserialize_any(DeserializeSerdeContext::<ValueKey>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, ValueKey>
{
    type Value = ValueKey;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "a value key, which can be a string, an integer index, or a value container"
        )
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(ValueKey::Text(v.to_string()))
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(ValueKey::Index(v))
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(ValueKey::Index(v as i64))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut map_access: A,
    ) -> Result<Self::Value, A::Error> {
        let key = map_access.next_key::<String>()?;
        if let Some(key) = key {
            if key == "value" {
                let value = map_access
                    .next_value_seed(self.cast::<ValueContainer>())?;
                Ok(ValueKey::Value(value))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Unexpected key in map when deserializing ValueKey: expected 'value', found '{}'",
                    key
                )))
            }
        } else {
            Err(serde::de::Error::custom(
                "Expected a map with a single entry when deserializing ValueKey, but found an empty map",
            ))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BorrowedValueKey<'a> {
    Text(Cow<'a, str>),
    Index(i64),
    Value(Cow<'a, ValueContainer>),
}

impl<'a> From<ValueKey> for BorrowedValueKey<'a> {
    fn from(owned: ValueKey) -> Self {
        match owned {
            ValueKey::Text(text) => BorrowedValueKey::Text(Cow::Owned(text)),
            ValueKey::Index(index) => BorrowedValueKey::Index(index),
            ValueKey::Value(value_container) => {
                BorrowedValueKey::Value(Cow::Owned(value_container))
            }
        }
    }
}

impl<'a> From<&'a ValueKey> for BorrowedValueKey<'a> {
    fn from(owned: &'a ValueKey) -> Self {
        match owned {
            ValueKey::Text(text) => BorrowedValueKey::Text(Cow::Borrowed(text)),
            ValueKey::Index(index) => BorrowedValueKey::Index(*index),
            ValueKey::Value(value_container) => {
                BorrowedValueKey::Value(Cow::Borrowed(value_container))
            }
        }
    }
}

impl From<BorrowedValueKey<'_>> for ValueKey {
    fn from(owned: BorrowedValueKey) -> Self {
        match owned {
            BorrowedValueKey::Text(text) => ValueKey::Text(text.into_owned()),
            BorrowedValueKey::Index(index) => ValueKey::Index(index),
            BorrowedValueKey::Value(value_container) => {
                ValueKey::Value(value_container.into_owned())
            }
        }
    }
}

impl<'a> BorrowedValueKey<'a> {
    pub fn with_value_container<R>(
        &self,
        callback: impl FnOnce(&ValueContainer) -> R,
    ) -> R {
        match self {
            BorrowedValueKey::Value(value_container) => {
                callback(value_container)
            }
            BorrowedValueKey::Text(text) => {
                let value_container =
                    ValueContainer::Local(text.to_string().into());
                callback(&value_container)
            }
            BorrowedValueKey::Index(index) => {
                let value_container =
                    ValueContainer::Local(Value::from(*index));
                callback(&value_container)
            }
        }
    }
}

impl<'a> Display for BorrowedValueKey<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BorrowedValueKey::Text(text) => core::write!(f, "{}", text),
            BorrowedValueKey::Index(index) => core::write!(f, "{}", index),
            BorrowedValueKey::Value(value_container) => {
                core::write!(f, "{}", value_container)
            }
        }
    }
}

impl<'a> From<&'a String> for BorrowedValueKey<'a> {
    fn from(text: &'a String) -> Self {
        BorrowedValueKey::Text(Cow::from(text))
    }
}

impl<'a> From<&'a str> for BorrowedValueKey<'a> {
    fn from(text: &'a str) -> Self {
        BorrowedValueKey::Text(Cow::from(text))
    }
}

impl<'a> From<i64> for BorrowedValueKey<'a> {
    fn from(index: i64) -> Self {
        BorrowedValueKey::Index(index)
    }
}

impl<'a> From<u32> for BorrowedValueKey<'a> {
    fn from(index: u32) -> Self {
        BorrowedValueKey::Index(index as i64)
    }
}

impl<'a> From<i32> for BorrowedValueKey<'a> {
    fn from(index: i32) -> Self {
        BorrowedValueKey::Index(index as i64)
    }
}

impl<'a> From<&'a ValueContainer> for BorrowedValueKey<'a> {
    fn from(value_container: &'a ValueContainer) -> Self {
        BorrowedValueKey::Value(Cow::Borrowed(value_container))
    }
}

impl From<ValueContainer> for BorrowedValueKey<'_> {
    fn from(value_container: ValueContainer) -> Self {
        BorrowedValueKey::Value(Cow::Owned(value_container))
    }
}

impl<'a> From<&'a str> for ValueKey {
    fn from(text: &'a str) -> Self {
        ValueKey::Text(text.to_string())
    }
}

impl From<ValueContainer> for ValueKey {
    fn from(value_container: ValueContainer) -> Self {
        ValueKey::Value(value_container)
    }
}

impl From<i32> for ValueKey {
    fn from(index: i32) -> Self {
        ValueKey::Index(index as i64)
    }
}

impl From<i64> for ValueKey {
    fn from(index: i64) -> Self {
        ValueKey::Index(index)
    }
}

impl From<u32> for ValueKey {
    fn from(index: u32) -> Self {
        ValueKey::Index(index as i64)
    }
}

impl<'a> BorrowedValueKey<'a> {
    pub fn try_as_text(&self) -> Option<&str> {
        if let BorrowedValueKey::Text(text) = self {
            Some(text)
        } else if let BorrowedValueKey::Value(val) = self
            && let ValueContainer::Local(Value::Core(
                CoreValueWithClassification {
                    inner: CoreValue::Text(text),
                    ..
                },
            )) = val.as_ref()
        {
            Some(&text.0)
        } else {
            None
        }
    }

    pub fn try_as_index(&self) -> Option<i64> {
        if let BorrowedValueKey::Index(index) = self {
            Some(*index)
        } else if let BorrowedValueKey::Value(value) = self
            && let ValueContainer::Local(Value::Core(
                CoreValueWithClassification {
                    inner: CoreValue::Integer(index),
                    ..
                },
            )) = value.as_ref()
        {
            index.as_i64()
        } else if let BorrowedValueKey::Value(value) = self
            && let ValueContainer::Local(Value::Core(
                CoreValueWithClassification {
                    inner: CoreValue::TypedInteger(index),
                    ..
                },
            )) = value.as_ref()
        {
            index.as_i64()
        } else {
            None
        }
    }
}
