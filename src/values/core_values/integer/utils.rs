use core::prelude::rust_2024::*;
use crate::values::core_values::integer::typed_integer::TypedInteger;

pub fn smallest_fitting_unsigned(val: u128) -> TypedInteger {
    if val <= u8::MAX as u128 {
        TypedInteger::U8(val as u8)
    } else if val <= u16::MAX as u128 {
        TypedInteger::U16(val as u16)
    } else if val <= u32::MAX as u128 {
        TypedInteger::U32(val as u32)
    } else if val <= u64::MAX as u128 {
        TypedInteger::U64(val as u64)
    } else {
        TypedInteger::U128(val)
    }
}

pub fn smallest_fitting_signed(val: i128) -> TypedInteger {
    if val >= i8::MIN as i128 && val <= i8::MAX as i128 {
        TypedInteger::I8(val as i8)
    } else if val >= i16::MIN as i128 && val <= i16::MAX as i128 {
        TypedInteger::I16(val as i16)
    } else if val >= i32::MIN as i128 && val <= i32::MAX as i128 {
        TypedInteger::I32(val as i32)
    } else if val >= i64::MIN as i128 && val <= i64::MAX as i128 {
        TypedInteger::I64(val as i64)
    } else {
        TypedInteger::I128(val)
    }
}
