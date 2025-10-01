use serde::{Deserialize, Serialize};

use crate::dif::value::DIFValueContainer;

/// Represents a property in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DIFProperty {
    /// a simple string property
    Text(String),
    /// an integer property (e.g. an array index)
    Integer(i64),
    /// any other property type
    Value(DIFValueContainer),
}

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DIFUpdate {
    Replace {
        value: DIFValueContainer,
    },
    UpdateProperty {
        property: DIFProperty,
        value: DIFValueContainer,
    },
    Push {
        value: DIFValueContainer,
    },
}
