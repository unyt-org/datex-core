use crate::{
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
        core_values::boolean::Boolean,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    },
};

impl ConvertValue for bool {
    fn to_value(self) -> Value {
        CoreValue::Boolean(Boolean(self)).into()
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Boolean(Boolean(bool)),
                ..
            }) => Ok(bool),
            Value::Native(native) => {
                native.try_into_value().map_err(Value::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Boolean(Boolean(bool)),
                ..
            }) => Ok(bool),
            Value::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::Boolean(Boolean(bool)),
                ..
            }) => Ok(bool),
            Value::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, bool> {
    type Error = BorrowedValue<'a>;
    fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValue::Core(BorrowedCoreValueWithClassification {
                inner: BorrowedCoreValue::Boolean(v),
                ..
            }) => Ok(v.map(|v| &v.0)),
            BorrowedValue::Native(native) => {
                native.try_as().map_err(BorrowedValue::Native)
            }
            _ => Err(value),
        }
    }
}

impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, bool> {
    type Error = BorrowedValueMut<'a>;
    fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValueMut::Core(
                BorrowedCoreValueWithClassificationMut {
                    inner: BorrowedCoreValueMut::Boolean(v),
                    ..
                },
            ) => Ok(v.map(|v| &mut v.0)),
            BorrowedValueMut::Native(native) => {
                native.try_as_mut().map_err(BorrowedValueMut::Native)
            }
            _ => Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        traits::convert_value::ConvertValue,
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            core_values::boolean::Boolean,
            value::borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    };

    #[test]
    fn try_bool_from_core_value() {
        let core_value = CoreValue::Boolean(Boolean(true)).to_value();
        let result = core_value.try_as::<bool>();
        assert!(*result.unwrap());

        let core_value = CoreValue::Boolean(Boolean(false)).to_value();
        let result = core_value.try_into_value::<bool>();
        assert!(!result.unwrap());
    }

    #[test]
    fn try_borrow_bool_from_core_value() {
        let core_value = CoreValue::Boolean(Boolean(true)).to_value();
        let result = core_value.try_as::<bool>();
        assert!(*result.unwrap());
    }

    #[test]
    fn try_borrow_mut_bool_from_core_value() {
        let mut core_value = CoreValue::Boolean(Boolean(false)).to_value();
        let result = core_value.try_as_mut::<bool>();
        *result.unwrap() = true;
        assert_eq!(core_value, CoreValue::Boolean(Boolean(true)).to_value());
    }

    #[test]
    fn try_bool_from_native_core_value() {
        let core_value = true.to_value();
        let result = core_value.try_as::<bool>();
        assert!(*result.unwrap());
    }

    #[test]
    fn try_borrow_mut_bool_from_native_core_value() {
        let mut core_value = false.to_value();
        let result = core_value.try_as_mut::<bool>();
        *result.unwrap() = true;
        assert!(*core_value.try_as::<bool>().unwrap());
    }

    #[test]
    fn try_owned_bool_from_native_core_value() {
        let core_value = true.to_value();
        let result = core_value.try_into_value::<bool>();
        assert!(result.unwrap());
    }

    #[test]
    fn try_bool_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as::<bool>().is_none());
    }

    #[test]
    fn try_borrow_mut_bool_from_wrong_core_value_fails() {
        let mut core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as_mut::<bool>().is_none());
    }

    #[test]
    fn try_owned_bool_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_into_value::<bool>().is_err());
    }

    #[test]
    fn try_borrowed_core_value_bool() {
        let value = CoreValue::Boolean(Boolean(true)).to_value();

        let borrowed = BorrowedValue::from(&value);
        let result = Goat::<bool>::try_from(borrowed);
        assert!(*result.unwrap());
    }

    #[test]
    fn try_borrowed_core_value_mut_bool() {
        let mut value = CoreValue::Boolean(Boolean(false)).to_value();
        let borrowed = BorrowedValueMut::from(&mut value);
        let mut result = GoatMut::<bool>::try_from(borrowed).unwrap();
        *result = true;
        drop(result);
        assert_eq!(value, CoreValue::Boolean(Boolean(true)).to_value());
    }

    #[test]
    fn try_borrowed_core_value_wrong_type_fails() {
        let core_value = CoreValue::Null.into();
        let borrowed = BorrowedValue::from(&core_value);
        assert!(Goat::<bool>::try_from(borrowed).is_err());
    }

    #[test]
    fn try_borrowed_core_value_mut_wrong_type_fails() {
        let mut core_value = CoreValue::Null.into();
        let borrowed = BorrowedValueMut::from(&mut core_value);
        assert!(GoatMut::<bool>::try_from(borrowed).is_err());
    }
}
