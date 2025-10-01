use serde::{Deserialize, Serialize};

use crate::dif::value::DIFValueContainer;

/// Represents a property in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DIFProperty {
    /// a simple string property
    Key(String),
    /// an integer property (e.g. an array index)
    Index(i64),
    /// any other property type
    Value(DIFValueContainer),
}

impl From<String> for DIFProperty {
    fn from(s: String) -> Self {
        DIFProperty::Key(s)
    }
}
impl From<&str> for DIFProperty {
    fn from(s: &str) -> Self {
        DIFProperty::Key(s.to_string())
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

type DIFPath = Vec<DIFProperty>;

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DIFUpdate {
    /// Represents a replacement operation for a DIF value.
    Replace(DIFValueContainer),

    /// Represents an update to a specific property of a DIF value.
    /// The `key` specifies which property to update, and `value` is the new value for that property.
    Set {
        key: DIFProperty,
        value: DIFValueContainer,
    },

    /// Represents the removal of a specific property from a DIF value.
    Remove(DIFProperty),

    /// Represents clearing all elements from a collection-type DIF value (like an array or map).
    Clear,

    /// Represents adding a new element to a collection-type DIF value (like an array or map).
    Push(DIFValueContainer),
}

impl DIFUpdate {
    /// Creates a new `DIFUpdate::Replace` variant with the given value.
    pub fn replace(value: impl Into<DIFValueContainer>) -> Self {
        DIFUpdate::Replace(value.into())
    }

    /// Creates a new `DIFUpdate::Set` variant with the given key and value.
    pub fn set(
        key: impl Into<DIFProperty>,
        value: impl Into<DIFValueContainer>,
    ) -> Self {
        DIFUpdate::Set {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Creates a new `DIFUpdate::Remove` variant with the given key.
    pub fn remove(key: impl Into<DIFProperty>) -> Self {
        DIFUpdate::Remove(key.into())
    }

    /// Creates a new `DIFUpdate::Clear` variant.
    pub fn clear() -> Self {
        DIFUpdate::Clear
    }

    /// Creates a new `DIFUpdate::Push` variant with the given value.
    pub fn push(value: impl Into<DIFValueContainer>) -> Self {
        DIFUpdate::Push(value.into())
    }
}
