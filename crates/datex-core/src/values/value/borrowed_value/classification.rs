use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::classification::Classification,
    values::value::{
        borrowed_value::BorrowedValue,
        value_classification::{
            ValueClassification,
            unresolved_value_classification::UnresolvedValueClassification,
        },
    },
};

impl<'a> Classification for BorrowedValue<'a> {
    fn classification(
        &self,
        cache: &mut SharedReferencesCache,
    ) -> ValueClassification {
        match self {
            BorrowedValue::Core(core_value) => core_value.classification(cache),
            BorrowedValue::Native(custom_value) => {
                custom_value.classification(cache)
            }
        }
    }

    fn unresolved_classification(&self) -> UnresolvedValueClassification {
        match self {
            BorrowedValue::Core(core_value) => {
                core_value.unresolved_classification()
            }
            BorrowedValue::Native(custom_value) => {
                custom_value.unresolved_classification()
            }
        }
    }
}
