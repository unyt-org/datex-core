use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::classification::Classification,
    values::{
        core_value_with_classification::CoreValueWithClassification,
        value::value_classification::{
            ValueClassification,
            unresolved_value_classification::UnresolvedValueClassification,
        },
    },
};

impl Classification for CoreValueWithClassification {
    fn classification(
        &self,
        _cache: &mut SharedReferencesCache,
    ) -> ValueClassification {
        self.classification.clone()
    }

    fn unresolved_classification(&self) -> UnresolvedValueClassification {
        self.classification.clone().into()
    }
}
