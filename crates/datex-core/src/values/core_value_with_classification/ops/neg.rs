use core::ops::Neg;

use crate::values::{
    core_value_with_classification::CoreValueWithClassification,
    value_container::error::ValueError,
};

impl Neg for CoreValueWithClassification {
    type Output = Result<CoreValueWithClassification, ValueError>;

    fn neg(self) -> Self::Output {
        (-self.inner).map(CoreValueWithClassification::from)
    }
}
