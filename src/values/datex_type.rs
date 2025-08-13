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

    // TODO: distinguish between Decimal typed variant and Decimal default type? (Decimal/BigDecimal)
    Decimal,
    F32,
    F64,

    Boolean,
    Array,
    Object,
    Endpoint,
    Tuple,
}
