use super::{datex_type::CoreValueType, value::Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Pointer {
    pub value: Value,
    pub allowed_type: CoreValueType, // custom type for the pointer that the Datex value can get
}
