use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use crate::datex_values::soft_eq::SoftEq;

use super::super::core_value_trait::CoreValueTrait;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Integer {
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

impl Integer {
    fn subtype(&self) -> &'static str {
        match self {
            Integer::I8(_) => "/i8",
            Integer::I16(_) => "/i16",
            Integer::I32(_) => "/i32",
            Integer::I64(_) => "/i64",
            Integer::I128(_) => "/i128",
            Integer::U8(_) => "/u8",
            Integer::U16(_) => "/u16",
            Integer::U32(_) => "/u32",
            Integer::U64(_) => "/u64",
            Integer::U128(_) => "/u128",
        }
    }
    fn as_u128(&self) -> u128 {
        match self {
            Integer::I8(v) => *v as u128,
            Integer::I16(v) => *v as u128,
            Integer::I32(v) => *v as u128,
            Integer::I64(v) => *v as u128,
            Integer::I128(v) => *v as u128,
            Integer::U8(v) => *v as u128,
            Integer::U16(v) => *v as u128,
            Integer::U32(v) => *v as u128,
            Integer::U64(v) => *v as u128,
            Integer::U128(v) => *v,
        }
    }

    pub fn as_i128(&self) -> i128 {
        match self {
            Integer::I8(v) => *v as i128,
            Integer::I16(v) => *v as i128,
            Integer::I32(v) => *v as i128,
            Integer::I64(v) => *v as i128,
            Integer::I128(v) => *v,
            Integer::U8(v) => *v as i128,
            Integer::U16(v) => *v as i128,
            Integer::U32(v) => *v as i128,
            Integer::U64(v) => *v as i128,
            Integer::U128(v) => *v as i128, // This will panic if v > i128::MAX
        }
    }
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            Integer::I8(_)
                | Integer::I16(_)
                | Integer::I32(_)
                | Integer::I64(_)
                | Integer::I128(_)
        )
    }
    pub fn is_unsigned(&self) -> bool {
        !self.is_signed()
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Integer::I8(v) => write!(f, "{v}"),
            Integer::I16(v) => write!(f, "{v}"),
            Integer::I32(v) => write!(f, "{v}"),
            Integer::I64(v) => write!(f, "{v}"),
            Integer::I128(v) => write!(f, "{v}"),
            Integer::U8(v) => write!(f, "{v}"),
            Integer::U16(v) => write!(f, "{v}"),
            Integer::U32(v) => write!(f, "{v}"),
            Integer::U64(v) => write!(f, "{v}"),
            Integer::U128(v) => write!(f, "{v}"),
        }
    }
}

impl CoreValueTrait for Integer {}

impl SoftEq for Integer {
    fn soft_eq(&self, other: &Self) -> bool {
        if self.is_unsigned() && other.is_unsigned() {
            self.as_u128() == other.as_u128()
        } else {
            self.as_i128() == other.as_i128()
        }
    }
}

