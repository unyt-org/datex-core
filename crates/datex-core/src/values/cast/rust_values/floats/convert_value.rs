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
        core_values::decimal::typed_decimal::TypedDecimal,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    },
};

impl ConvertValue for f32 {
    fn to_value(self) -> Value {
        CoreValue::TypedDecimal(TypedDecimal::F32(self.into())).into()
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::TypedDecimal(TypedDecimal::F32(value)),
                ..
            }) => Ok(value.0),
            Value::Native(native) => {
                native.try_into_value().map_err(Value::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::TypedDecimal(TypedDecimal::F32(value)),
                ..
            }) => Ok(&value.0),
            Value::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::TypedDecimal(TypedDecimal::F32(value)),
                ..
            }) => Ok(&mut value.0),
            Value::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, f32> {
    type Error = BorrowedValue<'a>;
    fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValue::Core(BorrowedCoreValueWithClassification {
                inner: BorrowedCoreValue::TypedDecimal(value),
                classification,
            }) => value.try_filter_map(|v| v.borrow_as_f32()).map_err(|v| {
                BorrowedValue::Core(BorrowedCoreValueWithClassification {
                    inner: BorrowedCoreValue::TypedDecimal(v),
                    classification,
                })
            }),
            BorrowedValue::Native(native) => {
                native.try_as().map_err(BorrowedValue::Native)
            }
            _ => Err(value),
        }
    }
}

impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, f32> {
    type Error = BorrowedValueMut<'a>;
    fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValueMut::Core(
                BorrowedCoreValueWithClassificationMut {
                    inner: BorrowedCoreValueMut::TypedDecimal(value),
                    classification,
                },
            ) => value
                .try_filter_map(|v| v.borrow_mut_as_f32())
                .map_err(|v| {
                    BorrowedValueMut::Core(
                        BorrowedCoreValueWithClassificationMut {
                            inner: BorrowedCoreValueMut::TypedDecimal(v),
                            classification,
                        },
                    )
                }),
            BorrowedValueMut::Native(native) => {
                native.try_as_mut().map_err(BorrowedValueMut::Native)
            }
            _ => Err(value),
        }
    }
}

impl ConvertValue for f64 {
    fn to_value(self) -> Value {
        CoreValue::TypedDecimal(TypedDecimal::F64(self.into())).into()
    }
    fn try_from_value(value: Value) -> Result<Self, Value> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::TypedDecimal(TypedDecimal::F64(value)),
                ..
            }) => Ok(value.0),
            Value::Native(native) => {
                native.try_into_value().map_err(Value::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::TypedDecimal(TypedDecimal::F64(value)),
                ..
            }) => Ok(&value.0),
            Value::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
        match value {
            Value::Core(CoreValueWithClassification {
                inner: CoreValue::TypedDecimal(TypedDecimal::F64(value)),
                ..
            }) => Ok(&mut value.0),
            Value::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, f64> {
    type Error = ();
    fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValue::Core(BorrowedCoreValueWithClassification {
                inner: BorrowedCoreValue::TypedDecimal(value),
                ..
            }) => value.filter_map(|v| v.borrow_as_f64()).ok_or(()),
            BorrowedValue::Native(native) => native
                .filter_map(|v| v.as_any().downcast_ref::<f64>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, f64> {
    type Error = ();
    fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedValueMut::Core(
                BorrowedCoreValueWithClassificationMut {
                    inner: BorrowedCoreValueMut::TypedDecimal(value),
                    ..
                },
            ) => value.filter_map(|v| v.borrow_mut_as_f64()).ok_or(()),
            BorrowedValueMut::Native(native) => native
                .filter_map(|v| v.as_any_mut().downcast_mut::<f64>())
                .ok_or(()),
            _ => Err(()),
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
            core_values::decimal::typed_decimal::TypedDecimal,
            value::borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    };

    #[test]
    fn try_f32_from_core_value() {
        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into())).to_value();
        let result = core_value.try_as::<f32>();
        assert_eq!(*result.unwrap(), 1.5);

        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into())).to_value();
        let result = core_value.try_into_value::<f32>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_f32_from_core_value() {
        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into())).to_value();

        let result = core_value.try_as::<f32>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f32_from_core_value() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into())).to_value();

        let result = core_value.try_as_mut::<f32>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f32>().unwrap(), 2.5);
    }

    #[test]
    fn try_f32_from_native_core_value() {
        let core_value = 1.5f32.to_value();
        let result = core_value.try_as::<f32>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f32_from_native_core_value() {
        let mut core_value = 1.5f32.to_value();

        let result = core_value.try_as_mut::<f32>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f32>().unwrap(), 2.5);
    }

    #[test]
    fn try_owned_f32_from_native_core_value() {
        let core_value = 1.5f32.to_value();
        let result = core_value.try_into_value::<f32>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_f32_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as::<f32>().is_none());
        assert!(core_value.try_into_value::<f32>().is_err());
    }

    #[test]
    fn try_borrow_mut_f32_from_wrong_core_value_fails() {
        let mut core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as_mut::<f32>().is_none());
    }

    #[test]
    fn try_borrowed_core_value_f32() {
        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into())).to_value();
        let borrowed = BorrowedValue::from(&core_value);
        let result = Goat::<f32>::try_from(borrowed);
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrowed_core_value_mut_f32() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into())).to_value();
        let borrowed = BorrowedValueMut::from(&mut core_value);
        let result = GoatMut::<f32>::try_from(borrowed);
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f32>().unwrap(), 2.5);
    }

    #[test]
    fn try_f64_from_core_value() {
        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into())).to_value();
        let result = core_value.try_as::<f64>();
        assert_eq!(*result.unwrap(), 1.5);
        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into())).to_value();
        let result = core_value.try_into_value::<f64>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_f64_from_core_value() {
        let core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into())).to_value();
        let result = core_value.try_as::<f64>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f64_from_core_value() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into())).to_value();

        let result = core_value.try_as_mut::<f64>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f64>().unwrap(), 2.5);
    }

    #[test]
    fn try_f64_from_native_core_value() {
        let core_value = 1.5f64.to_value();
        let result = core_value.try_as::<f64>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f64_from_native_core_value() {
        let mut core_value = 1.5f64.to_value();
        let result = core_value.try_as_mut::<f64>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f64>().unwrap(), 2.5);
    }

    #[test]
    fn try_owned_f64_from_native_core_value() {
        let core_value = 1.5f64.to_value();
        let result = core_value.try_into_value::<f64>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_f64_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as::<f64>().is_none());
        assert!(core_value.try_into_value::<f64>().is_err());
    }

    #[test]
    fn try_borrow_mut_f64_from_wrong_core_value_fails() {
        let mut core_value = CoreValue::Null.to_value();
        assert!(core_value.try_as_mut::<f64>().is_none());
    }

    #[test]
    fn try_borrowed_core_value_f64() {
        let core_value = 1.5f64.to_value();
        let borrowed = BorrowedValue::from(&core_value);
        let result = Goat::<f64>::try_from(borrowed);
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrowed_core_value_mut_f64() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into())).to_value();
        let borrowed = BorrowedValueMut::from(&mut core_value);
        let result = GoatMut::<f64>::try_from(borrowed);
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f64>().unwrap(), 2.5);
    }
}
