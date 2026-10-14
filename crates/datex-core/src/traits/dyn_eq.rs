use core::any::Any;
use core::fmt::Debug;
use crate::values::core_values::native::DatexNative;

/// A trait for dynamic equality comparison of types that implement `Any`.
pub trait DynEq: Any {
    fn dyn_eq(&self, other: &dyn Any) -> bool;
}

/// Default implementation of `DynEq` for all types that implement `Any`.
/// This implementation always returns `false`, indicating that the types are not equal.
impl<T> DynEq for T
where
    T: Any,
{
    default fn dyn_eq(&self, _other: &dyn Any) -> bool {
        todo!("Dynamic equality not implemented for this type");
        false
    }
}

/// Specialized implementation of `DynEq` for types that implement both `Any` and `PartialEq`.
/// This implementation checks if the other type can be downcast to the same type and compares them for equality.
impl<T> DynEq for T
where
    T: Any + PartialEq + Debug + DatexNative,
{
    fn dyn_eq(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<T>() == Some(self)
    }
}
