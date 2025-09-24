use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::text::Text;
use crate::values::core_values::r#type::structural_type_definition::StructuralTypeDefinition;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use datex_core::values::core_value::CoreValue;
use datex_core::values::core_values::integer::integer::Integer;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serializer, de};
use serde_with::serde_derive::Serialize;
use std::fmt;
use crate::values::type_container::TypeContainer;

/// Represents a value in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFValue {
    pub value: Option<DIFCoreValue>,
    pub r#type: TypeContainer,
    pub ptr_id: Option<String>,
}

impl From<&ValueContainer> for DIFValue {
    fn from(value: &ValueContainer) -> Self {
        let val_rc = value.to_value();
        let val = val_rc.borrow();
        let core_value = &val.inner;

        let dif_core_value = match core_value {
            CoreValue::Type(ty) => todo!("Type value not supported in DIF"),
            CoreValue::Null => Some(DIFCoreValue::Null),
            CoreValue::Boolean(bool) => Some(DIFCoreValue::Boolean(bool.0)),
            CoreValue::Integer(integer) => {
                // TODO: optimize this and pass as integer if in range
                Some(DIFCoreValue::String(integer.to_string()))
            }
            CoreValue::TypedInteger(integer) => {
                // Some(DIFCoreValue::Number(integer.as_i64().unwrap() as f64))
                match integer {
                    TypedInteger::I8(i) => {
                        Some(DIFCoreValue::Number(*i as f64))
                    }
                    TypedInteger::U8(u) => {
                        Some(DIFCoreValue::Number(*u as f64))
                    }
                    TypedInteger::I16(i) => {
                        Some(DIFCoreValue::Number(*i as f64))
                    }
                    TypedInteger::U16(u) => {
                        Some(DIFCoreValue::Number(*u as f64))
                    }
                    TypedInteger::I32(i) => {
                        Some(DIFCoreValue::Number(*i as f64))
                    }
                    TypedInteger::U32(u) => {
                        Some(DIFCoreValue::Number(*u as f64))
                    }
                    // i64 and above are serialized as strings in DIF
                    TypedInteger::I64(i) => {
                        Some(DIFCoreValue::String(i.to_string()))
                    }
                    TypedInteger::U64(u) => {
                        Some(DIFCoreValue::String(u.to_string()))
                    }
                    TypedInteger::I128(i) => {
                        Some(DIFCoreValue::String(i.to_string()))
                    }
                    TypedInteger::U128(u) => {
                        Some(DIFCoreValue::String(u.to_string()))
                    }
                    TypedInteger::Big(i) => {
                        Some(DIFCoreValue::String(i.to_string()))
                    }
                }
            }
            CoreValue::Decimal(decimal) => {
                // TODO: optimize this and pass as decimal if in range
                Some(DIFCoreValue::String(decimal.to_string()))
            }
            CoreValue::TypedDecimal(decimal) => match decimal {
                TypedDecimal::F32(f) => Some(DIFCoreValue::Number(f.0 as f64)),
                TypedDecimal::F64(f) => Some(DIFCoreValue::Number(f.0)),
                TypedDecimal::Decimal(bd) => {
                    Some(DIFCoreValue::String(bd.to_string()))
                }
            },
            CoreValue::Text(text) => Some(DIFCoreValue::String(text.0.clone())),
            CoreValue::Endpoint(endpoint) => {
                Some(DIFCoreValue::String(endpoint.to_string()))
            }

            CoreValue::Struct(structure) => Some(DIFCoreValue::Struct(
                structure
                    .iter()
                    .map(|(key, value)| (key.clone(), DIFValue::from(value)))
                    .collect(),
            )),
            CoreValue::List(list) => Some(DIFCoreValue::List(
                list.into_iter().map(|v| v.into()).collect(),
            )),
            CoreValue::Array(arr) => Some(DIFCoreValue::Array(
                arr.into_iter().map(|v| v.into()).collect(),
            )),
            CoreValue::Map(map) => Some(DIFCoreValue::Map(
                map.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
            )),
        };

        DIFValue {
            value: dif_core_value,
            // FIXME custom type when serializing the whole actual_type to a json object
            r#type: value.actual_type(),
            ptr_id: None,
        }
    }
}

