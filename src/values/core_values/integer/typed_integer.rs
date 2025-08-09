use std::{
    fmt::Display,
    hash::Hash,
    ops::{Add, AddAssign, Neg, Sub},
};
use serde::{Deserialize, Serialize};
use crate::values::{
    core_value_trait::CoreValueTrait,
    core_values::integer::utils::{
        smallest_fitting_signed, smallest_fitting_unsigned,
    },
    traits::structural_eq::StructuralEq,
    value_container::{ValueContainer, ValueError},
};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, Serialize, Deserialize)]
pub enum TypedInteger {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
}

impl TypedInteger {
    pub fn to_smallest_fitting(&self) -> TypedInteger {
        if self.is_unsigned() {
            smallest_fitting_unsigned(self.as_u128())
        } else {
            smallest_fitting_signed(self.as_i128().unwrap())
        }
    }

    fn subtype(&self) -> &'static str {
        match self {
            TypedInteger::I8(_) => "/i8",
            TypedInteger::I16(_) => "/i16",
            TypedInteger::I32(_) => "/i32",
            TypedInteger::I64(_) => "/i64",
            TypedInteger::I128(_) => "/i128",
            TypedInteger::U8(_) => "/u8",
            TypedInteger::U16(_) => "/u16",
            TypedInteger::U32(_) => "/u32",
            TypedInteger::U64(_) => "/u64",
            TypedInteger::U128(_) => "/u128",
        }
    }

    pub fn as_i8(&self) -> Option<i8> {
        match self {
            TypedInteger::I8(v) => i8::try_from(*v).ok(),
            TypedInteger::I16(v) => i8::try_from(*v).ok(),
            TypedInteger::I32(v) => i8::try_from(*v).ok(),
            TypedInteger::I64(v) => i8::try_from(*v).ok(),
            TypedInteger::I128(v) => i8::try_from(*v).ok(),

            TypedInteger::U8(v) => i8::try_from(*v).ok(),
            TypedInteger::U16(v) => i8::try_from(*v).ok(),
            TypedInteger::U32(v) => i8::try_from(*v).ok(),
            TypedInteger::U64(v) => i8::try_from(*v).ok(),
            TypedInteger::U128(v) => i8::try_from(*v).ok(),
        }
    }
    pub fn as_i16(&self) -> Option<i16> {
        match self {
            TypedInteger::I8(v) => i16::try_from(*v).ok(),
            TypedInteger::I16(v) => i16::try_from(*v).ok(),
            TypedInteger::I32(v) => i16::try_from(*v).ok(),
            TypedInteger::I64(v) => i16::try_from(*v).ok(),
            TypedInteger::I128(v) => i16::try_from(*v).ok(),

            TypedInteger::U8(v) => i16::try_from(*v).ok(),
            TypedInteger::U16(v) => i16::try_from(*v).ok(),
            TypedInteger::U32(v) => i16::try_from(*v).ok(),
            TypedInteger::U64(v) => i16::try_from(*v).ok(),
            TypedInteger::U128(v) => i16::try_from(*v).ok(),
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            TypedInteger::I8(v) => i32::try_from(*v).ok(),
            TypedInteger::I16(v) => i32::try_from(*v).ok(),
            TypedInteger::I32(v) => i32::try_from(*v).ok(),
            TypedInteger::I64(v) => i32::try_from(*v).ok(),
            TypedInteger::I128(v) => i32::try_from(*v).ok(),

            TypedInteger::U8(v) => i32::try_from(*v).ok(),
            TypedInteger::U16(v) => i32::try_from(*v).ok(),
            TypedInteger::U32(v) => i32::try_from(*v).ok(),
            TypedInteger::U64(v) => i32::try_from(*v).ok(),
            TypedInteger::U128(v) => i32::try_from(*v).ok(),
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TypedInteger::I8(v) => i64::try_from(*v).ok(),
            TypedInteger::I16(v) => i64::try_from(*v).ok(),
            TypedInteger::I32(v) => i64::try_from(*v).ok(),
            TypedInteger::I64(v) => i64::try_from(*v).ok(),
            TypedInteger::I128(v) => i64::try_from(*v).ok(),

            TypedInteger::U8(v) => i64::try_from(*v).ok(),
            TypedInteger::U16(v) => i64::try_from(*v).ok(),
            TypedInteger::U32(v) => i64::try_from(*v).ok(),
            TypedInteger::U64(v) => i64::try_from(*v).ok(),
            TypedInteger::U128(v) => i64::try_from(*v).ok(),
        }
    }

    // FIXME #125 we should probably allow casting to i128 from u128
    pub fn as_u128(&self) -> u128 {
        match self {
            TypedInteger::U8(v) => *v as u128,
            TypedInteger::U16(v) => *v as u128,
            TypedInteger::U32(v) => *v as u128,
            TypedInteger::U64(v) => *v as u128,
            TypedInteger::U128(v) => *v,
            _ => unreachable!("as_u128 called on a signed integer"),
        }
    }

    pub fn as_i128(&self) -> Option<i128> {
        match self {
            TypedInteger::I8(v) => Some(*v as i128),
            TypedInteger::I16(v) => Some(*v as i128),
            TypedInteger::I32(v) => Some(*v as i128),
            TypedInteger::I64(v) => Some(*v as i128),
            TypedInteger::I128(v) => Some(*v),
            TypedInteger::U8(v) => Some(*v as i128),
            TypedInteger::U16(v) => Some(*v as i128),
            TypedInteger::U32(v) => Some(*v as i128),
            TypedInteger::U64(v) => Some(*v as i128),
            TypedInteger::U128(v) => Some(i128::try_from(*v).ok()?),
        }
    }
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            TypedInteger::I8(_)
                | TypedInteger::I16(_)
                | TypedInteger::I32(_)
                | TypedInteger::I64(_)
                | TypedInteger::I128(_)
        )
    }
    pub fn is_unsigned(&self) -> bool {
        !self.is_signed()
    }
}

