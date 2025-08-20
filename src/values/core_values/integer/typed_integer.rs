use crate::values::{
    core_value_trait::CoreValueTrait,
    core_values::integer::{
        integer::Integer,
        utils::{smallest_fitting_signed, smallest_fitting_unsigned},
    },
    traits::structural_eq::StructuralEq,
    value_container::{ValueContainer, ValueError},
};
use core::panic;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    hash::Hash,
    ops::{Add, AddAssign, Neg, Sub},
};
use strum_macros::{AsRefStr, EnumIter, EnumString};

/// The integer type variants to be used as a inline
/// definition in DATEX (such as 42u32 or -42i64).
/// Note that changing the enum variants will change
/// the way integers are parsed in DATEX scripts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum IntegerTypeVariant {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Big,
}

#[derive(Debug, Clone, Eq)]
pub enum TypedInteger {
    Big(Integer),
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

impl Hash for TypedInteger {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypedInteger::Big(v) => v.hash(state),
            TypedInteger::I8(v) => v.hash(state),
            TypedInteger::I16(v) => v.hash(state),
            TypedInteger::I32(v) => v.hash(state),
            TypedInteger::I64(v) => v.hash(state),
            TypedInteger::I128(v) => v.hash(state),
            TypedInteger::U8(v) => v.hash(state),
            TypedInteger::U16(v) => v.hash(state),
            TypedInteger::U32(v) => v.hash(state),
            TypedInteger::U64(v) => v.hash(state),
            TypedInteger::U128(v) => v.hash(state),
        }
    }
}

impl TypedInteger {
    pub fn from_string_with_variant(
        s: &str,
        variant: IntegerTypeVariant,
    ) -> Option<TypedInteger> {
        Self::from_string_radix_with_variant(s, 10, variant)
    }
    pub fn from_string_radix_with_variant(
        s: &str,
        radix: u32,
        variant: IntegerTypeVariant,
    ) -> Option<TypedInteger> {
        let s = &s.replace('_', "");
        Some(match variant {
            IntegerTypeVariant::U8 => u8::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::U8(v)),
            IntegerTypeVariant::U16 => u16::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::U16(v)),
            IntegerTypeVariant::U32 => u32::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::U32(v)),
            IntegerTypeVariant::U64 => u64::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::U64(v)),
            IntegerTypeVariant::U128 => u128::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::U128(v)),
            IntegerTypeVariant::I8 => i8::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::I8(v)),
            IntegerTypeVariant::I16 => i16::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::I16(v)),
            IntegerTypeVariant::I32 => i32::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::I32(v)),
            IntegerTypeVariant::I64 => i64::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::I64(v)),
            IntegerTypeVariant::I128 => i128::from_str_radix(&s, radix)
                .ok()
                .map(|v| TypedInteger::I128(v)),
            IntegerTypeVariant::Big => Integer::from_string_radix(s, radix)
                .ok()
                .map(TypedInteger::Big),
        }?)
    }

    pub fn to_smallest_fitting(&self) -> TypedInteger {
        if self.is_unsigned()
            && let Some(u128) = self.as_u128()
        {
            smallest_fitting_unsigned(u128)
        } else if let Some(i128) = self.as_i128() {
            smallest_fitting_signed(i128)
        } else {
            self.clone()
        }
    }

    // fn subtype(&self) -> &'static str {
    //     match self {
    //         TypedInteger::I8(_) => "/i8",
    //         TypedInteger::I16(_) => "/i16",
    //         TypedInteger::I32(_) => "/i32",
    //         TypedInteger::I64(_) => "/i64",
    //         TypedInteger::I128(_) => "/i128",
    //         TypedInteger::U8(_) => "/u8",
    //         TypedInteger::U16(_) => "/u16",
    //         TypedInteger::U32(_) => "/u32",
    //         TypedInteger::U64(_) => "/u64",
    //         TypedInteger::U128(_) => "/u128",
    //     }
    // }
    pub fn as_integer(&self) -> Integer {
        match self {
            TypedInteger::Big(v) => v.clone(),
            TypedInteger::I8(v) => Integer::from(*v),
            TypedInteger::I16(v) => Integer::from(*v),
            TypedInteger::I32(v) => Integer::from(*v),
            TypedInteger::I64(v) => Integer::from(*v),
            TypedInteger::I128(v) => Integer::from(*v),
            TypedInteger::U8(v) => Integer::from(*v),
            TypedInteger::U16(v) => Integer::from(*v),
            TypedInteger::U32(v) => Integer::from(*v),
            TypedInteger::U64(v) => Integer::from(*v),
            TypedInteger::U128(v) => Integer::from(*v),
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

            TypedInteger::Big(v) => v.as_i8(),
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
            TypedInteger::Big(v) => v.as_i16(),
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
            TypedInteger::Big(v) => v.as_i32(),
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
            TypedInteger::Big(v) => v.as_i64(),
        }
    }

    // FIXME #125 we should probably allow casting to i128 from u128
    pub fn as_u128(&self) -> Option<u128> {
        match self {
            TypedInteger::U8(v) => Some(*v as u128),
            TypedInteger::U16(v) => Some(*v as u128),
            TypedInteger::U32(v) => Some(*v as u128),
            TypedInteger::U64(v) => Some(*v as u128),
            TypedInteger::U128(v) => Some(*v),
            TypedInteger::Big(v) => v.as_u128(),
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
            TypedInteger::Big(v) => v.as_i128(),
        }
    }
    pub fn is_signed(&self) -> bool {
        if let TypedInteger::Big(v) = self {
            return true;
        }
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

    pub fn is_positive(&self) -> bool {
        if let TypedInteger::Big(v) = self {
            return v.is_positive();
        }
        let v = self.as_i128().unwrap();
        v > 0
    }
    pub fn is_negative(&self) -> bool {
        if let TypedInteger::Big(v) = self {
            return v.is_negative();
        }
        let v = self.as_i128().unwrap();
        v < 0
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
            TypedInteger::Big(v) => write!(f, "{v}"),
        }
    }
}

