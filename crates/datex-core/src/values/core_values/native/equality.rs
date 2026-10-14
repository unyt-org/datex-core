use core::any::Any;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::values::core_values::native::NativeCoreValue;

impl ValueEq for NativeCoreValue {
    fn value_eq(&self, other: &Self) -> bool {
        self.value.dyn_eq(&*other.value as &dyn Any)
    }
}

impl PartialEq for NativeCoreValue {
    fn eq(&self, other: &Self) -> bool {
        self.value_eq(other)
    }
}

// TODO: distinguish between different equality traits for dyn equality
impl StructuralEq for NativeCoreValue {
    fn structural_eq(&self, other: &Self) -> bool {
        self.value.dyn_eq(&*other.value as &dyn Any)
    }
}