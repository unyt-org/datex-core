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

type DIFPath = Vec<DIFProperty>;

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DIFUpdate {
    /// Represents a replacement operation for a DIF value.
    /// If `path` is `None`, the entire value is replaced.
    /// If `path` is `Some`, only the sub-value at the specified path is replaced.
    Replace {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<DIFPath>,
        value: DIFValueContainer,
    },

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
