use serde::{Deserialize, Serialize};

use crate::dif::{DIFConvertible, value::DIFValueContainer};
use crate::references::observers::TransceiverId;

/// Represents a property in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", content = "value")]
pub enum DIFProperty {
    /// a simple string property
    Text(String),
    /// an integer property (e.g. an array index)
    // FIXME #385 use usize or u32 u64
    Index(i64),
    /// any other property type
    Value(DIFValueContainer),
}

impl From<String> for DIFProperty {
    fn from(s: String) -> Self {
        DIFProperty::Text(s)
    }
}
impl From<&str> for DIFProperty {
    fn from(s: &str) -> Self {
        DIFProperty::Text(s.to_string())
    }
}
impl From<i64> for DIFProperty {
    fn from(i: i64) -> Self {
        DIFProperty::Index(i)
    }
}
impl From<DIFValueContainer> for DIFProperty {
    fn from(v: DIFValueContainer) -> Self {
        DIFProperty::Value(v)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFUpdate {
    pub source_id: TransceiverId,
    pub data: DIFUpdateData,
}

impl DIFUpdate {
    /// Creates a new `DIFUpdate` with the given source ID and update data.
    pub fn new(source_id: TransceiverId, data: DIFUpdateData) -> Self {
        DIFUpdate { source_id, data }
    }
}

// TODO #386 optimize structural representation by using integer values for enum variants
// and shrink down keys (kind: 0, 1, 2 instead of "clear", "set", "remove", ...)

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DIFUpdateData {
    /// Represents a replacement operation for a DIF value.
    Replace { value: DIFValueContainer },

    /// Represents an update to a specific property of a DIF value.
    /// The `key` specifies which property to update, and `value` is the new value for that property.
    Set {
        key: DIFProperty,
        value: DIFValueContainer,
    },

    /// Represents the removal of a specific property from a DIF value.
    Remove { key: DIFProperty },

    /// Represents clearing all elements from a collection-type DIF value (like an array or map).
    Clear,

    /// Represents adding a new element to a collection-type DIF value (like an array or map).
    Push { value: DIFValueContainer },
}

impl DIFConvertible for DIFUpdateData {}

impl DIFUpdateData {
    /// Creates a new `DIFUpdateData::Replace` variant with the given value.
    pub fn replace(value: impl Into<DIFValueContainer>) -> Self {
        DIFUpdateData::Replace {
            value: value.into(),
        }
    }

    /// Creates a new `DIFUpdateData::Set` variant with the given key and value.
    pub fn set(
        key: impl Into<DIFProperty>,
        value: impl Into<DIFValueContainer>,
    ) -> Self {
        DIFUpdateData::Set {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Creates a new `DIFUpdateData::Remove` variant with the given key.
    pub fn remove(key: impl Into<DIFProperty>) -> Self {
        DIFUpdateData::Remove { key: key.into() }
    }

    /// Creates a new `DIFUpdateData::Clear` variant.
    pub fn clear() -> Self {
        DIFUpdateData::Clear
    }

    /// Creates a new `DIFUpdateData::Push` variant with the given value.
    pub fn push(value: impl Into<DIFValueContainer>) -> Self {
        DIFUpdateData::Push {
            value: value.into(),
        }
    }

    pub fn with_source(self, source_id: TransceiverId) -> DIFUpdate {
        DIFUpdate {
            source_id,
            data: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dif::representation::DIFValueRepresentation;
    use crate::dif::value::DIFValue;

    #[test]
    fn serialize_replace() {
        let dif_update =
            DIFUpdateData::replace(DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::String("Hello".to_string()),
                r#type: None,
            }));
        let serialized = dif_update.as_json();
        assert_eq!(
            serialized,
            r#"{"kind":"replace","value":{"value":"Hello"}}"#
        );
        let deserialized = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }

    #[test]
    fn serialize_set() {
        let dif_update = DIFUpdateData::set(
            "name",
            DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::Number(42.0),
                r#type: None,
            }),
        );
        let serialized = dif_update.as_json();
        assert_eq!(
            serialized,
            r#"{"kind":"set","key":{"kind":"text","value":"name"},"value":{"value":42.0}}"#
        );
        let deserialized = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }

    #[test]
    fn serialize_remove() {
        let dif_update = DIFUpdateData::remove("age");
        let serialized = dif_update.as_json();
        assert_eq!(
            serialized,
            r#"{"kind":"remove","key":{"kind":"text","value":"age"}}"#
        );
        let deserialized = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }

    #[test]
    fn serialize_clear() {
        let dif_update = DIFUpdateData::clear();
        let serialized = dif_update.as_json();
        assert_eq!(serialized, r#"{"kind":"clear"}"#);
        let deserialized = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }

    #[test]
    fn serialize_push() {
        let dif_update =
            DIFUpdateData::push(DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::Boolean(true),
                r#type: None,
            }));
        let serialized = dif_update.as_json();
        assert_eq!(serialized, r#"{"kind":"push","value":{"value":true}}"#);
        let deserialized = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }
}
