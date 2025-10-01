use std::cell::RefCell;
use crate::dif::value::{DIFReferenceNotFoundError, DIFValueContainer};
use crate::types::structural_type_definition::StructuralTypeDefinition;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use indexmap::IndexMap;
use log::info;
use ordered_float::OrderedFloat;
use crate::dif::r#type::{DIFTypeContainer, DIFTypeDefinition};
use crate::libs::core::{get_core_lib_type, CoreLibPointerId};
use crate::runtime::memory::Memory;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::typed_decimal::{DecimalTypeVariant, TypedDecimal};
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

#[derive(Clone, Debug, PartialEq)]
pub enum DIFValueRepresentation {
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

#[derive(Clone, Debug, PartialEq)]
pub enum DIFTypeRepresentation {
    Null,
    /// Represents a boolean value in DIF.
    Boolean(bool),
    /// Represents a string value in DIF.
    String(String),
    /// Represents a number in DIF.
    Number(f64),
    /// Represents a array of DIF values.
    Array(Vec<DIFTypeContainer>),
    /// Represents a map of DIF values.
    Map(Vec<(DIFTypeContainer, DIFTypeContainer)>),
    /// Represents a struct value in DIF.
    Object(Vec<(String, DIFTypeContainer)>),
}


impl DIFValueRepresentation {

    /// Converts a DIFRepresentationValue into a default Value, without considering additional type information.
    /// Returns an error if a reference cannot be resolved.
    pub fn to_default_value(self, memory: &RefCell<Memory>) -> Result<Value, DIFReferenceNotFoundError> {
        Ok(match self {
            DIFValueRepresentation::Null => Value::null(),
            DIFValueRepresentation::String(str) => Value {
                actual_type: Box::new(get_core_lib_type(
                    CoreLibPointerId::Text,
                )),
                inner: CoreValue::Text(str.into()),
            },
            DIFValueRepresentation::Boolean(b) => Value {
                actual_type: Box::new(get_core_lib_type(
                    CoreLibPointerId::Boolean,
                )),
                inner: CoreValue::Boolean(b.into()),
            },
            DIFValueRepresentation::Number(n) => Value {
                actual_type: Box::new(get_core_lib_type(
                    CoreLibPointerId::Decimal(Some(
                        DecimalTypeVariant::F64,
                    )),
                )),
                inner: CoreValue::TypedDecimal(TypedDecimal::F64(
                    OrderedFloat::from(n),
                )),
            },
            DIFValueRepresentation::Array(array) => Value {
                actual_type: Box::new(get_core_lib_type(
                    CoreLibPointerId::List,
                )),
                inner: CoreValue::List(
                    array
                        .into_iter()
                        .map(|v| v.to_value_container(memory))
                        .collect::<Result<Vec<ValueContainer>, _>>()?
                        .into(),
                ),
            },
            DIFValueRepresentation::Object(object) => {
                let mut map = IndexMap::new();
                for (k, v) in object {
                    map.insert(
                        ValueContainer::Value(Value::from(k)),
                        v.to_value_container(memory)?,
                    );
                }
                Value {
                    actual_type: Box::new(get_core_lib_type(
                        CoreLibPointerId::Map,
                    )),
                    inner: CoreValue::Map(map.into()),
                }
            }
            DIFValueRepresentation::Map(map) => {
                let mut core_map = IndexMap::new();
                for (k, v) in map {
                    core_map.insert(
                        k.to_value_container(memory)?,
                        v.to_value_container(memory)?,
                    );
                }
                Value {
                    actual_type: Box::new(get_core_lib_type(
                        CoreLibPointerId::Map,
                    )),
                    inner: CoreValue::Map(core_map.into()),
                }
            }
            _ => todo!(
                "Other DIFRepresentationValue variants not supported yet"
            ),
        })
    }

