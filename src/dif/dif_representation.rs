use crate::dif::value::DIFValueContainer;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum DIFRepresentationValue {
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeserializeMapOrArray {
    MapEntry(DIFValueContainer, DIFValueContainer),
    ArrayEntry(DIFValueContainer),
}

impl From<StructuralTypeDefinition> for DIFRepresentationValue {
    fn from(struct_def: StructuralTypeDefinition) -> Self {
        match struct_def {
            StructuralTypeDefinition::Null => DIFRepresentationValue::Null,
            StructuralTypeDefinition::Boolean(b) => {
                DIFRepresentationValue::Boolean(b.as_bool())
            }
            StructuralTypeDefinition::Integer(i) => {
                // FIXME: this can overflow
                DIFRepresentationValue::Number(i.as_i128().unwrap() as f64)
            }
            StructuralTypeDefinition::TypedInteger(i) => {
                DIFRepresentationValue::Number(i.as_i128().unwrap() as f64)
            }
            StructuralTypeDefinition::Decimal(d) => {
                DIFRepresentationValue::Number(d.try_into_f64().unwrap())
            }
            StructuralTypeDefinition::TypedDecimal(d) => {
                DIFRepresentationValue::Number(d.as_f64())
            }
            StructuralTypeDefinition::Text(t) => {
                DIFRepresentationValue::String(t.0)
            }
            // StructuralTypeDefinition::Array(arr) => DIFCoreValue::Array(
            //     arr.into_iter().map(DIFValueContainer::from).collect(),
            // ),
            // StructuralTypeDefinition::List(list) => DIFCoreValue::Array(
            //     list.into_iter().map(DIFValueContainer::from).collect(),
            // ),
            // StructuralTypeDefinition::Map(map) => DIFCoreValue::Map(
            //     map.into_iter()
            //         .map(|(k, v)| {
            //             (DIFValueContainer::from(k), DIFValueContainer::from(v))
            //         })
            //         .collect(),
            // ),
            // StructuralTypeDefinition::Struct(fields) => DIFCoreValue::Object(
            //     fields
            //         .into_iter()
            //         .map(|(k, v)| (k, DIFValueContainer::from(v)))
            //         .collect(),
            // ),
            _ => unimplemented!(
                "Conversion for structural type definition {:?} not implemented yet",
                struct_def
            ),
        }
    }
}

impl Serialize for DIFRepresentationValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DIFRepresentationValue::Null => serializer.serialize_none(), // FIXME
            DIFRepresentationValue::Boolean(b) => serializer.serialize_bool(*b),
            DIFRepresentationValue::String(s) => serializer.serialize_str(s),
            DIFRepresentationValue::Number(f) => serializer.serialize_f64(*f),
            DIFRepresentationValue::Array(vec) => vec.serialize(serializer),
            DIFRepresentationValue::Map(entries) => {
                let mut seq = serializer.serialize_seq(Some(entries.len()))?;
                for (k, v) in entries {
                    seq.serialize_element(&vec![k, v])?;
                }
                seq.end()
            }
            DIFRepresentationValue::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (k, v) in fields {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for DIFRepresentationValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DIFCoreValueVisitor;

        impl<'de> Visitor<'de> for DIFCoreValueVisitor {
            type Value = DIFRepresentationValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DIFCoreValue")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(DIFRepresentationValue::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(DIFRepresentationValue::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                // Safe cast since DIFCoreValue uses i64
                Ok(DIFRepresentationValue::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(DIFRepresentationValue::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DIFRepresentationValue::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(DIFRepresentationValue::String(value))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DIFRepresentationValue::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DIFRepresentationValue::Null)
            }

            // array / map
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let first_entry =
                    seq.next_element::<DeserializeMapOrArray>()?;
                match first_entry {
                    Some(DeserializeMapOrArray::ArrayEntry(first)) => {
                        let mut elements = vec![first];
                        while let Some(elem) =
                            seq.next_element::<DIFValueContainer>()?
                        {
                            elements.push(elem);
                        }
                        Ok(DIFRepresentationValue::Array(elements))
                    }
                    Some(DeserializeMapOrArray::MapEntry(k, v)) => {
                        let mut elements = vec![(k, v)];
                        while let Some((k, v)) = seq.next_element::<(
                            DIFValueContainer,
                            DIFValueContainer,
                        )>(
                        )? {
                            elements.push((k, v));
                        }
                        Ok(DIFRepresentationValue::Map(elements))
                    }
                    None => Ok(DIFRepresentationValue::Array(vec![])), // empty array
                }
            }

            // object
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                while let Some((k, v)) = map.next_entry()? {
                    entries.push((k, v));
                }
                Ok(DIFRepresentationValue::Object(entries))
            }
        }

        deserializer.deserialize_any(DIFCoreValueVisitor)
    }
}