impl From<ValueContainer> for DIFValue {
    fn from(value: ValueContainer) -> Self {
        DIFValue::from(&value)
    }
}

impl From<DIFValue> for ValueContainer {
    fn from(value: DIFValue) -> Self {
        ValueContainer::from(&value)
    }
}
impl From<&DIFValue> for ValueContainer {
    fn from(value: &DIFValue) -> Self {
        let struct_type = value
            .r#type
            .clone()
            .as_type()
            .structural_type().cloned();
        let core_value = match &value.value {
            Some(DIFCoreValue::Null) => CoreValue::Null,
            Some(DIFCoreValue::Boolean(b)) => CoreValue::Boolean(Boolean(*b)),
            Some(DIFCoreValue::String(s)) => {
                match struct_type.expect("") {
                    StructuralTypeDefinition::Text(_) => {
                        CoreValue::Text(Text(s.clone()))
                    }
                    StructuralTypeDefinition::Endpoint(_) => {
                        CoreValue::Endpoint(s.parse().unwrap())
                    }
                    // i64 and above are also serialized as strings in DIF
                    // StructuralType::I64(_) => CoreValue::TypedInteger(
                    //     TypedInteger::I64(s.parse().unwrap()),
                    // ),
                    // StructuralType::U64(_) => CoreValue::TypedInteger(
                    //     TypedInteger::U64(s.parse().unwrap()),
                    // ),
                    StructuralTypeDefinition::Integer(_) => CoreValue::Integer(
                        Integer::from(s.parse::<i64>().unwrap()),
                    ),
                    // big decimal types are also serialized as strings in DIF
                    StructuralTypeDefinition::Decimal(_) => CoreValue::Decimal(
                        Decimal::from(s.parse::<f64>().unwrap()),
                    ),
                    _ => unreachable!(
                        "Unsupported core type for string conversion"
                    ),
                }
            }
            Some(DIFCoreValue::Number(n)) => match struct_type.unwrap() {
                StructuralTypeDefinition::TypedInteger(typed_int) => {
                    match typed_int {
                        TypedInteger::I8(_) => {
                            CoreValue::TypedInteger(TypedInteger::I8(*n as i8))
                        }
                        TypedInteger::U8(_) => {
                            CoreValue::TypedInteger(TypedInteger::U8(*n as u8))
                        }
                        TypedInteger::I16(_) => CoreValue::TypedInteger(
                            TypedInteger::I16(*n as i16),
                        ),
                        TypedInteger::U16(_) => CoreValue::TypedInteger(
                            TypedInteger::U16(*n as u16),
                        ),
                        TypedInteger::I32(_) => CoreValue::TypedInteger(
                            TypedInteger::I32(*n as i32),
                        ),
                        TypedInteger::U32(_) => CoreValue::TypedInteger(
                            TypedInteger::U32(*n as u32),
                        ),
                        _ => unreachable!(
                            "Unsupported core type for number conversion"
                        ),
                    }
                }
                StructuralTypeDefinition::TypedDecimal(typed_decimal) => {
                    match typed_decimal {
                        TypedDecimal::F32(_) => CoreValue::TypedDecimal(
                            TypedDecimal::from(*n as f32),
                        ),
                        TypedDecimal::F64(_) => {
                            CoreValue::TypedDecimal(TypedDecimal::from(*n))
                        }
                        _ => unreachable!(
                            "Unsupported core type for number conversion"
                        ),
                    }
                }
                _ => {
                    unreachable!("Unsupported core type for number conversion")
                }
            },
            Some(DIFCoreValue::Array(arr)) => {
                CoreValue::Array(arr.iter().map(ValueContainer::from).collect())
            }
            Some(DIFCoreValue::List(list)) => {
                CoreValue::List(list.iter().map(ValueContainer::from).collect())
            }
            Some(DIFCoreValue::Map(entries)) => CoreValue::Map(
                entries
                    .iter()
                    .map(|(k, v)| {
                        (ValueContainer::from(k), ValueContainer::from(v))
                    })
                    .collect(),
            ),
            Some(DIFCoreValue::Struct(fields)) => CoreValue::Struct(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), ValueContainer::from(v)))
                    .collect(),
            ),
            None => CoreValue::Null,
        };

        ValueContainer::Value(Value::from(core_value))
    }
}