impl Add for Integer {
    type Output = Integer;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Integer::I8(v1) => match rhs {
                Integer::I8(v2) => Integer::I8(v1 + v2),
                Integer::I16(v2) => Integer::I8(v1 + v2 as i8),
                Integer::I32(v2) => Integer::I8(v1 + v2 as i8),
                Integer::I64(v2) => Integer::I8(v1 + v2 as i8),
                Integer::I128(v2) => Integer::I8(v1 + v2 as i8),
                Integer::U8(v2) => Integer::I8(v1 + v2 as i8),
                Integer::U16(v2) => Integer::I8(v1 + v2 as i8),
                Integer::U32(v2) => Integer::I8(v1 + v2 as i8),
                Integer::U64(v2) => Integer::I8(v1 + v2 as i8),
                Integer::U128(v2) => Integer::I8(v1 + v2 as i8),
            },
            Integer::I16(v1) => match rhs {
                Integer::I8(v2) => Integer::I16(v1 + v2 as i16),
                Integer::I16(v2) => Integer::I16(v1 + v2),
                Integer::I32(v2) => Integer::I16(v1 + v2 as i16),
                Integer::I64(v2) => Integer::I16(v1 + v2 as i16),
                Integer::I128(v2) => Integer::I16(v1 + v2 as i16),
                Integer::U8(v2) => Integer::I16(v1 + v2 as i16),
                Integer::U16(v2) => Integer::I16(v1 + v2 as i16),
                Integer::U32(v2) => Integer::I16(v1 + v2 as i16),
                Integer::U64(v2) => Integer::I16(v1 + v2 as i16),
                Integer::U128(v2) => Integer::I16(v1 + v2 as i16),
            },
            Integer::I32(v1) => match rhs {
                Integer::I8(v2) => Integer::I32(v1 + v2 as i32),
                Integer::I16(v2) => Integer::I32(v1 + v2 as i32),
                Integer::I32(v2) => Integer::I32(v1 + v2),
                Integer::I64(v2) => Integer::I32(v1 + v2 as i32),
                Integer::I128(v2) => Integer::I32(v1 + v2 as i32),
                Integer::U8(v2) => Integer::I32(v1 + v2 as i32),
                Integer::U16(v2) => Integer::I32(v1 + v2 as i32),
                Integer::U32(v2) => Integer::I32(v1 + v2 as i32),
                Integer::U64(v2) => Integer::I32(v1 + v2 as i32),
                Integer::U128(v2) => Integer::I32(v1 + v2 as i32),
            },
            Integer::I64(v1) => match rhs {
                Integer::I8(v2) => Integer::I64(v1 + v2 as i64),
                Integer::I16(v2) => Integer::I64(v1 + v2 as i64),
                Integer::I32(v2) => Integer::I64(v1 + v2 as i64),
                Integer::I64(v2) => Integer::I64(v1 + v2),
                Integer::I128(v2) => Integer::I64(v1 + v2 as i64),
                Integer::U8(v2) => Integer::I64(v1 + v2 as i64),
                Integer::U16(v2) => Integer::I64(v1 + v2 as i64),
                Integer::U32(v2) => Integer::I64(v1 + v2 as i64),
                Integer::U64(v2) => Integer::I64(v1 + v2 as i64),
                Integer::U128(v2) => Integer::I64(v1 + v2 as i64),
            },
            Integer::I128(v1) => match rhs {
                Integer::I8(v2) => Integer::I128(v1 + v2 as i128),
                Integer::I16(v2) => Integer::I128(v1 + v2 as i128),
                Integer::I32(v2) => Integer::I128(v1 + v2 as i128),
                Integer::I64(v2) => Integer::I128(v1 + v2 as i128),
                Integer::I128(v2) => Integer::I128(v1 + v2),
                Integer::U8(v2) => Integer::I128(v1 + v2 as i128),
                Integer::U16(v2) => Integer::I128(v1 + v2 as i128),
                Integer::U32(v2) => Integer::I128(v1 + v2 as i128),
                Integer::U64(v2) => Integer::I128(v1 + v2 as i128),
                Integer::U128(v2) => Integer::I128(v1 + v2 as i128),
            },
            Integer::U8(v1) => match rhs {
                Integer::I8(v2) => Integer::U8(v1 + v2 as u8),
                Integer::I16(v2) => Integer::U8(v1 + v2 as u8),
                Integer::I32(v2) => Integer::U8(v1 + v2 as u8),
                Integer::I64(v2) => Integer::U8(v1 + v2 as u8),
                Integer::I128(v2) => Integer::U8(v1 + v2 as u8),
                Integer::U8(v2) => Integer::U8(v1 + v2),
                Integer::U16(v2) => Integer::U8(v1 + v2 as u8),
                Integer::U32(v2) => Integer::U8(v1 + v2 as u8),
                Integer::U64(v2) => Integer::U8(v1 + v2 as u8),
                Integer::U128(v2) => Integer::U8(v1 + v2 as u8),
            },
            Integer::U16(v1) => match rhs {
                Integer::I8(v2) => Integer::U16(v1 + v2 as u16),
                Integer::I16(v2) => Integer::U16(v1 + v2 as u16),
                Integer::I32(v2) => Integer::U16(v1 + v2 as u16),
                Integer::I64(v2) => Integer::U16(v1 + v2 as u16),
                Integer::I128(v2) => Integer::U16(v1 + v2 as u16),
                Integer::U8(v2) => Integer::U16(v1 + v2 as u16),
                Integer::U16(v2) => Integer::U16(v1 + v2),
                Integer::U32(v2) => Integer::U16(v1 + v2 as u16),
                Integer::U64(v2) => Integer::U16(v1 + v2 as u16),
                Integer::U128(v2) => Integer::U16(v1 + v2 as u16),
            },
            Integer::U32(v1) => match rhs {
                Integer::I8(v2) => Integer::U32(v1 + v2 as u32),
                Integer::I16(v2) => Integer::U32(v1 + v2 as u32),
                Integer::I32(v2) => Integer::U32(v1 + v2 as u32),
                Integer::I64(v2) => Integer::U32(v1 + v2 as u32),
                Integer::I128(v2) => Integer::U32(v1 + v2 as u32),
                Integer::U8(v2) => Integer::U32(v1 + v2 as u32),
                Integer::U16(v2) => Integer::U32(v1 + v2 as u32),
                Integer::U32(v2) => Integer::U32(v1 + v2),
                Integer::U64(v2) => Integer::U32(v1 + v2 as u32),
                Integer::U128(v2) => Integer::U32(v1 + v2 as u32),
            },
            Integer::U64(v1) => match rhs {
                Integer::I8(v2) => Integer::U64(v1 + v2 as u64),
                Integer::I16(v2) => Integer::U64(v1 + v2 as u64),
                Integer::I32(v2) => Integer::U64(v1 + v2 as u64),
                Integer::I64(v2) => Integer::U64(v1 + v2 as u64),
                Integer::I128(v2) => Integer::U64(v1 + v2 as u64),
                Integer::U8(v2) => Integer::U64(v1 + v2 as u64),
                Integer::U16(v2) => Integer::U64(v1 + v2 as u64),
                Integer::U32(v2) => Integer::U64(v1 + v2 as u64),
                Integer::U64(v2) => Integer::U64(v1 + v2),
                Integer::U128(v2) => Integer::U64(v1 + v2 as u64),
            },
            Integer::U128(v1) => match rhs {
                Integer::I8(v2) => Integer::U128(v1 + v2 as u128),
                Integer::I16(v2) => Integer::U128(v1 + v2 as u128),
                Integer::I32(v2) => Integer::U128(v1 + v2 as u128),
                Integer::I64(v2) => Integer::U128(v1 + v2 as u128),
                Integer::I128(v2) => Integer::U128(v1 + v2 as u128),
                Integer::U8(v2) => Integer::U128(v1 + v2 as u128),
                Integer::U16(v2) => Integer::U128(v1 + v2 as u128),
                Integer::U32(v2) => Integer::U128(v1 + v2 as u128),
                Integer::U64(v2) => Integer::U128(v1 + v2 as u128),
                Integer::U128(v2) => Integer::U128(v1 + v2),
            },
        }
    }
}

