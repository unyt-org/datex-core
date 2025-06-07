use std::{
    fmt::Display,
    hash::Hash,
    ops::{Add, AddAssign, Neg, Sub},
};

use crate::datex_values::soft_eq::SoftEq;
use super::super::core_value_trait::CoreValueTrait;


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


#[derive(Debug, Clone, Eq, Copy)]
pub struct Integer(pub TypedInteger);
impl Integer {
    pub fn to_smallest_fitting(&self) -> TypedInteger {
        self.0.to_smallest_fitting()
    }
}

impl SoftEq for Integer {
    fn soft_eq(&self, other: &Self) -> bool {
        self.0.soft_eq(&other.0)
    }
}

impl<T: Into<TypedInteger>> From<T> for Integer {
    fn from(value: T) -> Self {
        Integer(value.into())
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// FIXME use integer i32 by default and switch automaticially if a calculation provoces one of the values to get out of bounds
impl Add for Integer {
    type Output = Option<Integer>;

    fn add(self, rhs: Self) -> Self::Output {
        let a = self.0;
        let b = rhs.0;
        if a.is_unsigned() && b.is_unsigned() {
            Some(Integer(smallest_fitting_unsigned(
                a.as_u128().checked_add(b.as_u128())?,
            )))
        } else {
            Some(Integer(smallest_fitting_signed(
                a.as_i128()?.checked_add(b.as_i128()?)?,
            )))
        }
    }
}
impl Add for &Integer {
    type Output = Option<Integer>;

    fn add(self, rhs: Self) -> Self::Output {
        Integer::add(*self, *rhs)
    }
}

impl Sub for Integer {
    type Output = Option<Integer>;

    fn sub(self, rhs: Self) -> Self::Output {
        let a = self.0;
        let b = rhs.0;
        Some(Integer(smallest_fitting_signed(
            a.as_i128()?.checked_sub(b.as_i128()?)?,
        )))
    }
}

impl PartialEq for Integer {
    fn eq(&self, other: &Self) -> bool {
        self.soft_eq(other)
    }
}

impl Hash for Integer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.0.is_signed() {
            self.0.as_i128().hash(state);
        } else {
            self.0.as_u128().hash(state);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
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
    fn as_u128(&self) -> u128 {
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

impl SoftEq for TypedInteger {
    fn soft_eq(&self, other: &Self) -> bool {
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
    // FIXME add try_add_assign
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_integer_addition() {
        let a = TypedInteger::I8(10);
        let b = TypedInteger::I8(20);

        let result = a + b;
        assert_eq!(result, Some(TypedInteger::I8(30)));

        let c = TypedInteger::U8(10);
        let result = a + c;
        assert_eq!(result, Some(TypedInteger::I8(20)));

        let result = c + a;
        assert_eq!(result, Some(TypedInteger::U8(20)));

        // out of bounds
        let d = TypedInteger::I8(100);
        let e = TypedInteger::I8(50);
        let result = d + e;
        assert_eq!(result, None);
    }

    #[test]
    fn test_typed_integer_subtraction() {
        let a = TypedInteger::I8(30);
        let b = TypedInteger::I8(20);
        let result = a - b;
        assert_eq!(result, Some(TypedInteger::I8(10)));

        // negative result
        let c = TypedInteger::I8(20);
        let d = TypedInteger::I8(30);
        let result = c - d;
        assert_eq!(result, Some(TypedInteger::I8(-10)));

        // out of bounds
        let e = TypedInteger::I8(-100);
        let f = TypedInteger::I8(50);
        let result = e - f;
        assert_eq!(result, None);

        let g = TypedInteger::U8(30);
        let h = TypedInteger::I8(30);
        let result = g - h;
        assert_eq!(result, Some(TypedInteger::U8(0)));

        let h = TypedInteger::U8(30);
        let i = TypedInteger::I8(31);

        let result = h - i;
        assert_eq!(result, None);
    }

    #[test]
    fn test_integer_addition() {
        let a = Integer::from(10_i8);
        let b = Integer::from(20_i8);
        let result = a + b;
        assert_eq!(result, Some(Integer(TypedInteger::I8(30))));

        // let c = Integer::from(10 as u8);
        // let result = a + c;
        // assert_eq!(result, Some(Integer(TypedInteger::I8(20))));

        // let result = c + a;
        // assert_eq!(result, Some(Integer(TypedInteger::U8(20))));

        // // out of bounds
        // let d = Integer::from(100 as i8);
        // let e = Integer::from(50 as i8);
        // let result = d + e;
        // assert_eq!(result, None);
    }

    #[test]
    fn test_integer() {
        let a = Integer::from(1_i8);
        assert_eq!(a.0, TypedInteger::I8(1));

        let b = Integer::from(1_u8);
        assert_eq!(b.0, TypedInteger::U8(1));

        let c = Integer::from(1_i16);
        assert_eq!(c.0, TypedInteger::I16(1));

        let d = Integer::from(1_u16);
        assert_eq!(d.0, TypedInteger::U16(1));

        let e = Integer::from(1_i32);
        assert_eq!(e.0, TypedInteger::I32(1));

        let f = Integer::from(1_u32);
        assert_eq!(f.0, TypedInteger::U32(1));

        let g = Integer::from(1_i64);
        assert_eq!(g.0, TypedInteger::I64(1));

        let h = Integer::from(1_u64);
        assert_eq!(h.0, TypedInteger::U64(1));

        let i = Integer::from(1_i128);
        assert_eq!(i.0, TypedInteger::I128(1));

        let j = Integer::from(1_u128);
        assert_eq!(j.0, TypedInteger::U128(1));

        assert_eq!(a.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(b.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(c.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(d.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(e.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(f.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(g.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(h.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(i.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(j.to_smallest_fitting(), TypedInteger::U8(1));
    }
}
