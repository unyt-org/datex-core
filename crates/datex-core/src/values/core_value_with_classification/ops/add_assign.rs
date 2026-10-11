use core::ops::AddAssign;

use crate::values::{
    core_value_with_classification::CoreValueWithClassification,
    core_values::native::NativeCoreValue,
};

// TODO #119: crate a TryAddAssign trait etc.
impl<T> AddAssign<T> for CoreValueWithClassification
where
    CoreValueWithClassification: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        let rhs: CoreValueWithClassification = rhs.into();
        let res = &self.inner + &rhs.inner;
        if let Ok(res) = res {
            self.inner = res;
        } else {
            todo!("Handle add assign error")
        }
    }
}

impl AddAssign<NativeCoreValue> for CoreValueWithClassification {
    fn add_assign(&mut self, _rhs: NativeCoreValue) {
        todo!()
    }
}