pub enum DIFType {
    Core(String),
    Custom(StructuralTypeDefinition),
}

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
    Array(Vec<DIFValue>),
    /// Represents a list of DIF values.
    List(Vec<DIFValue>),
    /// Represents a map of DIF values.
    Map(Vec<(DIFValue, DIFValue)>),
    /// Represents a struct value in DIF.
    Struct(Vec<(String, DIFValue)>),
}

impl serde::Serialize for DIFCoreValue {
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
            DIFCoreValue::List(vec) => vec.serialize(serializer),
            DIFCoreValue::Map(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (k, v) in entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            DIFCoreValue::Struct(fields) => {
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

/// Represents a property in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DIFProperty {
    /// a simple string property
    Text(String),
    /// an integer property (e.g. an array index)
    Integer(i64),
    /// any other property type
    Value(DIFValue),
}

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DIFUpdate {
    Replace(DIFValue),
    UpdateProperty {
        property: DIFProperty,
        value: DIFValue,
    },
    Push(DIFValue),
}

#[cfg(test)]
mod tests {
    use datex_core::values::core_values::r#type::Type;
    use super::*;

    #[test]
    fn dif_value_serialization() {
        let value = DIFValue {
            value: None,
            r#type: TypeContainer::Type(Type::structural(StructuralTypeDefinition::Null)),
            ptr_id: None,
        };
        let serialized = serde_json::to_string(&value).unwrap();
        println!("Serialized DIFValue: {}", serialized);
        let deserialized: DIFValue = serde_json::from_str(&serialized).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn dif_property_serialization() {
        let property = DIFProperty::Text("example".to_string());
        let serialized = serde_json::to_string(&property).unwrap();
        let deserialized: DIFProperty =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(property, deserialized);
    }

    #[test]
    fn from_value_container_i32() {
        let value_container = ValueContainer::from(42i32);
        let dif_value: DIFValue = DIFValue::from(&value_container);
        assert_eq!(dif_value.value, Some(DIFCoreValue::Number(42f64)));
        // assert_eq!(dif_value.r#type, "i32");
        assert!(dif_value.ptr_id.is_none());
        let serialized = serde_json::to_string(&dif_value).unwrap();
        println!("Serialized DIFValue from int: {}", serialized);
    }

    #[test]
    fn from_value_container_text() {
        let value_container = ValueContainer::from("Hello, World!");
        let dif_value: DIFValue = DIFValue::from(&value_container);
        assert_eq!(
            dif_value.value,
            Some(DIFCoreValue::String("Hello, World!".to_string()))
        );
        // assert_eq!(dif_value.core_type, CoreValueType::Text);
        // assert_eq!(dif_value.r#type, "text");
        assert!(dif_value.ptr_id.is_none());
    }

    #[test]
    fn to_value_container_i32() {
        let dif_value = DIFValue {
            value: Some(DIFCoreValue::Number(42f64)),
            r#type: TypeContainer::Type(Type::structural(StructuralTypeDefinition::Null)), // TODO
            ptr_id: None,
        };
        let value_container: ValueContainer = ValueContainer::from(&dif_value);
        if let ValueContainer::Value(val) = value_container {
            assert_eq!(
                val.inner,
                CoreValue::TypedInteger(TypedInteger::I32(42))
            );
            // assert_eq!(val.get_type(), CoreValueType::I32);
        } else {
            panic!("Expected ValueContainer::Value");
        }
    }

    #[test]
    fn to_value_container_text() {
        let dif_value = DIFValue {
            value: Some(DIFCoreValue::String("Hello, World!".to_string())),
            r#type: TypeContainer::Type(Type::structural(StructuralTypeDefinition::Null)), // TODO
            ptr_id: None,
        };
        let value_container: ValueContainer = ValueContainer::from(&dif_value);
        if let ValueContainer::Value(val) = value_container {
            assert_eq!(
                val.inner,
                CoreValue::Text(Text("Hello, World!".to_string()))
            );
            // assert_eq!(val.get_type(), CoreValueType::Text);
        } else {
            panic!("Expected ValueContainer::Value");
        }
    }
}
