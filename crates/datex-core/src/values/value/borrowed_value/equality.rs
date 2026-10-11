use crate::{
    traits::{structural_eq::StructuralEq, value_eq::ValueEq},
    values::value::borrowed_value::BorrowedValue,
};
use core::ops::Deref;

/// Two values are structurally equal, if their inner values are structurally equal, regardless
/// of the actual_type of the values
impl StructuralEq for BorrowedValue<'_> {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                BorrowedValue::Core(core_self),
                BorrowedValue::Core(core_other),
            ) => core_self.structural_eq(core_other),
            (
                BorrowedValue::Native(native_self),
                BorrowedValue::Native(native_other),
            ) => {
                native_self.dyn_eq(native_other.deref()) // TODO, structural_dyn_eq
            }
            _ => {
                todo!("Structural equality not implemented for these variants")
            }
        }
    }
}

/// Value equality corresponds to partial equality:
/// Both type and inner value are the same
impl ValueEq for BorrowedValue<'_> {
    fn value_eq(&self, other: &Self) -> bool {
        match (self, other) {
            // core vs core equality
            (
                BorrowedValue::Core(core_self),
                BorrowedValue::Core(core_other),
            ) => core_self.value_eq(core_other),

            // native vs native equality
            (
                BorrowedValue::Native(native_self),
                BorrowedValue::Native(native_other),
            ) => native_self.dyn_eq(native_other.deref()),

            // core vs native equality
            (
                BorrowedValue::Core(_core_self),
                BorrowedValue::Native(_native_other),
            )
            | (
                BorrowedValue::Native(_native_other),
                BorrowedValue::Core(_core_self),
            ) => {
                todo!()
                // native_other.value.try_into_map_parts(cache)
            }
        }
    }
}

impl PartialEq for BorrowedValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.value_eq(other)
    }
}
