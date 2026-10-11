use core::ops::Deref;

use crate::values::{
    core_value::CoreValue,
    core_value_with_classification::CoreValueWithClassification,
};

impl Deref for CoreValueWithClassification {
    type Target = CoreValue;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
