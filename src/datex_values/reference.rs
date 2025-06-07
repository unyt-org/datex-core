use super::{datex_type::CoreValueType, value::Value};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reference {
    pub value: Value,
    // pointer
    pub allowed_type: CoreValueType, // custom type for the pointer that the Datex value can get
}