impl Add for &Integer {
    type Output = Integer;

    fn add(self, rhs: Self) -> Self::Output {
        Integer::add(self.clone(), rhs.clone())
    }
}

impl AddAssign for Integer {
    fn add_assign(&mut self, rhs: Self) {
        let res = self.clone() + rhs;
        match res {
            Integer::I8(v) => *self = Integer::I8(v),
            Integer::I16(v) => *self = Integer::I16(v),
            Integer::I32(v) => *self = Integer::I32(v),
            Integer::I64(v) => *self = Integer::I64(v),
            Integer::I128(v) => *self = Integer::I128(v),
            Integer::U8(v) => *self = Integer::U8(v),
            Integer::U16(v) => *self = Integer::U16(v),
            Integer::U32(v) => *self = Integer::U32(v),
            Integer::U64(v) => *self = Integer::U64(v),
            Integer::U128(v) => *self = Integer::U128(v),
        }
    }
}

impl From<i8> for Integer {
    fn from(v: i8) -> Self {
        Integer::I8(v)
    }
}
impl From<i16> for Integer {
    fn from(v: i16) -> Self {
        Integer::I16(v)
    }
}
impl From<i32> for Integer {
    fn from(v: i32) -> Self {
        Integer::I32(v)
    }
}
impl From<i64> for Integer {
    fn from(v: i64) -> Self {
        Integer::I64(v)
    }
}
impl From<i128> for Integer {
    fn from(v: i128) -> Self {
        Integer::I128(v)
    }
}
impl From<u8> for Integer {
    fn from(v: u8) -> Self {
        Integer::U8(v)
    }
}
impl From<u16> for Integer {
    fn from(v: u16) -> Self {
        Integer::U16(v)
    }
}
impl From<u32> for Integer {
    fn from(v: u32) -> Self {
        Integer::U32(v)
    }
}
impl From<u64> for Integer {
    fn from(v: u64) -> Self {
        Integer::U64(v)
    }
}
impl From<u128> for Integer {
    fn from(v: u128) -> Self {
        Integer::U128(v)
    }
}
