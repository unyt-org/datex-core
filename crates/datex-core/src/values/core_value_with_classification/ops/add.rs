use core::ops::Add;

use crate::values::{
    core_value_with_classification::CoreValueWithClassification,
    value_container::error::ValueError,
};

impl Add for &CoreValueWithClassification {
    type Output = Result<CoreValueWithClassification, ValueError>;
    fn add(self, rhs: &CoreValueWithClassification) -> Self::Output {
        Ok((&self.inner + &rhs.inner)?.into())
    }
}

impl Add for CoreValueWithClassification {
    type Output = Result<CoreValueWithClassification, ValueError>;
    fn add(self, rhs: CoreValueWithClassification) -> Self::Output {
        &self + &rhs
    }
}