    /// Converts a DIFRepresentationValue into a Value, using the provided type information to guide the conversion.
    /// Returns an error if a reference cannot be resolved.
    pub fn to_value_with_type(self, type_container: &DIFTypeContainer, memory: &RefCell<Memory>) -> Result<Value, DIFReferenceNotFoundError> {
        Ok(match r#type_container {
            DIFTypeContainer::Reference(r) => {
                if let Ok(core_lib_ptr_id) = CoreLibPointerId::try_from(r) {
                    match core_lib_ptr_id {
                        // special mappings:
                        // type map and represented as object -> convert to map
                        CoreLibPointerId::Map if let DIFValueRepresentation::Object(object) = self => {
                            let mut core_map: IndexMap<ValueContainer, ValueContainer> = IndexMap::new();
                            for (k, v) in object {
                                core_map.insert(
                                    Value::from(k).into(),
                                    v.to_value_container(memory)?,
                                );
                            }
                            Value::from(CoreValue::Map(core_map.into()))
                        }
                        // otherwise, use default mapping
                        _ => self.to_default_value(memory)?,
                    }
                }
                else {
                    todo!("Handle non-core library type references")
                }
            }
            DIFTypeContainer::Type(dif_type) => {
                match &dif_type.type_definition {
                    DIFTypeDefinition::Structural(s) => {
                        todo!(
                            "Structural type conversion not supported yet"
                        )
                    }
                    DIFTypeDefinition::Unit => Value {
                        actual_type: Box::new(get_core_lib_type(
                            CoreLibPointerId::Null,
                        )),
                        inner: CoreValue::Null,
                    },
                    _ => todo!("Other type definitions not supported yet"),
                }
            }
        })
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeserializeMapOrArray<T>{
    MapEntry(T, T),
    ArrayEntry(T),
}


impl DIFTypeRepresentation {
    pub fn from_structural_type_definition(struct_def: &StructuralTypeDefinition, memory: &RefCell<Memory>) -> Self {
        match struct_def {
            StructuralTypeDefinition::Null => DIFTypeRepresentation::Null,
            StructuralTypeDefinition::Boolean(b) => {
                DIFTypeRepresentation::Boolean(b.as_bool())
            }
            StructuralTypeDefinition::Integer(i) => {
                // FIXME: this can overflow
                DIFTypeRepresentation::Number(i.as_i128().unwrap() as f64)
            }
            StructuralTypeDefinition::TypedInteger(i) => {
                DIFTypeRepresentation::Number(i.as_i128().unwrap() as f64)
            }
            StructuralTypeDefinition::Decimal(d) => {
                DIFTypeRepresentation::Number(d.try_into_f64().unwrap())
            }
            StructuralTypeDefinition::TypedDecimal(d) => {
                DIFTypeRepresentation::Number(d.as_f64())
            }
            StructuralTypeDefinition::Text(t) => {
                DIFTypeRepresentation::String(t.0.clone())
            }
            StructuralTypeDefinition::Endpoint(endpoint) => {
                DIFTypeRepresentation::String(endpoint.to_string())
            }
            StructuralTypeDefinition::List(arr) => DIFTypeRepresentation::Array(
                arr.iter().map(|v| DIFTypeContainer::from_type_container(v, memory)).collect(),
            ),
            StructuralTypeDefinition::Map(fields) => DIFTypeRepresentation::Map(
                fields
                    .into_iter()
                    .map(|(k, v)| (
                        DIFTypeContainer::from_type_container(k, memory),
                        DIFTypeContainer::from_type_container(v, memory))
                    )
                    .collect(),
            ),
        }
    }
}

impl Serialize for DIFValueRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DIFValueRepresentation::Null => serializer.serialize_unit(),
            DIFValueRepresentation::Boolean(b) => serializer.serialize_bool(*b),
            DIFValueRepresentation::String(s) => serializer.serialize_str(s),
            DIFValueRepresentation::Number(f) => serializer.serialize_f64(*f),
            DIFValueRepresentation::Array(vec) => vec.serialize(serializer),
            DIFValueRepresentation::Map(entries) => {
                let mut seq = serializer.serialize_seq(Some(entries.len()))?;
                for (k, v) in entries {
                    seq.serialize_element(&vec![k, v])?;
                }
                seq.end()
            }
            DIFValueRepresentation::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (k, v) in fields {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for DIFValueRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DIFCoreValueVisitor;

        impl<'de> Visitor<'de> for DIFCoreValueVisitor {
            type Value = DIFValueRepresentation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DIFCoreValue")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(DIFValueRepresentation::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(DIFValueRepresentation::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                // Safe cast since DIFCoreValue uses i64
                Ok(DIFValueRepresentation::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(DIFValueRepresentation::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DIFValueRepresentation::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(DIFValueRepresentation::String(value))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DIFValueRepresentation::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DIFValueRepresentation::Null)
            }

            // array / map
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let first_entry =
                    seq.next_element::<DeserializeMapOrArray<DIFValueContainer>>()?;
                match first_entry {
                    Some(DeserializeMapOrArray::ArrayEntry(first)) => {
                        let mut elements = vec![first];
                        while let Some(elem) =
                            seq.next_element::<DIFValueContainer>()?
                        {
                            elements.push(elem);
                        }
                        Ok(DIFValueRepresentation::Array(elements))
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
                        Ok(DIFValueRepresentation::Map(elements))
                    }
                    None => Ok(DIFValueRepresentation::Array(vec![])), // empty array
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
                Ok(DIFValueRepresentation::Object(entries))
            }
        }

        deserializer.deserialize_any(DIFCoreValueVisitor)
    }
}


impl Serialize for DIFTypeRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DIFTypeRepresentation::Null => serializer.serialize_unit(),
            DIFTypeRepresentation::Boolean(b) => serializer.serialize_bool(*b),
            DIFTypeRepresentation::String(s) => serializer.serialize_str(s),
            DIFTypeRepresentation::Number(f) => serializer.serialize_f64(*f),
            DIFTypeRepresentation::Array(vec) => vec.serialize(serializer),
            DIFTypeRepresentation::Map(entries) => {
                let mut seq = serializer.serialize_seq(Some(entries.len()))?;
                for (k, v) in entries {
                    seq.serialize_element(&vec![k, v])?;
                }
                seq.end()
            }
            DIFTypeRepresentation::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (k, v) in fields {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for DIFTypeRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DIFCoreValueVisitor;

        impl<'de> Visitor<'de> for DIFCoreValueVisitor {
            type Value = DIFTypeRepresentation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DIFCoreValue")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(DIFTypeRepresentation::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(DIFTypeRepresentation::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                // Safe cast since DIFCoreValue uses i64
                Ok(DIFTypeRepresentation::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(DIFTypeRepresentation::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DIFTypeRepresentation::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(DIFTypeRepresentation::String(value))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DIFTypeRepresentation::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DIFTypeRepresentation::Null)
            }

            // array / map
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let first_entry =
                    seq.next_element::<DeserializeMapOrArray<DIFTypeContainer>>()?;
                match first_entry {
                    Some(DeserializeMapOrArray::ArrayEntry(first)) => {
                        let mut elements = vec![first];
                        while let Some(elem) =
                            seq.next_element::<DIFTypeContainer>()?
                        {
                            elements.push(elem);
                        }
                        Ok(DIFTypeRepresentation::Array(elements))
                    }
                    Some(DeserializeMapOrArray::MapEntry(k, v)) => {
                        let mut elements = vec![(k, v)];
                        while let Some((k, v)) = seq.next_element::<(
                            DIFTypeContainer,
                            DIFTypeContainer,
                        )>(
                        )? {
                            elements.push((k, v));
                        }
                        Ok(DIFTypeRepresentation::Map(elements))
                    }
                    None => Ok(DIFTypeRepresentation::Array(vec![])), // empty array
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
                Ok(DIFTypeRepresentation::Object(entries))
            }
        }

        deserializer.deserialize_any(DIFCoreValueVisitor)
    }
}

