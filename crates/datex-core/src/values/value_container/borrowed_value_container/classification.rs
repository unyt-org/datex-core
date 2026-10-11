use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::classification::Classification,
    values::{
        value::value_classification::{
            ValueClassification,
            unresolved_value_classification::UnresolvedValueClassification,
        },
        value_container::borrowed_value_container::BorrowedValueContainer,
    },
};

impl<'a> Classification for BorrowedValueContainer<'a> {
    fn classification(
        &self,
        cache: &mut SharedReferencesCache,
    ) -> ValueClassification {
        match self {
            BorrowedValueContainer::Local(value) => {
                Classification::classification(value, cache)
            }
            BorrowedValueContainer::Shared(_shared) => {
                ValueClassification::new_unclassified()
            }
        }
    }
    fn unresolved_classification(&self) -> UnresolvedValueClassification {
        match self {
            BorrowedValueContainer::Local(value) => {
                Classification::unresolved_classification(value)
            }
            BorrowedValueContainer::Shared(_shared) => {
                UnresolvedValueClassification::new_unclassified()
            }
        }
    }
}