impl CoreValueTrait for TypedInteger {}
// FIXME discuss on structural vs partial equality for integers
impl StructuralEq for TypedInteger {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypedInteger::I8(v1), TypedInteger::I8(v2)) => v1 == v2,
            (TypedInteger::I16(v1), TypedInteger::I16(v2)) => v1 == v2,
            (TypedInteger::I32(v1), TypedInteger::I32(v2)) => v1 == v2,
            (TypedInteger::I64(v1), TypedInteger::I64(v2)) => v1 == v2,
            (TypedInteger::I128(v1), TypedInteger::I128(v2)) => v1 == v2,
            (TypedInteger::U8(v1), TypedInteger::U8(v2)) => v1 == v2,
            (TypedInteger::U16(v1), TypedInteger::U16(v2)) => v1 == v2,
            (TypedInteger::U32(v1), TypedInteger::U32(v2)) => v1 == v2,
            (TypedInteger::U64(v1), TypedInteger::U64(v2)) => v1 == v2,
            (TypedInteger::U128(v1), TypedInteger::U128(v2)) => v1 == v2,
            (TypedInteger::Big(i1), TypedInteger::Big(i2)) => i1 == i2,
            (a, b) => a.as_integer() == b.as_integer(),
        }
    }
}

impl PartialEq for TypedInteger {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypedInteger::I8(v1), TypedInteger::I8(v2)) => v1 == v2,
            (TypedInteger::I16(v1), TypedInteger::I16(v2)) => v1 == v2,
            (TypedInteger::I32(v1), TypedInteger::I32(v2)) => v1 == v2,
            (TypedInteger::I64(v1), TypedInteger::I64(v2)) => v1 == v2,
            (TypedInteger::I128(v1), TypedInteger::I128(v2)) => v1 == v2,
            (TypedInteger::U8(v1), TypedInteger::U8(v2)) => v1 == v2,
            (TypedInteger::U16(v1), TypedInteger::U16(v2)) => v1 == v2,
            (TypedInteger::U32(v1), TypedInteger::U32(v2)) => v1 == v2,
            (TypedInteger::U64(v1), TypedInteger::U64(v2)) => v1 == v2,
            (TypedInteger::U128(v1), TypedInteger::U128(v2)) => v1 == v2,
            (TypedInteger::Big(i1), TypedInteger::Big(i2)) => i1 == i2,
            _ => false,
        }
    }
}

