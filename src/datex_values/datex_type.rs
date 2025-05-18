use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Type {
    Null,
    Text,
    I8,
    Bool,
    Array,
    Endpoint,
}
