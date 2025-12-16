use crate::values::{
    core_values::{
        error::NumberParseError,
        integer::{
            Integer,
            utils::{smallest_fitting_signed, smallest_fitting_unsigned},
        },
    },
    value_container::{ValueContainer, ValueError},
};

use crate::libs::core::CoreLibPointerId;
use crate::stdlib::format;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::traits::structural_eq::StructuralEq;
use core::hash::Hash;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::unreachable;
use core::{
    fmt::Display,
    ops::{Add, AddAssign, Neg, Sub},
};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use strum::Display;
use strum_macros::{AsRefStr, EnumIter, EnumString};

/// The integer type variants to be used as a inline
/// definition in DATEX (such as 42u32 or -42i64).
/// Note that changing the enum variants will change
/// the way integers are parsed in DATEX scripts.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    EnumIter,
    AsRefStr,
    IntoPrimitive,
    TryFromPrimitive,
    Serialize,
    Deserialize,
    Display,
)]
#[repr(u8)]
#[strum(serialize_all = "lowercase")]
pub enum IntegerTypeVariant {
    U8 = 1, // rationale: We need to start with 1 here, as the core lib pointer id for the base type is using OFFSET_X + variant as index
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    IBig,
    // TODO: ubig?
}

#[derive(Debug, Clone, Eq)]
pub enum TypedInteger {
    IBig(Integer),
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

impl Serialize for TypedInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TypedInteger::IBig(v) => serializer.serialize_str(&v.to_string()),
            TypedInteger::I8(v) => serializer.serialize_i8(*v),
            TypedInteger::I16(v) => serializer.serialize_i16(*v),
            TypedInteger::I32(v) => serializer.serialize_i32(*v),
            TypedInteger::I64(v) => serializer.serialize_i64(*v),
            TypedInteger::I128(v) => serializer.serialize_i128(*v),
            TypedInteger::U8(v) => serializer.serialize_u8(*v),
            TypedInteger::U16(v) => serializer.serialize_u16(*v),
            TypedInteger::U32(v) => serializer.serialize_u32(*v),
            TypedInteger::U64(v) => serializer.serialize_u64(*v),
            TypedInteger::U128(v) => serializer.serialize_u128(*v),
        }
    }
}

impl From<&TypedInteger> for CoreLibPointerId {
    fn from(value: &TypedInteger) -> Self {
        CoreLibPointerId::Integer(Some(value.variant()))
    }
}

impl<'de> Deserialize<'de> for TypedInteger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Try to parse as Integer (big)
        if let Ok(big_integer) = Integer::from_string(&s) {
            return Ok(TypedInteger::IBig(big_integer));
        }

        // Try to parse as i128
        if let Ok(i128_value) = s.parse::<i128>() {
            return Ok(smallest_fitting_signed(i128_value));
        }

        // Try to parse as u128
        if let Ok(u128_value) = s.parse::<u128>() {
            return Ok(smallest_fitting_unsigned(u128_value));
        }

        Err(serde::de::Error::custom(format!(
            "Failed to parse '{}' as TypedInteger",
            s
        )))
    }
}

impl Hash for TypedInteger {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypedInteger::IBig(v) => v.hash(state),
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
    // TODO #342: add from_integer_with_variant

    /// Parses a string into a TypedInteger with the given variant.
    /// If the string is not a valid integer, returns an error.
    pub fn from_string_with_variant(
        s: &str,
        variant: IntegerTypeVariant,
    ) -> Result<TypedInteger, NumberParseError> {
        Self::from_string_radix_with_variant(s, 10, variant)
    }

