use crate::{
    traits::{structural_eq::StructuralEq, value_eq::ValueEq},
    values::core_value::borrowed_core_value::BorrowedCoreValue,
};

impl StructuralEq for BorrowedCoreValue<'_> {
    fn structural_eq(&self, other: &Self) -> bool {
        // TODO: optimize?
        self.clone_to_core_value()
            .structural_eq(&other.clone_to_core_value())
    }
}

impl ValueEq for BorrowedCoreValue<'_> {
    fn value_eq(&self, other: &Self) -> bool {
        self.clone_to_core_value()
            .structural_eq(&other.clone_to_core_value())
    }
}

impl PartialEq for BorrowedCoreValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.clone_to_core_value()
            .structural_eq(&other.clone_to_core_value())
    }
}

impl Eq for BorrowedCoreValue<'_> {}
