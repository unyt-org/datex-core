use core::hash::{Hash, Hasher};

use crate::{
    traits::{
        structural_eq::StructuralEq,
        value_eq::ValueEq,
    },
    values::value::Value,
};

/// Two values are structurally equal, if their inner values are structurally equal, regardless
/// of the actual_type of the values
impl StructuralEq for Value {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Core(core_self), Value::Core(core_other)) => {
                core_self.structural_eq(core_other)
            }
            (Value::Native(native_self), Value::Native(native_other)) => {
                native_self.structural_eq(native_other)
            }
            _ => {
                todo!("Structural equality not implemented for these variants")
            }
        }
    }
}

/// Value equality corresponds to partial equality:
/// Both type and inner value are the same
impl ValueEq for Value {
    fn value_eq(&self, other: &Self) -> bool {
        match (self, other) {
            // core vs core equality
            (Value::Core(core_self), Value::Core(core_other)) => {
                core_self.value_eq(core_other)
            }

            // native vs native equality
            (Value::Native(native_self), Value::Native(native_other)) => {
                native_self.value_eq(native_other)
            }

            // core vs native equality
            (Value::Core(_core_self), Value::Native(_native_other))
            | (Value::Native(_native_other), Value::Core(_core_self)) => {
                todo!()
                // native_other.value.try_into_map_parts(cache)
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.value_eq(other)
    }
}

impl Eq for Value {}
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Core(core_self) => {
                core_self.hash(state);
            }
            Value::Native(native_self) => {
                native_self.datex_hash(state);
            }
        }
    }
}
