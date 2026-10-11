use core::ops::Not;

use crate::values::core_value_with_classification::CoreValueWithClassification;

impl Not for &CoreValueWithClassification {
    type Output = Option<CoreValueWithClassification>;

    fn not(self) -> Self::Output {
        let inner = &self.inner;
        let neg = !inner;
        neg.map(CoreValueWithClassification::from)
    }
}
impl Not for CoreValueWithClassification {
    type Output = Option<CoreValueWithClassification>;

    fn not(self) -> Self::Output {
        (&self).not()
    }
}
