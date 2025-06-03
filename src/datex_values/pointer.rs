use super::{datex_type::Type, value::Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Pointer {
    pub value: Value,
    pub allowed_type: Type, // custom type for the pointer that the Datex value can get
}
