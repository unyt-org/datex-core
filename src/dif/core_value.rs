use crate::dif::value::DIFValueContainer;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum DIFCoreValue {
    Null,
    /// Represents a boolean value in DIF.
    Boolean(bool),
    /// Represents a string value in DIF.
    String(String),
    /// Represents a number in DIF.
    Number(f64),
    /// Represents a array of DIF values.
    Array(Vec<DIFValueContainer>),
    /// Represents a map of DIF values.
    Map(Vec<(DIFValueContainer, DIFValueContainer)>),
    /// Represents a struct value in DIF.
    Object(Vec<(String, DIFValueContainer)>),
}

impl Serialize for DIFCoreValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DIFCoreValue::Null => serializer.serialize_unit(),
            DIFCoreValue::Boolean(b) => serializer.serialize_bool(*b),
            DIFCoreValue::String(s) => serializer.serialize_str(s),
            DIFCoreValue::Number(f) => serializer.serialize_f64(*f),
            DIFCoreValue::Array(vec) => vec.serialize(serializer),
            DIFCoreValue::Map(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (k, v) in entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            DIFCoreValue::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (k, v) in fields {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for DIFCoreValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DIFCoreValueVisitor;

        impl<'de> Visitor<'de> for DIFCoreValueVisitor {
            type Value = DIFCoreValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DIFCoreValue")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                // Safe cast since DIFCoreValue uses i64
                Ok(DIFCoreValue::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DIFCoreValue::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::String(value))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::Null)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut elements = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    elements.push(elem);
                }
                Ok(DIFCoreValue::Array(elements))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                while let Some((k, v)) = map.next_entry()? {
                    entries.push((k, v));
                }
                Ok(DIFCoreValue::Map(entries))
            }
        }

        deserializer.deserialize_any(DIFCoreValueVisitor)
    }
}
