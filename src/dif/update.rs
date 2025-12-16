use crate::dif::{DIFConvertible, value::DIFValueContainer};
use crate::references::observers::TransceiverId;
use crate::runtime::memory::Memory;
use crate::stdlib::borrow::Cow;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::values::value_container::ValueKey;
use core::cell::RefCell;
use core::prelude::rust_2024::*;
use serde::{Deserialize, Serialize};

/// Represents a key in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", content = "value")]
pub enum DIFKey {
    /// a simple string property
    Text(String),
    /// an integer property (e.g. an array index)
    // FIXME #385 use usize or u32 u64
    Index(i64),
    /// any other property type
    Value(DIFValueContainer),
}

impl DIFKey {
    pub fn from_value_key(key: &ValueKey, memory: &RefCell<Memory>) -> Self {
        match key {
            ValueKey::Text(s) => DIFKey::Text(s.to_string()),
            ValueKey::Index(i) => DIFKey::Index(*i),
            ValueKey::Value(v) => DIFKey::Value(
                DIFValueContainer::from_value_container(v, memory),
            ),
        }
    }
}

impl From<String> for DIFKey {
    fn from(s: String) -> Self {
        DIFKey::Text(s)
    }
}
impl From<&str> for DIFKey {
    fn from(s: &str) -> Self {
        DIFKey::Text(s.to_string())
    }
}
impl From<i64> for DIFKey {
    fn from(i: i64) -> Self {
        DIFKey::Index(i)
    }
}
impl From<DIFValueContainer> for DIFKey {
    fn from(v: DIFValueContainer) -> Self {
        DIFKey::Value(v)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFUpdate<'a> {
    pub source_id: TransceiverId,
    pub data: Cow<'a, DIFUpdateData>,
}

impl<'a> DIFUpdate<'a> {
    /// Creates a new `DIFUpdate` with the given source ID and update data.
    pub fn new(source_id: TransceiverId, data: Cow<'a, DIFUpdateData>) -> Self {
        DIFUpdate { source_id, data }
    }
}

// TODO #386 optimize structural representation by using integer values for enum variants
// and shrink down keys (kind: 0, 1, 2 instead of "clear", "set", "remove", ...)

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DIFUpdateData {
    /// Represents a replacement operation for a DIF value.
    Replace { value: DIFValueContainer },

    /// Represents an update to a specific property of a DIF value.
    /// The `key` specifies which property to update, and `value` is the new value for that property.
    Set {
        key: DIFKey,
        value: DIFValueContainer,
    },

    /// Represents the removal of a specific property from a DIF value.
    Delete { key: DIFKey },

    /// Represents clearing all elements from a collection-type DIF value (like an array or map).
    Clear,

    /// Represents adding a new element to a collection-type DIF value (like an array or map).
    Append { value: DIFValueContainer },

    /// Special update operation for list values that allows splicing
    ListSplice {
        start: u32,
        delete_count: u32,
        items: Vec<DIFValueContainer>,
    },
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
        key: impl Into<DIFKey>,
        value: impl Into<DIFValueContainer>,
    ) -> Self {
        DIFUpdateData::Set {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Creates a new `DIFUpdateData::Delete` variant with the given key.
    pub fn delete(key: impl Into<DIFKey>) -> Self {
        DIFUpdateData::Delete { key: key.into() }
    }

    /// Creates a new `DIFUpdateData::Clear` variant.
    pub fn clear() -> Self {
        DIFUpdateData::Clear
    }

    /// Creates a new `DIFUpdateData::Append` variant with the given value.
    pub fn append(value: impl Into<DIFValueContainer>) -> Self {
        DIFUpdateData::Append {
            value: value.into(),
        }
    }

    /// Creates a new `DIFUpdateData::ListSplice` variant with the given parameters.
    pub fn list_splice(
        range: core::ops::Range<u32>,
        items: Vec<DIFValueContainer>,
    ) -> Self {
        DIFUpdateData::ListSplice {
            start: range.start,
            delete_count: range.end - range.start,
            items,
        }
    }

    pub fn with_source(&self, source_id: TransceiverId) -> DIFUpdate<'_> {
        DIFUpdate {
            source_id,
            data: Cow::Borrowed(self),
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
                ty: None,
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
                ty: None,
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
        let dif_update = DIFUpdateData::delete("age");
        let serialized = dif_update.as_json();
        assert_eq!(
            serialized,
            r#"{"kind":"delete","key":{"kind":"text","value":"age"}}"#
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
            DIFUpdateData::append(DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::Boolean(true),
                ty: None,
            }));
        let serialized = dif_update.as_json();
        assert_eq!(serialized, r#"{"kind":"append","value":{"value":true}}"#);
        let deserialized = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }
}
