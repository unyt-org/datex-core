use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(
    Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum CoreValueType {
    Null,
    Text,

    Integer,
    I8,
    I16,
    I32,
    I64,
    I128,

    U8,
    U16,
    U32,
    U64,
    U128,

    Decimal,
    F32,
    F64,

    Bool,
    Array,
    Object,
    Endpoint,
    Tuple,
}
