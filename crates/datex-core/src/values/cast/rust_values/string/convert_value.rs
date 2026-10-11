use crate::{
    prelude::*,
    traits::convert_value::ConvertValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::{
            CoreValue,
            borrowed_core_value::{BorrowedCoreValue, BorrowedCoreValueMut},
        },
        core_value_with_classification::{
            CoreValueWithClassification,
            borrowed_core_value_with_classification::{
                BorrowedCoreValueWithClassification,
                BorrowedCoreValueWithClassificationMut,
            },
        },
        core_values::text::Text,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
        value_container::borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};
impl ConvertValue for String {
    fn to_value(self) -> Value {
        CoreValue::Text(Text(self)).into()
    }

    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Text(Text(string)),
                ..
            }) => Ok(string),
            Value::Native(native) => {
                native.try_into_value().map_err(Value::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Text(Text(string)),
                ..
            }) => Ok(string),
            Value::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Text(Text(string)),
                ..
            }) => Ok(string),
            Value::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, str> {
    type Error = BorrowedValue<'a>;
    fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValue::Core(BorrowedCoreValueWithClassification {
                inner: BorrowedCoreValue::Text(v),
                ..
            }) => Ok(v),
            BorrowedValue::Native(native) => native
                .try_as::<String>()
                .map(|s| s.map(|s| s.as_str()))
                .map_err(BorrowedValue::Native),
            _ => Err(value),
        }
    }
}

impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, str> {
    type Error = BorrowedValueMut<'a>;
    fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValueMut::Core(
                BorrowedCoreValueWithClassificationMut {
                    inner: BorrowedCoreValueMut::Text(v),
                    ..
                },
            ) => Ok(v),
            BorrowedValueMut::Native(native) => native
                .try_as_mut::<String>()
                .map(|s| s.map(|s| s.as_mut_str()))
                .map_err(BorrowedValueMut::Native),
            _ => Err(value),
        }
    }
}

impl<'a, T: ?Sized> From<&'a T> for BorrowedValueContainer<'a>
where
    &'a T: Into<BorrowedCoreValue<'a>>,
{
    fn from(s: &'a T) -> Self {
        BorrowedValueContainer::Local(BorrowedValue::core(s))
    }
}

impl<'a, T: ?Sized> From<&'a T> for BorrowedValueContainerMut<'a>
where
    &'a T: Into<BorrowedCoreValueMut<'a>>,
{
    fn from(s: &'a T) -> Self {
        BorrowedValueContainerMut::Local(BorrowedValueMut::core(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_value::CoreValue;

    #[test]
    fn try_string_from_core_value() {
        let core_value =
            CoreValue::Text(Text("Hello, World!".to_string())).to_value();
        let result = core_value.try_as::<String>();
        assert_eq!(result.unwrap(), "Hello, World!");

        let core_value =
            CoreValue::Text(Text("Hello, World!".to_string())).to_value();
        let result = core_value.try_into_value::<String>();
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn try_borrow_string_from_core_value() {
        let core_value =
            CoreValue::Text(Text("Hello, World!".to_string())).to_value();
        let result = core_value.try_as::<String>();
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn try_borrow_mut_string_from_core_value() {
        let mut core_value = Text("Hello, World!".to_string()).to_value();
        let result = core_value.try_as_mut::<String>();

        let value = result.unwrap();
        value.push_str("!");
        assert_eq!(core_value.try_as::<String>().unwrap(), "Hello, World!!");
    }

    #[test]
    fn try_string_from_native_core_value() {
        let core_value = "Hello, World!".to_string().to_value();
        let result = core_value.try_as::<String>();
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn try_borrow_mut_string_from_native_core_value() {
        let mut core_value = "Hello, World!".to_string().to_value();
        let result = core_value.try_as_mut::<String>();
        result.unwrap().push('!');
        assert_eq!(core_value.try_as::<String>().unwrap(), "Hello, World!!");
    }

    #[test]
    fn try_owned_string_from_native_core_value() {
        let core_value = "Hello, World!".to_string().to_value();
        let result = core_value.try_into_value::<String>();
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn try_string_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as::<String>().is_none());
    }

    #[test]
    fn try_borrow_mut_string_from_wrong_core_value_fails() {
        let mut core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as_mut::<String>().is_none());
    }
}
