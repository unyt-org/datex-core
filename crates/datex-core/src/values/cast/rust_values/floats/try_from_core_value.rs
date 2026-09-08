use crate::{
    preludes::derive::{BorrowedCoreValue, BorrowedCoreValueMut},
    traits::convert_core_value::ConvertCoreValue,
    utils::{goat::Goat, goat_mut::GoatMut},
    values::{
        core_value::CoreValue,
        core_values::decimal::typed_decimal::TypedDecimal,
    },
};

impl ConvertCoreValue for f32 {
    fn try_from_core_value(value: CoreValue) -> Result<Self, CoreValue> {
        match value {
            CoreValue::TypedDecimal(TypedDecimal::F32(value)) => Ok(value.0),
            CoreValue::Native(native) => {
                native.try_into_value().map_err(CoreValue::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_core_value(value: &CoreValue) -> Result<&Self, ()> {
        match value {
            CoreValue::TypedDecimal(TypedDecimal::F32(value)) => Ok(&value.0),
            CoreValue::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_core_value(
        value: &mut CoreValue,
    ) -> Result<&mut Self, ()> {
        match value {
            CoreValue::TypedDecimal(TypedDecimal::F32(value)) => {
                Ok(&mut value.0)
            }
            CoreValue::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedCoreValue<'a>> for Goat<'a, f32> {
    type Error = ();
    fn try_from(value: BorrowedCoreValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValue::TypedDecimal(value) => {
                value.filter_map(|v| v.borrow_as_f32()).ok_or(())
            }
            BorrowedCoreValue::Native(native) => native
                .filter_map(|v| v.as_any().downcast_ref::<f32>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedCoreValueMut<'a>> for GoatMut<'a, f32> {
    type Error = ();
    fn try_from(value: BorrowedCoreValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValueMut::TypedDecimal(value) => {
                value.filter_map(|v| v.borrow_mut_as_f32()).ok_or(())
            }
            BorrowedCoreValueMut::Native(native) => native
                .filter_map(|v| v.as_any_mut().downcast_mut::<f32>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl ConvertCoreValue for f64 {
    fn try_from_core_value(value: CoreValue) -> Result<Self, CoreValue> {
        match value {
            CoreValue::TypedDecimal(TypedDecimal::F64(value)) => Ok(value.0),
            CoreValue::Native(native) => {
                native.try_into_value().map_err(CoreValue::Native)
            }
            _ => Err(value),
        }
    }

    fn try_borrow_from_core_value(value: &CoreValue) -> Result<&Self, ()> {
        match value {
            CoreValue::TypedDecimal(TypedDecimal::F64(value)) => Ok(&value.0),
            CoreValue::Native(native) => native.try_as().ok_or(()),
            _ => Err(()),
        }
    }

    fn try_borrow_mut_from_core_value(
        value: &mut CoreValue,
    ) -> Result<&mut Self, ()> {
        match value {
            CoreValue::TypedDecimal(TypedDecimal::F64(value)) => {
                Ok(&mut value.0)
            }
            CoreValue::Native(native) => native.try_as_mut().ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedCoreValue<'a>> for Goat<'a, f64> {
    type Error = ();
    fn try_from(value: BorrowedCoreValue<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValue::TypedDecimal(value) => {
                value.filter_map(|v| v.borrow_as_f64()).ok_or(())
            }
            BorrowedCoreValue::Native(native) => native
                .filter_map(|v| v.as_any().downcast_ref::<f64>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<BorrowedCoreValueMut<'a>> for GoatMut<'a, f64> {
    type Error = ();
    fn try_from(value: BorrowedCoreValueMut<'a>) -> Result<Self, Self::Error> {
        match value {
            BorrowedCoreValueMut::TypedDecimal(value) => {
                value.filter_map(|v| v.borrow_mut_as_f64()).ok_or(())
            }
            BorrowedCoreValueMut::Native(native) => native
                .filter_map(|v| v.as_any_mut().downcast_mut::<f64>())
                .ok_or(()),
            _ => Err(()),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        preludes::derive::{BorrowedCoreValue, BorrowedCoreValueMut},
        utils::{goat::Goat, goat_mut::GoatMut},
        values::{
            core_value::CoreValue,
            core_values::decimal::typed_decimal::TypedDecimal,
        },
    };

    #[test]
    fn try_f32_from_core_value() {
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into()));
        let result = core_value.try_as::<f32>();
        assert_eq!(*result.unwrap(), 1.5);

        let core_value = CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into()));
        let result = core_value.try_into_value::<f32>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_f32_from_core_value() {
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into()));

        let result = core_value.try_as::<f32>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f32_from_core_value() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into()));

        let result = core_value.try_as_mut::<f32>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f32>().unwrap(), 2.5);
    }

    #[test]
    fn try_f32_from_native_core_value() {
        let core_value = CoreValue::from(1.5f32);
        let result = core_value.try_as::<f32>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f32_from_native_core_value() {
        let mut core_value = CoreValue::from(1.5f32);

        let result = core_value.try_as_mut::<f32>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f32>().unwrap(), 2.5);
    }

    #[test]
    fn try_owned_f32_from_native_core_value() {
        let core_value = CoreValue::from(1.5f32);
        let result = core_value.try_into_value::<f32>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_f32_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null;
        assert!(core_value.try_as::<f32>().is_none());
        assert!(core_value.try_into_value::<f32>().is_err());
    }

    #[test]
    fn try_borrow_mut_f32_from_wrong_core_value_fails() {
        let mut core_value = CoreValue::Null;
        assert!(core_value.try_as_mut::<f32>().is_none());
    }

    #[test]
    fn try_borrowed_core_value_f32() {
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into()));
        let borrowed = BorrowedCoreValue::from(&core_value);
        let result = Goat::<f32>::try_from(borrowed);
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrowed_core_value_mut_f32() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F32(1.5.into()));
        let borrowed = BorrowedCoreValueMut::from(&mut core_value);
        let result = GoatMut::<f32>::try_from(borrowed);
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f32>().unwrap(), 2.5);
    }

    #[test]
    fn try_f64_from_core_value() {
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into()));
        let result = core_value.try_as::<f64>();
        assert_eq!(*result.unwrap(), 1.5);
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into()));
        let result = core_value.try_into_value::<f64>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_f64_from_core_value() {
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into()));
        let result = core_value.try_as::<f64>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f64_from_core_value() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into()));

        let result = core_value.try_as_mut::<f64>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f64>().unwrap(), 2.5);
    }

    #[test]
    fn try_f64_from_native_core_value() {
        let core_value = CoreValue::from(1.5f64);
        let result = core_value.try_as::<f64>();
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrow_mut_f64_from_native_core_value() {
        let mut core_value = CoreValue::from(1.5f64);
        let result = core_value.try_as_mut::<f64>();
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f64>().unwrap(), 2.5);
    }

    #[test]
    fn try_owned_f64_from_native_core_value() {
        let core_value = CoreValue::from(1.5f64);
        let result = core_value.try_into_value::<f64>();
        assert_eq!(result.unwrap(), 1.5);
    }

    #[test]
    fn try_f64_from_wrong_core_value_fails() {
        let core_value = CoreValue::Null;
        assert!(core_value.try_as::<f64>().is_none());
        assert!(core_value.try_into_value::<f64>().is_err());
    }

    #[test]
    fn try_borrow_mut_f64_from_wrong_core_value_fails() {
        let mut core_value = CoreValue::Null;
        assert!(core_value.try_as_mut::<f64>().is_none());
    }

    #[test]
    fn try_borrowed_core_value_f64() {
        let core_value = CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into()));
        let borrowed = BorrowedCoreValue::from(&core_value);
        let result = Goat::<f64>::try_from(borrowed);
        assert_eq!(*result.unwrap(), 1.5);
    }

    #[test]
    fn try_borrowed_core_value_mut_f64() {
        let mut core_value =
            CoreValue::TypedDecimal(TypedDecimal::F64(1.5.into()));
        let borrowed = BorrowedCoreValueMut::from(&mut core_value);
        let result = GoatMut::<f64>::try_from(borrowed);
        *result.unwrap() = 2.5;
        assert_eq!(*core_value.try_as::<f64>().unwrap(), 2.5);
    }
}
