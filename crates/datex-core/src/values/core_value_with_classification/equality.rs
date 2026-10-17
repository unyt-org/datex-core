use crate::{
    traits::{structural_eq::StructuralEq, value_eq::ValueEq},
    values::core_value_with_classification::CoreValueWithClassification,
};

impl PartialEq for CoreValueWithClassification {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
            && self.classification == other.classification
    }
}

impl StructuralEq for CoreValueWithClassification {
    fn structural_eq(&self, other: &Self) -> bool {
        self.inner.structural_eq(&other.inner)
            && self.classification.structural_eq(&other.classification)
    }
}

impl ValueEq for CoreValueWithClassification {
    fn value_eq(&self, other: &Self) -> bool {
        self.inner.value_eq(&other.inner)
            && self.classification.value_eq(&other.classification)
    }
}