impl Display for TypedInteger {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TypedInteger::I8(v) => write!(f, "{v}"),
            TypedInteger::I16(v) => write!(f, "{v}"),
            TypedInteger::I32(v) => write!(f, "{v}"),
            TypedInteger::I64(v) => write!(f, "{v}"),
            TypedInteger::I128(v) => write!(f, "{v}"),
            TypedInteger::U8(v) => write!(f, "{v}"),
            TypedInteger::U16(v) => write!(f, "{v}"),
            TypedInteger::U32(v) => write!(f, "{v}"),
            TypedInteger::U64(v) => write!(f, "{v}"),
            TypedInteger::U128(v) => write!(f, "{v}"),
        }
    }
}

impl CoreValueTrait for TypedInteger {}

impl StructuralEq for TypedInteger {
    fn structural_eq(&self, other: &Self) -> bool {
        if self.is_unsigned() && other.is_unsigned() {
            self.as_u128() == other.as_u128()
        } else {
            self.as_i128() == other.as_i128()
        }
    }
}

impl Add for TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        Some(match self {
            TypedInteger::I8(v1) => TypedInteger::I8(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(v2)?,
                TypedInteger::I16(v2) => {
                    i8::try_from((v1 as i16).checked_add(v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    i8::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    i8::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    i8::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => {
                    i8::try_from((v1 as i16).checked_add(v2 as i16)?).ok()?
                }
                TypedInteger::U16(v2) => {
                    i8::try_from((v1 as i32).checked_add(v2 as i32)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    i8::try_from((v1 as i64).checked_add(v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i8::try_from((v1 as i128).checked_add(v2 as i128)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    i8::try_from((v1 as i128).checked_add(v2.try_into().ok()?)?)
                        .ok()?
                }
            }),
            TypedInteger::I16(v1) => TypedInteger::I16(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(v2 as i16)?,
                TypedInteger::I16(v2) => v1.checked_add(v2)?,
                TypedInteger::I32(v2) => {
                    i16::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    i16::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    i16::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => {
                    i16::try_from(v1.checked_add(v2 as i16)?).ok()?
                }
                TypedInteger::U16(v2) => {
                    i16::try_from((v1 as i32).checked_add(v2 as i32)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    i16::try_from((v1 as i64).checked_add(v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i16::try_from((v1 as i128).checked_add(v2 as i128)?).ok()?
                }
                TypedInteger::U128(v2) => i16::try_from(
                    (v1 as i128).checked_add(v2.try_into().ok()?)?,
                )
                .ok()?,
            }),
            TypedInteger::I32(v1) => TypedInteger::I32(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(v2 as i32)?,
                TypedInteger::I16(v2) => v1.checked_add(v2 as i32)?,
                TypedInteger::I32(v2) => v1.checked_add(v2)?,
                TypedInteger::I64(v2) => {
                    i32::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    i32::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(v2 as i32)?,
                TypedInteger::U16(v2) => v1.checked_add(v2 as i32)?,
                TypedInteger::U32(v2) => {
                    i32::try_from((v1 as i64).checked_add(v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i32::try_from((v1 as i128).checked_add(v2 as i128)?).ok()?
                }
                TypedInteger::U128(v2) => i32::try_from(
                    (v1 as i128).checked_add(v2.try_into().ok()?)?,
                )
                .ok()?,
            }),
            TypedInteger::I64(v1) => TypedInteger::I64(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(v2 as i64)?,
                TypedInteger::I16(v2) => v1.checked_add(v2 as i64)?,
                TypedInteger::I32(v2) => v1.checked_add(v2 as i64)?,
                TypedInteger::I64(v2) => v1.checked_add(v2)?,
                TypedInteger::I128(v2) => {
                    i64::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => {
                    i64::try_from((v1 as i16).checked_add(v2 as i16)?).ok()?
                }
                TypedInteger::U16(v2) => {
                    i64::try_from((v1 as i32).checked_add(v2 as i32)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    i64::try_from(v1.checked_add(v2 as i64)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    i64::try_from((v1 as i128).checked_add(v2 as i128)?).ok()?
                }
                TypedInteger::U128(v2) => i64::try_from(
                    (v1 as i128).checked_add(v2.try_into().ok()?)?,
                )
                .ok()?,
            }),
            TypedInteger::I128(v1) => TypedInteger::I128(match rhs {
                TypedInteger::I8(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::I16(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::I32(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::I64(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::I128(v2) => v1.checked_add(v2)?,
                TypedInteger::U8(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::U16(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::U32(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::U64(v2) => v1.checked_add(v2 as i128)?,
                TypedInteger::U128(v2) => {
                    v1.checked_add(v2.try_into().ok()?)?
                }
            }),
            TypedInteger::U8(v1) => TypedInteger::U8(match rhs {
                TypedInteger::I8(v2) => {
                    u8::try_from((v1 as i8).checked_add(v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u8::try_from((v1 as i16).checked_add(v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u8::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u8::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u8::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(v2)?,
                TypedInteger::U16(v2) => {
                    u8::try_from((v1 as u16).checked_add(v2)?).ok()?
                }
                TypedInteger::U32(v2) => {
                    u8::try_from((v1 as u32).checked_add(v2)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    u8::try_from((v1 as u64).checked_add(v2)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    u8::try_from((v1 as u128).checked_add(v2)?).ok()?
                }
            }),
            TypedInteger::U16(v1) => TypedInteger::U16(match rhs {
                TypedInteger::I8(v2) => {
                    u16::try_from((v1 as i8).checked_add(v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u16::try_from((v1 as i16).checked_add(v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u16::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u16::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u16::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(v2 as u16)?,
                TypedInteger::U16(v2) => v1.checked_add(v2)?,
                TypedInteger::U32(v2) => {
                    u16::try_from((v1 as u32).checked_add(v2)?).ok()?
                }
                TypedInteger::U64(v2) => {
                    u16::try_from((v1 as u64).checked_add(v2)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    u16::try_from((v1 as u128).checked_add(v2)?).ok()?
                }
            }),

            TypedInteger::U32(v1) => TypedInteger::U32(match rhs {
                TypedInteger::I8(v2) => {
                    u32::try_from((v1 as i8).checked_add(v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u32::try_from((v1 as i16).checked_add(v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u32::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u32::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u32::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(v2 as u32)?,
                TypedInteger::U16(v2) => v1.checked_add(v2 as u32)?,
                TypedInteger::U32(v2) => v1.checked_add(v2)?,
                TypedInteger::U64(v2) => {
                    u32::try_from((v1 as u64).checked_add(v2)?).ok()?
                }
                TypedInteger::U128(v2) => {
                    u32::try_from((v1 as u128).checked_add(v2)?).ok()?
                }
            }),
            TypedInteger::U64(v1) => TypedInteger::U64(match rhs {
                TypedInteger::I8(v2) => {
                    u64::try_from((v1 as i8).checked_add(v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u64::try_from((v1 as i16).checked_add(v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u64::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u64::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u64::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(v2 as u64)?,
                TypedInteger::U16(v2) => v1.checked_add(v2 as u64)?,
                TypedInteger::U32(v2) => v1.checked_add(v2 as u64)?,
                TypedInteger::U64(v2) => v1.checked_add(v2)?,
                TypedInteger::U128(v2) => {
                    u64::try_from((v1 as u128).checked_add(v2)?).ok()?
                }
            }),
            TypedInteger::U128(v1) => TypedInteger::U128(match rhs {
                TypedInteger::I8(v2) => {
                    u128::try_from((v1 as i8).checked_add(v2)?).ok()?
                }
                TypedInteger::I16(v2) => {
                    u128::try_from((v1 as i16).checked_add(v2)?).ok()?
                }
                TypedInteger::I32(v2) => {
                    u128::try_from((v1 as i32).checked_add(v2)?).ok()?
                }
                TypedInteger::I64(v2) => {
                    u128::try_from((v1 as i64).checked_add(v2)?).ok()?
                }
                TypedInteger::I128(v2) => {
                    u128::try_from((v1 as i128).checked_add(v2)?).ok()?
                }
                TypedInteger::U8(v2) => v1.checked_add(v2 as u128)?,
                TypedInteger::U16(v2) => v1.checked_add(v2 as u128)?,
                TypedInteger::U32(v2) => v1.checked_add(v2 as u128)?,
                TypedInteger::U64(v2) => v1.checked_add(v2 as u128)?,
                TypedInteger::U128(v2) => v1.checked_add(v2)?,
            }),
        })
    }
}

impl Add for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        TypedInteger::add(*self, *rhs)
    }
}

impl AddAssign for TypedInteger {
    // FIXME #126 add try_add_assign
    fn add_assign(&mut self, rhs: Self) {
        let res = (*self + rhs).unwrap();
        match res {
            TypedInteger::I8(v) => *self = TypedInteger::I8(v),
            TypedInteger::I16(v) => *self = TypedInteger::I16(v),
            TypedInteger::I32(v) => *self = TypedInteger::I32(v),
            TypedInteger::I64(v) => *self = TypedInteger::I64(v),
            TypedInteger::I128(v) => *self = TypedInteger::I128(v),
            TypedInteger::U8(v) => *self = TypedInteger::U8(v),
            TypedInteger::U16(v) => *self = TypedInteger::U16(v),
            TypedInteger::U32(v) => *self = TypedInteger::U32(v),
            TypedInteger::U64(v) => *self = TypedInteger::U64(v),
            TypedInteger::U128(v) => *self = TypedInteger::U128(v),
        }
    }
}

impl Sub for TypedInteger {
    type Output = Option<TypedInteger>;

    fn sub(self, rhs: Self) -> Self::Output {
        let neg_rhs = match rhs {
            TypedInteger::I8(v) => TypedInteger::I8(v.neg()),
            TypedInteger::I16(v) => TypedInteger::I16(v.neg()),
            TypedInteger::I32(v) => TypedInteger::I32(v.neg()),
            TypedInteger::I64(v) => TypedInteger::I64(v.neg()),
            TypedInteger::I128(v) => TypedInteger::I128(v.neg()),
            TypedInteger::U8(v) => TypedInteger::I16((v as i16).neg()),
            TypedInteger::U16(v) => TypedInteger::I32((v as i32).neg()),
            TypedInteger::U32(v) => TypedInteger::I64((v as i64).neg()),
            TypedInteger::U64(v) => TypedInteger::I128((v as i128).neg()),
            TypedInteger::U128(v) => {
                TypedInteger::I128((i128::try_from(v).ok()?).neg())
            }
        };
        self + neg_rhs
    }
}

impl Sub for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn sub(self, rhs: Self) -> Self::Output {
        TypedInteger::sub(*self, *rhs)
    }
}

impl From<i8> for TypedInteger {
    fn from(v: i8) -> Self {
        TypedInteger::I8(v)
    }
}
impl From<i16> for TypedInteger {
    fn from(v: i16) -> Self {
        TypedInteger::I16(v)
    }
}
impl From<i32> for TypedInteger {
    fn from(v: i32) -> Self {
        TypedInteger::I32(v)
    }
}
impl From<i64> for TypedInteger {
    fn from(v: i64) -> Self {
        TypedInteger::I64(v)
    }
}
impl From<i128> for TypedInteger {
    fn from(v: i128) -> Self {
        TypedInteger::I128(v)
    }
}
impl From<u8> for TypedInteger {
    fn from(v: u8) -> Self {
        TypedInteger::U8(v)
    }
}
impl From<u16> for TypedInteger {
    fn from(v: u16) -> Self {
        TypedInteger::U16(v)
    }
}
impl From<u32> for TypedInteger {
    fn from(v: u32) -> Self {
        TypedInteger::U32(v)
    }
}
impl From<u64> for TypedInteger {
    fn from(v: u64) -> Self {
        TypedInteger::U64(v)
    }
}
impl From<u128> for TypedInteger {
    fn from(v: u128) -> Self {
        TypedInteger::U128(v)
    }
}

// new into
impl<T: Into<ValueContainer>> TryFrom<Option<T>> for TypedInteger {
    type Error = ValueError;
    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        match value {
            Some(v) => {
                let integer: ValueContainer = v.into();
                integer
                    .to_value()
                    .borrow()
                    .cast_to_integer()
                    .ok_or(ValueError::TypeConversionError)
            }
            None => Err(ValueError::IsVoid),
        }
    }
}