    /// Parses a string into a TypedInteger with the given variant and radix.
    /// If the string is not a valid integer, returns an error.
    pub fn from_string_radix_with_variant(
        s: &str,
        radix: u32,
        variant: IntegerTypeVariant,
    ) -> Result<TypedInteger, NumberParseError> {
        if core::matches!(variant, IntegerTypeVariant::IBig) {
            return Ok(TypedInteger::IBig(Integer::from_string_radix(
                s, radix,
            )?));
        }

        match variant {
            IntegerTypeVariant::U8 => {
                u8::from_str_radix(s, radix).map(TypedInteger::U8)
            }
            IntegerTypeVariant::U16 => {
                u16::from_str_radix(s, radix).map(TypedInteger::U16)
            }
            IntegerTypeVariant::U32 => {
                u32::from_str_radix(s, radix).map(TypedInteger::U32)
            }
            IntegerTypeVariant::U64 => {
                u64::from_str_radix(s, radix).map(TypedInteger::U64)
            }
            IntegerTypeVariant::U128 => {
                u128::from_str_radix(s, radix).map(TypedInteger::U128)
            }
            IntegerTypeVariant::I8 => {
                i8::from_str_radix(s, radix).map(TypedInteger::I8)
            }
            IntegerTypeVariant::I16 => {
                i16::from_str_radix(s, radix).map(TypedInteger::I16)
            }
            IntegerTypeVariant::I32 => {
                i32::from_str_radix(s, radix).map(TypedInteger::I32)
            }
            IntegerTypeVariant::I64 => {
                i64::from_str_radix(s, radix).map(TypedInteger::I64)
            }
            IntegerTypeVariant::I128 => {
                i128::from_str_radix(s, radix).map(TypedInteger::I128)
            }
            _ => unreachable!(""),
        }
        .map_err(|e| match e.kind() {
            core::num::IntErrorKind::Zero
            | core::num::IntErrorKind::Empty
            | core::num::IntErrorKind::InvalidDigit => {
                NumberParseError::InvalidFormat
            }
            core::num::IntErrorKind::PosOverflow
            | core::num::IntErrorKind::NegOverflow => {
                NumberParseError::OutOfRange
            }
            _ => core::panic!("Unhandled integer parse error: {:?}", e.kind()),
        })
    }

    /// Converts the integer to the smallest fitting TypedInteger variant.
    pub fn to_smallest_fitting(&self) -> TypedInteger {
        if self.is_unsigned_variant()
            && let Some(u128) = self.as_u128()
        {
            smallest_fitting_unsigned(u128)
        } else if let Some(i128) = self.as_i128() {
            smallest_fitting_signed(i128)
        } else {
            self.clone()
        }
    }

    /// Converts the TypedInteger to an Integer (big).
    pub fn as_integer(&self) -> Integer {
        match self {
            TypedInteger::IBig(v) => v.clone(),
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

    /// Converts the integer to an i8 if it fits, otherwise returns None.
    pub fn as_i8(&self) -> Option<i8> {
        match self {
            TypedInteger::I8(v) => Some(*v),
            TypedInteger::I16(v) => i8::try_from(*v).ok(),
            TypedInteger::I32(v) => i8::try_from(*v).ok(),
            TypedInteger::I64(v) => i8::try_from(*v).ok(),
            TypedInteger::I128(v) => i8::try_from(*v).ok(),

            TypedInteger::U8(v) => i8::try_from(*v).ok(),
            TypedInteger::U16(v) => i8::try_from(*v).ok(),
            TypedInteger::U32(v) => i8::try_from(*v).ok(),
            TypedInteger::U64(v) => i8::try_from(*v).ok(),
            TypedInteger::U128(v) => i8::try_from(*v).ok(),

            TypedInteger::IBig(v) => v.as_i8(),
        }
    }

    /// Converts the integer to an i16 if it fits, otherwise returns None.
    pub fn as_i16(&self) -> Option<i16> {
        match self {
            TypedInteger::I8(v) => Some(i16::from(*v)),
            TypedInteger::I16(v) => Some(*v),
            TypedInteger::I32(v) => i16::try_from(*v).ok(),
            TypedInteger::I64(v) => i16::try_from(*v).ok(),
            TypedInteger::I128(v) => i16::try_from(*v).ok(),

            TypedInteger::U8(v) => Some(i16::from(*v)),
            TypedInteger::U16(v) => i16::try_from(*v).ok(),
            TypedInteger::U32(v) => i16::try_from(*v).ok(),
            TypedInteger::U64(v) => i16::try_from(*v).ok(),
            TypedInteger::U128(v) => i16::try_from(*v).ok(),
            TypedInteger::IBig(v) => v.as_i16(),
        }
    }

    /// Converts the integer to an i32 if it fits, otherwise returns None.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            TypedInteger::I8(v) => Some(i32::from(*v)),
            TypedInteger::I16(v) => Some(i32::from(*v)),
            TypedInteger::I32(v) => Some(*v),
            TypedInteger::I64(v) => i32::try_from(*v).ok(),
            TypedInteger::I128(v) => i32::try_from(*v).ok(),

            TypedInteger::U8(v) => Some(i32::from(*v)),
            TypedInteger::U16(v) => Some(i32::from(*v)),
            TypedInteger::U32(v) => i32::try_from(*v).ok(),
            TypedInteger::U64(v) => i32::try_from(*v).ok(),
            TypedInteger::U128(v) => i32::try_from(*v).ok(),
            TypedInteger::IBig(v) => v.as_i32(),
        }
    }

