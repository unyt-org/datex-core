use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum DatexType {
    Null,
    Text,
    I8,
    Bool,
    Array,
}