impl Add for TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        Some(match self {
            TypedInteger::Big(v1) => TypedInteger::Big(v1 + Integer::from(rhs)),
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
                TypedInteger::Big(v2) => {
                    i8::try_from((v1).checked_add(v2.as_i8()?)?).ok()?
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
                TypedInteger::Big(v2) => {
                    i16::try_from(v1.checked_add(v2.as_i16()?)?).ok()?
                }
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
                TypedInteger::Big(v2) => {
                    i32::try_from(v1.checked_add(v2.as_i32()?)?).ok()?
                }
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
                TypedInteger::Big(v2) => {
                    i64::try_from(v1.checked_add(v2.as_i64()?)?).ok()?
                }
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
                TypedInteger::Big(v2) => v1.checked_add(v2.as_i128()?)?,
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
                TypedInteger::Big(v2) => {
                    u8::try_from((v1 as u16).checked_add(v2.as_u16()?)?).ok()?
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
                TypedInteger::Big(v2) => {
                    u16::try_from((v1 as u32).checked_add(v2.as_u32()?)?)
                        .ok()?
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
                TypedInteger::Big(v2) => {
                    u32::try_from((v1 as u64).checked_add(v2.as_u64()?)?)
                        .ok()?
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
                TypedInteger::Big(v2) => {
                    u64::try_from((v1 as u128).checked_add(v2.as_u128()?)?)
                        .ok()?
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
                TypedInteger::Big(v2) => {
                    u128::try_from((v1 as i128).checked_add(v2.as_i128()?)?)
                        .ok()?
                }
            }),
        })
    }
}

impl Add for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        TypedInteger::add(self.clone(), rhs.clone())
    }
}

impl AddAssign for TypedInteger {
    // FIXME error handling / wrapping if out of bounds
    fn add_assign(&mut self, rhs: Self) {
        *self = TypedInteger::add(self.clone(), rhs).expect("Failed to add");
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
            TypedInteger::Big(v) => TypedInteger::Big(v.neg()),
        };
        self + neg_rhs
    }
}

impl Sub for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn sub(self, rhs: Self) -> Self::Output {
        TypedInteger::sub(self.clone(), rhs.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_integer_addition() {
        let a = TypedInteger::I8(10);
        let b = TypedInteger::I8(20);

        let result = a.clone() + b;
        assert_eq!(result, Some(TypedInteger::I8(30)));

        let c = TypedInteger::U8(10);
        let result = a.clone() + c.clone();
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
        let a = TypedInteger::from(10_i8);
        let b = TypedInteger::from(20_i8);
        let result = a + b;
        assert_eq!(result, Some(TypedInteger::I8(30_i8)));
    }

    #[test]
    fn test_integer() {
        let a = TypedInteger::from(1_i8);
        assert_eq!(a, TypedInteger::I8(1));

        let b = TypedInteger::from(1_u8);
        assert_eq!(b, TypedInteger::U8(1));

        let c = TypedInteger::from(1_i16);
        assert_eq!(c, TypedInteger::I16(1));

        let d = TypedInteger::from(1_u16);
        assert_eq!(d, TypedInteger::U16(1));

        let e = TypedInteger::from(1_i32);
        assert_eq!(e, TypedInteger::I32(1));

        let f = TypedInteger::from(1_u32);
        assert_eq!(f, TypedInteger::U32(1));

        let g = TypedInteger::from(1_i64);
        assert_eq!(g, TypedInteger::I64(1));

        let h = TypedInteger::from(1_u64);
        assert_eq!(h, TypedInteger::U64(1));

        let i = TypedInteger::from(1_i128);
        assert_eq!(i, TypedInteger::I128(1));

        let j = TypedInteger::from(1_u128);
        assert_eq!(j, TypedInteger::U128(1));

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