    /// Converts the integer to a i64 if it fits, otherwise returns None.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TypedInteger::I8(v) => Some(i64::from(*v)),
            TypedInteger::I16(v) => Some(i64::from(*v)),
            TypedInteger::I32(v) => Some(i64::from(*v)),
            TypedInteger::I64(v) => Some(*v),
            TypedInteger::I128(v) => i64::try_from(*v).ok(),

            TypedInteger::U8(v) => Some(i64::from(*v)),
            TypedInteger::U16(v) => Some(i64::from(*v)),
            TypedInteger::U32(v) => Some(i64::from(*v)),
            TypedInteger::U64(v) => i64::try_from(*v).ok(),
            TypedInteger::U128(v) => i64::try_from(*v).ok(),
            TypedInteger::IBig(v) => v.as_i64(),
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
            TypedInteger::IBig(v) => v.as_u128(),
            _ => unreachable!("as_u128 called on a signed integer"),
        }
    }

    /// Converts the integer to an i128 if it fits, otherwise returns None.
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
            TypedInteger::IBig(v) => v.as_i128(),
        }
    }

    /// Converts the integer to a usize if it fits, otherwise returns None.
    pub fn as_usize(&self) -> Option<usize> {
        self.as_u128().and_then(|v| usize::try_from(v).ok())
    }

    /// Returns true if the integer is of a signed type.
    pub fn is_signed_variant(&self) -> bool {
        if let TypedInteger::IBig(_) = self {
            return true;
        }
        core::matches!(
            self,
            TypedInteger::I8(_)
                | TypedInteger::I16(_)
                | TypedInteger::I32(_)
                | TypedInteger::I64(_)
                | TypedInteger::I128(_)
        )
    }

    pub fn is_zero(&self) -> bool {
        match self {
            TypedInteger::IBig(v) => v.is_zero(),
            TypedInteger::I8(v) => *v == 0,
            TypedInteger::I16(v) => *v == 0,
            TypedInteger::I32(v) => *v == 0,
            TypedInteger::I64(v) => *v == 0,
            TypedInteger::I128(v) => *v == 0,
            TypedInteger::U8(v) => *v == 0,
            TypedInteger::U16(v) => *v == 0,
            TypedInteger::U32(v) => *v == 0,
            TypedInteger::U64(v) => *v == 0,
            TypedInteger::U128(v) => *v == 0,
        }
    }

    /// Returns true if the integer is of an unsigned type.
    pub fn is_unsigned_variant(&self) -> bool {
        !self.is_signed_variant()
    }

    /// Returns true if the integer is positive.
    /// Zero is not considered positive.
    pub fn is_positive(&self) -> bool {
        if let TypedInteger::IBig(v) = self {
            return v.is_positive();
        }
        let v = self.as_i128().unwrap();
        v > 0
    }

    /// Returns true if the integer is negative.
    /// Zero is not considered negative.
    pub fn is_negative(&self) -> bool {
        if let TypedInteger::IBig(v) = self {
            return v.is_negative();
        }
        let v = self.as_i128().unwrap();
        v < 0
    }

    pub fn variant(&self) -> IntegerTypeVariant {
        match self {
            TypedInteger::IBig(_) => IntegerTypeVariant::IBig,
            TypedInteger::I8(_) => IntegerTypeVariant::I8,
            TypedInteger::I16(_) => IntegerTypeVariant::I16,
            TypedInteger::I32(_) => IntegerTypeVariant::I32,
            TypedInteger::I64(_) => IntegerTypeVariant::I64,
            TypedInteger::I128(_) => IntegerTypeVariant::I128,
            TypedInteger::U8(_) => IntegerTypeVariant::U8,
            TypedInteger::U16(_) => IntegerTypeVariant::U16,
            TypedInteger::U32(_) => IntegerTypeVariant::U32,
            TypedInteger::U64(_) => IntegerTypeVariant::U64,
            TypedInteger::U128(_) => IntegerTypeVariant::U128,
        }
    }

    pub fn to_string_with_suffix(&self) -> String {
        match self {
            TypedInteger::I8(v) => format!("{v}i8"),
            TypedInteger::I16(v) => format!("{v}i16"),
            TypedInteger::I32(v) => format!("{v}i32"),
            TypedInteger::I64(v) => format!("{v}i64"),
            TypedInteger::I128(v) => format!("{v}i128"),
            TypedInteger::U8(v) => format!("{v}u8"),
            TypedInteger::U16(v) => format!("{v}u16"),
            TypedInteger::U32(v) => format!("{v}u32"),
            TypedInteger::U64(v) => format!("{v}u64"),
            TypedInteger::U128(v) => format!("{v}u128"),
            TypedInteger::IBig(v) => format!("{v}ibig"),
        }
    }
}

