use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::classification::Classification,
    values::value::{
        Value,
        value_classification::{
            ValueClassification,
            unresolved_value_classification::UnresolvedValueClassification,
        },
    },
};

impl Classification for Value {
    fn classification(
        &self,
        cache: &mut SharedReferencesCache,
    ) -> ValueClassification {
        match self {
            Value::Core(core_value) => core_value.classification(cache),
            Value::Native(custom_value) => custom_value.classification(cache),
        }
    }

    fn unresolved_classification(&self) -> UnresolvedValueClassification {
        match self {
            Value::Core(core_value) => core_value.unresolved_classification(),
            Value::Native(custom_value) => {
                custom_value.unresolved_classification()
            }
        }
    }
}
