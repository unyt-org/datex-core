use crate::values::{core_values::native::DatexNative, value::Value};

/// This trait is completely auto derived for all types and must
/// not be implemented manually.
/// It is used to attempt to clone a value into a new [Value].
pub trait TryClone {
    /// Attempts to clone [Self] into a new [Value].
    fn try_clone(&self) -> Result<Value, ()>;
}

/// Per default, types that do not implement Clone will return an error when try_clone is called.
impl<T> TryClone for T {
    default fn try_clone(&self) -> Result<Value, ()> {
        Err(())
    }
}

/// For any [DatexNative] type that implements [Clone],
/// try_clone gets this default impl that returns a new [CoreValue] containing the cloned value.
default impl<T> TryClone for T
where
    T: Clone + DatexNative,
{
    fn try_clone(&self) -> Result<Value, ()> {
        Ok(Value::native(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::*,
        traits::{convert_value::ConvertValue, try_clone::TryClone},
        values::core_value::CoreValue,
    };

    #[test]
    fn test_try_string() {
        let value = "Hello, world!".to_string();
        let cloned_value = value.try_clone().unwrap();
        assert_eq!(cloned_value, "Hello, world!".to_string().to_value());
    }

    #[test]
    fn test_try_unclonable_type() {
        struct UnclonableType;
        let value = UnclonableType;
        let cloned_value = value.try_clone();
        assert!(cloned_value.is_err());
    }
}
