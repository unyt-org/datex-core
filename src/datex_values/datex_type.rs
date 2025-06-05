use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum CoreValueType {
    Null,
    Text,
    I8,
    Bool,
    Array,
    Object,
    Endpoint,
}
