use crate::values::core_values::integer::Integer;

use crate::values::core_values::integer::typed_integer::TypedInteger;
use core::ops::Add;

impl Add for &Integer {
    type Output = Integer;

    fn add(self, rhs: Self) -> Self::Output {
        Integer((&self.0).add(&rhs.0))
    }
}
impl Add for Integer {
    type Output = Integer;

    fn add(self, rhs: Self) -> Self::Output {
        &self + &rhs
    }
}

impl Add for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        Some(match self {
            TypedInteger::IBig(v1) => TypedInteger::IBig(match rhs {
                TypedInteger::I8(v2) => v1 + &Integer::from(v2),
                TypedInteger::I16(v2) => v1 + &Integer::from(v2),
                TypedInteger::I32(v2) => v1 + &Integer::from(v2),
                TypedInteger::I64(v2) => v1 + &Integer::from(v2),
                TypedInteger::I128(v2) => v1 + &Integer::from(v2),
                TypedInteger::U8(v2) => v1 + &Integer::from(v2),
                TypedInteger::U16(v2) => v1 + &Integer::from(v2),
                TypedInteger::U32(v2) => v1 + &Integer::from(v2),
                TypedInteger::U64(v2) => v1 + &Integer::from(v2),
                TypedInteger::U128(v2) => v1 + &Integer::from(v2),
                TypedInteger::IBig(v2) => v1 + v2,
            }),
            TypedInteger::I8(v1) => TypedInteger::I8(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(*v2)?,
                TypedInteger::I16(v2) => {
                    i8::try_from((*v1 as i16).checked_add(*v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    i8::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    i8::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    i8::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => {
                    i8::try_from((*v1 as i16).checked_add(*v2 as i16)?).ok()?
                }
                TypedInteger::U16(v2) => {
                    i8::try_from((*v1 as i32).checked_add(*v2 as i32)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    i8::try_from((*v1 as i64).checked_add(*v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i8::try_from((*v1 as i128).checked_add(*v2 as i128)?)
                        .ok()?
                }
                TypedInteger::U128(v2) => i8::try_from(
                    (*v1 as i128).checked_add((*v2).try_into().ok()?)?,
                )
                .ok()?,
                TypedInteger::IBig(v2) => (v1).checked_add(v2.as_i8()?)?,
            }),
            TypedInteger::I16(v1) => TypedInteger::I16(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(*v2 as i16)?,
                TypedInteger::I16(v2) => v1.checked_add(*v2)?,
                TypedInteger::I32(v2) => {
                    i16::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    i16::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    i16::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(*v2 as i16)?,
                TypedInteger::U16(v2) => {
                    i16::try_from((*v1 as i32).checked_add(*v2 as i32)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    i16::try_from((*v1 as i64).checked_add(*v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i16::try_from((*v1 as i128).checked_add(*v2 as i128)?)
                        .ok()?
                }
                TypedInteger::U128(v2) => i16::try_from(
                    (*v1 as i128).checked_add((*v2).try_into().ok()?)?,
                )
                .ok()?,
                TypedInteger::IBig(v2) => (*v1).checked_add(v2.as_i16()?)?,
            }),
            TypedInteger::I32(v1) => TypedInteger::I32(match rhs {
                TypedInteger::I8(v2) => (*v1).checked_add(*v2 as i32)?,
                TypedInteger::I16(v2) => (*v1).checked_add(*v2 as i32)?,
                TypedInteger::I32(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::I64(v2) => {
                    i32::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    i32::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => (*v1).checked_add(*v2 as i32)?,
                TypedInteger::U16(v2) => (*v1).checked_add(*v2 as i32)?,
                TypedInteger::U32(v2) => {
                    i32::try_from((*v1 as i64).checked_add(*v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i32::try_from((*v1 as i128).checked_add(*v2 as i128)?)
                        .ok()?
                }
                TypedInteger::U128(v2) => i32::try_from(
                    (*v1 as i128).checked_add((*v2).try_into().ok()?)?,
                )
                .ok()?,
                TypedInteger::IBig(v2) => (*v1).checked_add(v2.as_i32()?)?,
            }),
            TypedInteger::I64(v1) => TypedInteger::I64(match rhs {
                TypedInteger::I8(v2) => (*v1).checked_add(*v2 as i64)?,
                TypedInteger::I16(v2) => (*v1).checked_add(*v2 as i64)?,
                TypedInteger::I32(v2) => (*v1).checked_add(*v2 as i64)?,
                TypedInteger::I64(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::I128(v2) => {
                    i64::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => {
                    i64::from((*v1 as i16).checked_add(*v2 as i16)?)
                }
                TypedInteger::U16(v2) => {
                    i64::from((*v1 as i32).checked_add(*v2 as i32)?)
                }
                TypedInteger::U32(v2) => (*v1).checked_add(*v2 as i64)?,
                TypedInteger::U64(v2) => {
                    i64::try_from((*v1 as i128).checked_add(*v2 as i128)?)
                        .ok()?
                }
                TypedInteger::U128(v2) => i64::try_from(
                    (*v1 as i128).checked_add((*v2).try_into().ok()?)?,
                )
                .ok()?,
                TypedInteger::IBig(v2) => (*v1).checked_add(v2.as_i64()?)?,
            }),
            TypedInteger::I128(v1) => TypedInteger::I128(match rhs {
                TypedInteger::I8(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::I16(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::I32(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::I64(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::I128(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::U8(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::U16(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::U32(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::U64(v2) => (*v1).checked_add(*v2 as i128)?,
                TypedInteger::U128(v2) => {
                    (*v1).checked_add((*v2).try_into().ok()?)?
                }
                TypedInteger::IBig(v2) => (*v1).checked_add(v2.as_i128()?)?,
            }),
            TypedInteger::U8(v1) => TypedInteger::U8(match rhs {
                TypedInteger::I8(v2) => {
                    u8::try_from((*v1 as i8).checked_add(*v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u8::try_from((*v1 as i16).checked_add(*v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u8::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u8::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u8::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::U16(v2) => {
                    u8::try_from((*v1 as u16).checked_add(*v2)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    u8::try_from((*v1 as u32).checked_add(*v2)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    u8::try_from((*v1 as u64).checked_add(*v2)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    u8::try_from((*v1 as u128).checked_add(*v2)?).ok()?
                }
                TypedInteger::IBig(v2) => {
                    u8::try_from(((*v1) as u16).checked_add((*v2).as_u16()?)?)
                        .ok()?
                }
            }),
            TypedInteger::U16(v1) => TypedInteger::U16(match rhs {
                TypedInteger::I8(v2) => {
                    u16::try_from((*v1 as i8).checked_add(*v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u16::try_from((*v1 as i16).checked_add(*v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u16::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u16::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u16::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => (*v1).checked_add(*v2 as u16)?,
                TypedInteger::U16(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::U32(v2) => {
                    u16::try_from((*v1 as u32).checked_add(*v2)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    u16::try_from((*v1 as u64).checked_add(*v2)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    u16::try_from((*v1 as u128).checked_add(*v2)?).ok()?
                }
                TypedInteger::IBig(v2) => {
                    u16::try_from((*v1 as u32).checked_add((*v2).as_u32()?)?)
                        .ok()?
                }
            }),

            TypedInteger::U32(v1) => TypedInteger::U32(match rhs {
                TypedInteger::I8(v2) => {
                    u32::try_from((*v1 as i8).checked_add(*v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u32::try_from((*v1 as i16).checked_add(*v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u32::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u32::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u32::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => (*v1).checked_add(*v2 as u32)?,
                TypedInteger::U16(v2) => (*v1).checked_add(*v2 as u32)?,
                TypedInteger::U32(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::U64(v2) => {
                    u32::try_from((*v1 as u64).checked_add(*v2)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    u32::try_from((*v1 as u128).checked_add(*v2)?).ok()?
                }
                TypedInteger::IBig(v2) => {
                    u32::try_from((*v1 as u64).checked_add((*v2).as_u64()?)?)
                        .ok()?
                }
            }),
            TypedInteger::U64(v1) => TypedInteger::U64(match rhs {
                TypedInteger::I8(v2) => {
                    u64::try_from((*v1 as i8).checked_add(*v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u64::try_from((*v1 as i16).checked_add(*v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u64::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u64::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u64::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => (*v1).checked_add(*v2 as u64)?,
                TypedInteger::U16(v2) => (*v1).checked_add(*v2 as u64)?,
                TypedInteger::U32(v2) => (*v1).checked_add(*v2 as u64)?,
                TypedInteger::U64(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::U128(v2) => {
                    u64::try_from((*v1 as u128).checked_add(*v2)?).ok()?
                }
                TypedInteger::IBig(v2) => {
                    u64::try_from((*v1 as u128).checked_add((*v2).as_u128()?)?)
                        .ok()?
                }
            }),
            TypedInteger::U128(v1) => TypedInteger::U128(match rhs {
                TypedInteger::I8(v2) => {
                    u128::try_from((*v1 as i8).checked_add(*v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u128::try_from((*v1 as i16).checked_add(*v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u128::try_from((*v1 as i32).checked_add(*v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u128::try_from((*v1 as i64).checked_add(*v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u128::try_from((*v1 as i128).checked_add(*v2)?).ok()?
                }
                TypedInteger::U8(v2) => (*v1).checked_add(*v2 as u128)?,
                TypedInteger::U16(v2) => (*v1).checked_add(*v2 as u128)?,
                TypedInteger::U32(v2) => (*v1).checked_add(*v2 as u128)?,
                TypedInteger::U64(v2) => (*v1).checked_add(*v2 as u128)?,
                TypedInteger::U128(v2) => (*v1).checked_add(*v2)?,
                TypedInteger::IBig(v2) => {
                    u128::try_from((*v1 as i128).checked_add((*v2).as_i128()?)?)
                        .ok()?
                }
            }),
        })
    }
}

impl Add for TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        &self + &rhs
    }
}
