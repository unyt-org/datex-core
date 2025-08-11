use std::fmt;
use serde::{de, Deserialize, Deserializer, Serializer};
use serde::de::{IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde_with::serde_derive::Serialize;
use datex_core::values::core_value::CoreValue;
use crate::values::datex_type::CoreValueType;
use crate::values::value_container::ValueContainer;

/// Represents a value in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFValue {
    pub value: Option<DIFCoreValue>,
    pub core_type: CoreValueType,
    // TODO: handle more complex types here
    pub r#type: String,
    pub ptr_id: Option<String>,
}

impl From<&ValueContainer> for DIFValue {
    fn from(value: &ValueContainer) -> Self {
        let val_rc = value.to_value();
        let val = val_rc.borrow();
        let core_value = &val.inner;
        let actual_type = &val.actual_type;
        let core_type = core_value.get_default_type();

        let dif_core_value = match core_value {
            CoreValue::Null => None,
            CoreValue::Bool(bool) => Some(DIFCoreValue::Boolean(bool.0)),
            CoreValue::Integer(integer) => {
                Some(DIFCoreValue::Integer(integer.0.as_i64().unwrap()))
            },
            CoreValue::TypedInteger(integer) => {
                Some(DIFCoreValue::Integer(integer.as_i64().unwrap()))
            }
            CoreValue::Decimal(decimal) => {
                Some(DIFCoreValue::Float(decimal.try_into_f64().unwrap()))
            }
            CoreValue::TypedDecimal(decimal) => {
                Some(DIFCoreValue::Float(decimal.as_f64()))
            }
            CoreValue::Text(text) => Some(DIFCoreValue::String(text.0.clone())),
            CoreValue::Endpoint(endpoint) => {
                Some(DIFCoreValue::String(endpoint.to_string()))
            }
            CoreValue::Array(array) => {
                Some(DIFCoreValue::List(
                    array.0.iter().map(|v| v.into()).collect(),
                ))
            }
            CoreValue::Object(object) => {
                Some(DIFCoreValue::Map(
                    object
                        .0
                        .iter()
                        .map(|(k, v)| (k.clone(), v.into()))
                        .collect(),
                ))
            }
            CoreValue::Tuple(tuple) => {
                Some(DIFCoreValue::List(
                    tuple.entries.iter().map(|(k, v)| {
                        DIFValue {
                            value: Some(DIFCoreValue::List(vec![k.into(), v.into()])),
                            core_type: CoreValueType::Array,
                            r#type: serde_json::to_string(&k).unwrap().trim_matches('"').to_string(),
                            ptr_id: None,
                        }
                    }).collect(),
                ))
            }
        };

        DIFValue {
            value: dif_core_value,
            core_type,
            r#type: serde_json::to_string(&actual_type).unwrap().trim_matches('"').to_string(),
            ptr_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DIFCoreValue {
    Null,
    /// Represents a boolean value in DIF.
    Boolean(bool),
    /// Represents a string value in DIF.
    String(String),
    /// Represents an integer value in DIF.
    Integer(i64),
    /// Represents a floating-point number in DIF.
    Float(f64),
    /// Represents a list of DIF values.
    List(Vec<DIFValue>),
    /// Represents a map of DIF values.
    Map(Vec<(String, DIFValue)>),
}

impl serde::Serialize for DIFCoreValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DIFCoreValue::Null => serializer.serialize_none(),
            DIFCoreValue::Boolean(b) => serializer.serialize_bool(*b),
            DIFCoreValue::String(s) => serializer.serialize_str(s),
            DIFCoreValue::Integer(i) => serializer.serialize_i64(*i),
            DIFCoreValue::Float(f) => serializer.serialize_f64(*f),
            DIFCoreValue::List(vec) => vec.serialize(serializer),
            DIFCoreValue::Map(map) => {
                use std::collections::BTreeMap;
                let mut m = BTreeMap::new();
                for (k, v) in map {
                    m.insert(k, v);
                }
                m.serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for DIFCoreValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
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
                Ok(DIFCoreValue::Integer(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                // Safe cast since DIFCoreValue uses i64
                Ok(DIFCoreValue::Integer(value as i64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(DIFCoreValue::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where E: de::Error {
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
            where A: SeqAccess<'de> {
                let mut elements = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    elements.push(elem);
                }
                Ok(DIFCoreValue::List(elements))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where A: MapAccess<'de> {
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
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dif_value_serialization() {
        let value = DIFValue {
            value: None,
            core_type: CoreValueType::Null,
            r#type: "null".to_string(),
            ptr_id: None,
        };
        let serialized = serde_json::to_string(&value).unwrap();
        println!("Serialized DIFValue: {}", serialized);
        let deserialized: DIFValue = serde_json::from_str(&serialized).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_dif_property_serialization() {
        let property = DIFProperty::Text("example".to_string());
        let serialized = serde_json::to_string(&property).unwrap();
        let deserialized: DIFProperty = serde_json::from_str(&serialized).unwrap();
        assert_eq!(property, deserialized);
    }

    #[test]
    fn test_from_value_container_int() {
        let value_container = ValueContainer::from(42i32);
        let dif_value: DIFValue = DIFValue::from(&value_container);
        assert_eq!(dif_value.value, Some(DIFCoreValue::Integer(42)));
        assert_eq!(dif_value.core_type, CoreValueType::I32);
        assert_eq!(dif_value.r#type, "i32");
        assert!(dif_value.ptr_id.is_none());
        let serialized = serde_json::to_string(&dif_value).unwrap();
        println!("Serialized DIFValue from int: {}", serialized);
    }
}