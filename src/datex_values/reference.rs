use crate::datex_values::pointer::Pointer;

use super::{datex_type::CoreValueType, value::Value};

// FIXME a reference clone should not clone the value
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reference {
    // the value that this reference points to
    pub value: Value,

    // pointer information
    // this can be None if only a local reference is needed
    pointer: Option<Pointer>,

    /// custom type for the pointer that the Datex value is allowed to reference
    pub allowed_type: CoreValueType,
}

impl Reference {
    pub fn pointer_id(&self) -> Option<u64> {
        self.pointer.as_ref().map(|p| p.pointer_id())
    }
}