impl Display for TypedInteger {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            TypedInteger::I8(v) => core::write!(f, "{v}"),
            TypedInteger::I16(v) => core::write!(f, "{v}"),
            TypedInteger::I32(v) => core::write!(f, "{v}"),
            TypedInteger::I64(v) => core::write!(f, "{v}"),
            TypedInteger::I128(v) => core::write!(f, "{v}"),
            TypedInteger::U8(v) => core::write!(f, "{v}"),
            TypedInteger::U16(v) => core::write!(f, "{v}"),
            TypedInteger::U32(v) => core::write!(f, "{v}"),
            TypedInteger::U64(v) => core::write!(f, "{v}"),
            TypedInteger::U128(v) => core::write!(f, "{v}"),
            TypedInteger::IBig(v) => core::write!(f, "{v}"),
        }
    }
}

// FIXME #343 discuss on structural vs partial equality for integers
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
            (TypedInteger::IBig(i1), TypedInteger::IBig(i2)) => i1 == i2,
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
            (TypedInteger::IBig(i1), TypedInteger::IBig(i2)) => i1 == i2,
            _ => false,
        }
    }
}

impl Add for TypedInteger {
    type Output = Option<TypedInteger>;

    fn add(self, rhs: Self) -> Self::Output {
        Some(match self {
            TypedInteger::IBig(v1) => {
                TypedInteger::IBig(v1 + Integer::from(rhs))
            }
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
                TypedInteger::IBig(v2) => (v1).checked_add(v2.as_i8()?)?,
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
                TypedInteger::U8(v2) => v1.checked_add(v2 as i16)?,
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
                TypedInteger::IBig(v2) => v1.checked_add(v2.as_i16()?)?,
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
                TypedInteger::IBig(v2) => v1.checked_add(v2.as_i32()?)?,
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
                    i64::from((v1 as i16).checked_add(v2 as i16)?)
                }
                TypedInteger::U16(v2) => {
                    i64::from((v1 as i32).checked_add(v2 as i32)?)
                }
                TypedInteger::U32(v2) => v1.checked_add(v2 as i64)?,
                TypedInteger::U64(v2) => {
                    i64::try_from((v1 as i128).checked_add(v2 as i128)?).ok()?
                }
                TypedInteger::U128(v2) => i64::try_from(
                    (v1 as i128).checked_add(v2.try_into().ok()?)?,
                )
                .ok()?,
                TypedInteger::IBig(v2) => v1.checked_add(v2.as_i64()?)?,
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
                TypedInteger::IBig(v2) => v1.checked_add(v2.as_i128()?)?,
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
                TypedInteger::IBig(v2) => {
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
                TypedInteger::IBig(v2) => {
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
                TypedInteger::IBig(v2) => {
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
                TypedInteger::IBig(v2) => {
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
                TypedInteger::IBig(v2) => {
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
        // FIXME #344 optimize to avoid cloning
        TypedInteger::add(self.clone(), rhs.clone())
    }
}

impl AddAssign for TypedInteger {
    // FIXME #345 error handling / wrapping if out of bounds
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
            TypedInteger::IBig(v) => TypedInteger::IBig(v.neg()),
        };
        self.add(neg_rhs)
    }
}

impl Sub for &TypedInteger {
    type Output = Option<TypedInteger>;

    fn sub(self, rhs: Self) -> Self::Output {
        // Fixme #346 optimize to avoid cloning
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
                    ._cast_to_integer_internal()
                    .ok_or(ValueError::TypeConversionError)
            }
            None => Err(ValueError::IsVoid),
        }
    }
}

// FIXME #347 shall we allow negation of unsigned integers and wrap around?
impl Neg for TypedInteger {
    type Output = Result<TypedInteger, ValueError>;

    fn neg(self) -> Self::Output {
        match self {
            TypedInteger::I8(v) => Ok(TypedInteger::I8(v.neg())),
            TypedInteger::I16(v) => Ok(TypedInteger::I16(v.neg())),
            TypedInteger::I32(v) => Ok(TypedInteger::I32(v.neg())),
            TypedInteger::I64(v) => Ok(TypedInteger::I64(v.neg())),
            TypedInteger::I128(v) => Ok(TypedInteger::I128(v.neg())),
            TypedInteger::IBig(v) => Ok(TypedInteger::IBig(v.neg())),
            _ => Err(ValueError::InvalidOperation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_integer_addition() {
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
    fn typed_integer_subtraction() {
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
    fn integer_addition() {
        let a = TypedInteger::from(10_i8);
        let b = TypedInteger::from(20_i8);
        let result = a + b;
        assert_eq!(result, Some(TypedInteger::I8(30_i8)));
    }

    #[test]
    fn integer() {
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
