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
        core_values::integer::typed_integer::TypedInteger,
        value::{
            Value,
            borrowed_value::{BorrowedValue, BorrowedValueMut},
        },
    },
};

macro_rules! impl_pointer_sized_core_value_conversions {
    ($($ty:ident => $variant:ident, $repr:ty, $borrow:ident, $borrow_mut:ident;)* $(,)?) => {
        $(
            const _: () = {
                assert!(core::mem::size_of::<$ty>() == core::mem::size_of::<$repr>());
                assert!(core::mem::align_of::<$ty>() == core::mem::align_of::<$repr>());
            };

            impl ConvertValue for $ty {
                fn to_value(self) -> Value {
                    Value::Core(CoreValueWithClassification::new(CoreValue::TypedInteger(TypedInteger::$variant(self as $repr))))
                }
                fn try_from_value(value: Value) -> Result<Self, Value> {
                    match value {
                        Value::Core(CoreValueWithClassification {inner: CoreValue::TypedInteger(TypedInteger::$variant(v)), ..}) => Ok(v as $ty),
                        Value::Native(native) => native.try_into_value().map_err(Value::Native),
                        _ => Err(value),
                    }
                }

                fn try_borrow_from_value(value: &Value) -> Result<&Self, ()> {
                    match value {
                        // SAFETY: checked above to have identical size and alignment,
                        // and both are plain integers with no padding or niches.
                        Value::Core(CoreValueWithClassification {inner: CoreValue::TypedInteger(TypedInteger::$variant(v)), ..}) => {
                            Ok(unsafe { &*(v as *const $repr as *const $ty) })
                        }
                        Value::Native(native) => native.try_as().ok_or(()),
                        _ => Err(()),
                    }
                }

                fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()> {
                    match value {
                        // SAFETY: as above; every bit pattern is valid for both types,
                        // so writes through the resulting reference are also fine.
                        Value::Core(CoreValueWithClassification {inner: CoreValue::TypedInteger(TypedInteger::$variant(v)), ..}) => {
                            Ok(unsafe { &mut *(v as *mut $repr as *mut $ty) })
                        }
                        Value::Native(native) => native.try_as_mut().ok_or(()),
                        _ => Err(()),
                    }
                }
            }

            impl<'a> TryFrom<BorrowedValue<'a>> for Goat<'a, $ty> {
                type Error = ();
                fn try_from(value: BorrowedValue<'a>) -> Result<Self, Self::Error> {
                    match value {
                        BorrowedValue::Core(BorrowedCoreValueWithClassification {inner: BorrowedCoreValue::TypedInteger(v), ..}) => v
                            // SAFETY: see above.
                            .filter_map(|v| {
                                v.$borrow().map(|v| unsafe { &*(v as *const $repr as *const $ty) })
                            })
                            .ok_or(()),
                        BorrowedValue::Native(native) => native
                            .filter_map(|v| v.as_any().downcast_ref::<$ty>())
                            .ok_or(()),
                        _ => Err(()),
                    }
                }
            }

            impl<'a> TryFrom<BorrowedValueMut<'a>> for GoatMut<'a, $ty> {
                type Error = ();
                fn try_from(value: BorrowedValueMut<'a>) -> Result<Self, Self::Error> {
                    match value {
                        BorrowedValueMut::Core(BorrowedCoreValueWithClassificationMut {inner: BorrowedCoreValueMut::TypedInteger(v), ..}) => v
                            // SAFETY: see above.
                            .filter_map(|v| {
                                v.$borrow_mut().map(|v| unsafe { &mut *(v as *mut $repr as *mut $ty) })
                            })
                            .ok_or(()),
                        BorrowedValueMut::Native(native) => native
                            .filter_map(|v| v.as_any_mut().downcast_mut::<$ty>())
                            .ok_or(()),
                        _ => Err(()),
                    }
                }
            }
        )*
    };
}

#[cfg(target_pointer_width = "64")]
impl_pointer_sized_core_value_conversions! {
    usize => U64, u64, borrow_as_u64, borrow_mut_as_u64;
    isize => I64, i64, borrow_as_i64, borrow_mut_as_i64;
}

#[cfg(target_pointer_width = "32")]
impl_pointer_sized_core_value_conversions! {
    usize => U32, u32, borrow_as_u32, borrow_mut_as_u32;
    isize => I32, i32, borrow_as_i32, borrow_mut_as_i32;
}

#[cfg(target_pointer_width = "16")]
impl_pointer_sized_core_value_conversions! {
    usize => U16, u16, borrow_as_u16, borrow_mut_as_u16;
    isize => I16, i16, borrow_as_i16, borrow_mut_as_i16;
}
