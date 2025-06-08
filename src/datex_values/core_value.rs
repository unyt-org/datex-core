use datex_macros::FromCoreValue;

use crate::datex_values::core_values::array::Array;
use crate::datex_values::core_values::boolean::Boolean;
use crate::datex_values::core_values::decimal::{Decimal, TypedDecimal};
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::core_values::integer::{Integer, TypedInteger};
use crate::datex_values::core_values::object::Object;
use crate::datex_values::core_values::text::Text;
use crate::datex_values::core_values::tuple::Tuple;
use crate::datex_values::datex_type::CoreValueType;
use crate::datex_values::traits::soft_eq::SoftEq;
use crate::datex_values::value_container::{ValueContainer, ValueError};
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not, Sub};

#[derive(Clone, Debug, PartialEq, Eq, Hash, FromCoreValue)]
pub enum CoreValue {
    Null,
    Bool(Boolean),
    Integer(Integer),
    TypedInteger(TypedInteger),
    Decimal(Decimal),
    TypedDecimal(TypedDecimal),
    Text(Text),
    Endpoint(Endpoint),
    Array(Array),
    Object(Object),
    Tuple(Tuple),
}
impl SoftEq for CoreValue {
    fn soft_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CoreValue::Bool(a), CoreValue::Bool(b)) => a.soft_eq(b),

            // Integers + TypedIntegers
            (
                CoreValue::Integer(Integer(a)) | CoreValue::TypedInteger(a),
                CoreValue::Integer(Integer(b)) | CoreValue::TypedInteger(b),
            ) => a.soft_eq(b),

            // Decimals + TypedDecimals
            (
                CoreValue::Decimal(Decimal(a)) | CoreValue::TypedDecimal(a),
                CoreValue::Decimal(Decimal(b)) | CoreValue::TypedDecimal(b),
            ) => a.soft_eq(b),

            // Mixed Integer and Decimal comparisons
            (
                CoreValue::Decimal(Decimal(a)) | CoreValue::TypedDecimal(a),
                CoreValue::Integer(Integer(b)) | CoreValue::TypedInteger(b),
            )
            | (
                CoreValue::Integer(Integer(b)) | CoreValue::TypedInteger(b),
                CoreValue::Decimal(Decimal(a)) | CoreValue::TypedDecimal(a),
            ) => match a.as_integer() {
                Some(int) => b.soft_eq(&TypedInteger::from(int)),
                None => false,
            },

            (CoreValue::Text(a), CoreValue::Text(b)) => a.soft_eq(b),
            (CoreValue::Null, CoreValue::Null) => true,
            (CoreValue::Endpoint(a), CoreValue::Endpoint(b)) => a.soft_eq(b),
            (CoreValue::Array(a), CoreValue::Array(b)) => a.soft_eq(b),
            (CoreValue::Object(a), CoreValue::Object(b)) => a.soft_eq(b),
            (CoreValue::Tuple(a), CoreValue::Tuple(b)) => a.soft_eq(b),
            _ => false,
        }
    }
}

impl From<&str> for CoreValue {
    fn from(value: &str) -> Self {
        CoreValue::Text(value.into())
    }
}
impl From<String> for CoreValue {
    fn from(value: String) -> Self {
        CoreValue::Text(Text(value))
    }
}

impl<T> From<Vec<T>> for CoreValue
where
    T: Into<ValueContainer>,
{
    fn from(vec: Vec<T>) -> Self {
        CoreValue::Array(vec.into())
    }
}

impl<T> FromIterator<T> for CoreValue
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        CoreValue::Array(Array(iter.into_iter().map(Into::into).collect()))
    }
}

impl From<bool> for CoreValue {
    fn from(value: bool) -> Self {
        CoreValue::Bool(value.into())
    }
}

impl From<i8> for CoreValue {
    fn from(value: i8) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<i16> for CoreValue {
    fn from(value: i16) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<i32> for CoreValue {
    fn from(value: i32) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<i64> for CoreValue {
    fn from(value: i64) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<i128> for CoreValue {
    fn from(value: i128) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}

impl From<u8> for CoreValue {
    fn from(value: u8) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<u16> for CoreValue {
    fn from(value: u16) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<u32> for CoreValue {
    fn from(value: u32) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<u64> for CoreValue {
    fn from(value: u64) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}
impl From<u128> for CoreValue {
    fn from(value: u128) -> Self {
        CoreValue::TypedInteger(value.into())
    }
}

impl From<f32> for CoreValue {
    fn from(value: f32) -> Self {
        CoreValue::TypedDecimal(value.into())
    }
}
impl From<f64> for CoreValue {
    fn from(value: f64) -> Self {
        CoreValue::TypedDecimal(value.into())
    }
}

impl CoreValue {
    pub fn new<T>(value: T) -> CoreValue
    where
        CoreValue: From<T>,
    {
        value.into()
    }

    pub fn get_default_type(&self) -> CoreValueType {
        match self {
            CoreValue::Bool(_) => CoreValueType::Bool,
            CoreValue::TypedInteger(int) => match int {
                TypedInteger::I8(_) => CoreValueType::I8,
                TypedInteger::I16(_) => CoreValueType::I16,
                TypedInteger::I32(_) => CoreValueType::I32,
                TypedInteger::I64(_) => CoreValueType::I64,
                TypedInteger::I128(_) => CoreValueType::I128,

                TypedInteger::U8(_) => CoreValueType::U8,
                TypedInteger::U16(_) => CoreValueType::U16,
                TypedInteger::U32(_) => CoreValueType::U32,
                TypedInteger::U64(_) => CoreValueType::U64,
                TypedInteger::U128(_) => CoreValueType::U128,
            },
            CoreValue::TypedDecimal(decimal) => match decimal {
                TypedDecimal::F32(_) => CoreValueType::F32,
                TypedDecimal::F64(_) => CoreValueType::F64,
                TypedDecimal::Big(_) => CoreValueType::BigDecimal,
            },
            CoreValue::Text(_) => CoreValueType::Text,
            CoreValue::Null => CoreValueType::Null,
            CoreValue::Endpoint(_) => CoreValueType::Endpoint,
            CoreValue::Array(_) => CoreValueType::Array,
            CoreValue::Object(_) => CoreValueType::Object,
            CoreValue::Tuple(_) => CoreValueType::Tuple,
            CoreValue::Integer(_) => CoreValueType::Integer,
            CoreValue::Decimal(_) => CoreValueType::Decimal,
        }
    }

    pub fn cast_to(&self, target_type: CoreValueType) -> Option<CoreValue> {
        match target_type {
            CoreValueType::Bool => Some(CoreValue::Bool(self.cast_to_bool()?)),
            CoreValueType::I8
            | CoreValueType::I16
            | CoreValueType::I32
            | CoreValueType::I64
            | CoreValueType::I128
            | CoreValueType::U8
            | CoreValueType::U16
            | CoreValueType::U32
            | CoreValueType::U64
            | CoreValueType::U128 => {
                Some(CoreValue::TypedInteger(self.cast_to_integer()?))
            }
            CoreValueType::F32 | CoreValueType::F64 => {
                Some(CoreValue::TypedDecimal(self.cast_to_decimal()?))
            }
            CoreValueType::Text => Some(CoreValue::Text(self.cast_to_text())),
            CoreValueType::Null => Some(CoreValue::Null),
            CoreValueType::Endpoint => {
                Some(CoreValue::Endpoint(self.cast_to_endpoint()?))
            }
            CoreValueType::Array => {
                Some(CoreValue::Array(self.cast_to_array()?))
            }
            CoreValueType::Object => {
                Some(CoreValue::Object(self.cast_to_object()?))
            }
            CoreValueType::Tuple => {
                Some(CoreValue::Tuple(self.cast_to_tuple()?))
            }
            CoreValueType::Integer => {
                Some(CoreValue::Integer(self.cast_to_integer()?.into()))
            }
            CoreValueType::Decimal => {
                Some(CoreValue::Decimal(self.cast_to_decimal()?.into()))
            }
            _ => todo!(),
        }
    }

    pub fn cast_to_text(&self) -> Text {
        match self {
            CoreValue::Text(text) => text.clone(),
            _ => Text(self.to_string()),
        }
    }

    pub fn cast_to_bool(&self) -> Option<Boolean> {
        match self {
            CoreValue::Text(text) => match text.0.as_str() {
                // FIXME can we combine thruthiness and casts??!
                "true" | "1" | "yes" => Some(Boolean(true)),
                "false" | "0" | "no" => Some(Boolean(false)),
                _ => None,
            },
            CoreValue::Bool(bool) => Some(bool.clone()),
            CoreValue::TypedInteger(int) => Some(Boolean(int.as_i128()? != 0)),
            CoreValue::Null => Some(Boolean(false)),
            _ => None,
        }
    }

    pub fn cast_to_decimal(&self) -> Option<TypedDecimal> {
        match self {
            CoreValue::Text(text) => {
                text.to_string().parse::<f64>().ok().map(TypedDecimal::from)
            }
            CoreValue::TypedInteger(int) => {
                Some(TypedDecimal::from(int.as_i128()? as f64))
            }
            CoreValue::TypedDecimal(decimal) => Some(decimal.clone()),
            _ => None,
        }
    }

    pub fn cast_to_integer(&self) -> Option<TypedInteger> {
        match self {
            CoreValue::Text(text) => text
                .to_string()
                .parse::<i128>()
                .ok()
                .map(TypedInteger::from),
            CoreValue::TypedInteger(int) => Some(*int),
            CoreValue::Integer(Integer(int)) => Some(*int),
            CoreValue::Decimal(decimal) => {
                Some(TypedInteger::from(decimal.0.as_f64() as i64))
            }
            CoreValue::TypedDecimal(decimal) => {
                Some(TypedInteger::from(decimal.as_f64() as i64))
            }
            _ => None,
        }
    }

    pub fn cast_to_endpoint(&self) -> Option<Endpoint> {
        match self {
            CoreValue::Text(text) => Endpoint::try_from(text.as_str()).ok(),
            CoreValue::Endpoint(endpoint) => Some(endpoint.clone()),
            _ => None,
        }
    }

    pub fn cast_to_array(&self) -> Option<Array> {
        match self {
            CoreValue::Array(array) => Some(array.clone()),
            _ => None,
        }
    }

    pub fn cast_to_object(&self) -> Option<Object> {
        match self {
            CoreValue::Tuple(tuple) => {
                Some(Object::from(tuple.entries.clone()))
            }
            CoreValue::Object(object) => Some(object.clone()),
            _ => None,
        }
    }

    pub fn cast_to_tuple(&self) -> Option<Tuple> {
        match self {
            CoreValue::Object(object) => Some(Tuple::from(object.0.clone())),
            CoreValue::Tuple(tuple) => Some(tuple.clone()),
            _ => None,
        }
    }
}

impl Add for CoreValue {
    type Output = Result<CoreValue, ValueError>;
    fn add(self, rhs: CoreValue) -> Self::Output {
        match (&self, &rhs) {
            (CoreValue::Text(text), other) => {
                let other = other.cast_to_text();
                Ok(CoreValue::Text(text + other))
            }
            (other, CoreValue::Text(text)) => {
                let other = other.cast_to_text();
                Ok(CoreValue::Text(other + text))
            }

            // Integers
            (CoreValue::Integer(lhs), CoreValue::Integer(rhs)) => {
                Ok(CoreValue::Integer(
                    (lhs + rhs).ok_or(ValueError::IntegerOverflow)?,
                ))
            }

            (
                CoreValue::TypedInteger(lhs) | CoreValue::Integer(Integer(lhs)),
                CoreValue::TypedInteger(rhs) | CoreValue::Integer(Integer(rhs)),
            ) => Ok(CoreValue::TypedInteger(
                (lhs + rhs).ok_or(ValueError::IntegerOverflow)?,
            )),

            // Decimals
            (CoreValue::Decimal(lhs), CoreValue::Decimal(rhs)) => {
                Ok(CoreValue::Decimal(lhs + rhs))
            }

            (
                CoreValue::TypedDecimal(lhs) | CoreValue::Decimal(Decimal(lhs)),
                CoreValue::TypedDecimal(rhs) | CoreValue::Decimal(Decimal(rhs)),
            ) => Ok(CoreValue::TypedDecimal(lhs + rhs)),

            // Mixed Integer and Decimal additions
            (
                CoreValue::Decimal(Decimal(decimal))
                | CoreValue::TypedDecimal(decimal),
                CoreValue::Integer(Integer(integer))
                | CoreValue::TypedInteger(integer),
            )
            | (
                CoreValue::Integer(Integer(integer))
                | CoreValue::TypedInteger(integer),
                CoreValue::Decimal(Decimal(decimal))
                | CoreValue::TypedDecimal(decimal),
            ) => {
                // convert integer to float
                let int_as_float = if integer.is_signed() {
                    integer.as_i128().unwrap() as f64
                } else {
                    integer.as_u128() as f64
                };
                Ok(CoreValue::TypedDecimal(
                    decimal + &TypedDecimal::from(int_as_float),
                ))
            }

            _ => Err(ValueError::InvalidOperation),
        }
    }
}

impl Add for &CoreValue {
    type Output = Result<CoreValue, ValueError>;
    fn add(self, rhs: &CoreValue) -> Self::Output {
        CoreValue::add(self.clone(), rhs.clone())
    }
}

impl Sub for CoreValue {
    type Output = Result<CoreValue, ValueError>;
    fn sub(self, rhs: CoreValue) -> Self::Output {
        match (&self, &rhs) {
            (CoreValue::Integer(lhs), CoreValue::Integer(rhs)) => {
                Ok(CoreValue::Integer(
                    (lhs - rhs).ok_or(ValueError::IntegerOverflow)?,
                ))
            }

            (
                CoreValue::TypedInteger(lhs) | CoreValue::Integer(Integer(lhs)),
                CoreValue::TypedInteger(rhs) | CoreValue::Integer(Integer(rhs)),
            ) => Ok(CoreValue::TypedInteger(
                (lhs - rhs).ok_or(ValueError::IntegerOverflow)?,
            )),

            (CoreValue::Decimal(lhs), CoreValue::Decimal(rhs)) => {
                Ok(CoreValue::Decimal(lhs - rhs))
            }

            (
                CoreValue::TypedDecimal(lhs) | CoreValue::Decimal(Decimal(lhs)),
                CoreValue::TypedDecimal(rhs) | CoreValue::Decimal(Decimal(rhs)),
            ) => Ok(CoreValue::TypedDecimal(lhs - rhs)),

            _ => Err(ValueError::InvalidOperation),
        }
    }
}

impl Sub for &CoreValue {
    type Output = Result<CoreValue, ValueError>;
    fn sub(self, rhs: &CoreValue) -> Self::Output {
        CoreValue::sub(self.clone(), rhs.clone())
    }
}


impl AddAssign<CoreValue> for CoreValue {
    fn add_assign(&mut self, rhs: CoreValue) {
        let res = self.clone() + rhs;
        if let Ok(value) = res {
            *self = value;
        } else {
            panic!("Failed to add value: {res:?}");
        }
    }
}

impl Not for CoreValue {
    type Output = Option<CoreValue>;

    fn not(self) -> Self::Output {
        match self {
            CoreValue::Bool(bool) => Some(CoreValue::Bool(!bool)),
            _ => None, // Not applicable for other types
        }
    }
}

impl Display for CoreValue {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            CoreValue::Bool(bool) => write!(f, "{bool}"),
            CoreValue::TypedInteger(int) => write!(f, "{int}"),
            CoreValue::TypedDecimal(decimal) => write!(f, "{decimal}"),
            CoreValue::Text(text) => write!(f, "{text}"),
            CoreValue::Null => write!(f, "null"),
            CoreValue::Endpoint(endpoint) => write!(f, "{endpoint}"),
            CoreValue::Array(array) => write!(f, "{array}"),
            CoreValue::Object(object) => write!(f, "{object}"),
            CoreValue::Tuple(tuple) => write!(f, "{tuple}"),
            CoreValue::Integer(integer) => write!(f, "{integer}"),
            CoreValue::Decimal(decimal) => write!(f, "{decimal}"),
        }
    }
}

#[cfg(test)]
/// This module contains tests for the CoreValue struct.
/// Each CoreValue is a representation of a underlying native value.
/// The tests cover addition, casting, and type conversions.
mod tests {
    use log::{debug, info};

    use crate::logger::init_logger;

    use super::*;

    #[test]
    fn test_addition() {
        init_logger();
        let a = CoreValue::from(42i32);
        let b = CoreValue::from(11i32);
        let c = CoreValue::from("11");

        assert_eq!(a.get_default_type(), CoreValueType::I32);
        assert_eq!(b.get_default_type(), CoreValueType::I32);
        assert_eq!(c.get_default_type(), CoreValueType::Text);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.clone().get_default_type(), CoreValueType::I32);
        assert_eq!(a_plus_b.clone(), CoreValue::from(53));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn test_endpoint() {
        let endpoint: Endpoint = CoreValue::from("@test").try_into().unwrap();
        debug!("Endpoint: {}", endpoint);
        assert_eq!(endpoint.to_string(), "@test");
    }

    #[test]
    fn test_integer_decimal_casting() {
        let int_value = CoreValue::from(42);
        assert_eq!(
            int_value.cast_to(CoreValueType::Decimal).unwrap(),
            CoreValue::from(Decimal::from(42.0))
        );

        let decimal_value = CoreValue::from(Decimal::from(42.7));
        assert_eq!(
            decimal_value.cast_to(CoreValueType::Integer).unwrap(),
            CoreValue::from(Integer::from(42))
        );
    }

    #[test]
    fn test_boolean_casting() {
        let bool_value = CoreValue::from(true);
        assert_eq!(
            bool_value.cast_to(CoreValueType::Bool).unwrap(),
            CoreValue::from(true)
        );

        let int_value = CoreValue::from(1);
        assert_eq!(
            int_value.cast_to(CoreValueType::Bool).unwrap(),
            CoreValue::from(true)
        );

        let zero_int_value = CoreValue::from(0);
        assert_eq!(
            zero_int_value.cast_to(CoreValueType::Bool).unwrap(),
            CoreValue::from(false)
        );

        let text_value = CoreValue::from("true");
        assert_eq!(
            text_value.cast_to(CoreValueType::Bool).unwrap(),
            CoreValue::from(true)
        );
        let false_text_value = CoreValue::from("false");
        assert_eq!(
            false_text_value.cast_to(CoreValueType::Bool).unwrap(),
            CoreValue::from(false)
        );
        let invalid_text_value = CoreValue::from("invalid");
        assert_eq!(invalid_text_value.cast_to(CoreValueType::Bool), None);
    }

    #[test]
    fn test_invalid_casting() {
        let text_value = CoreValue::from("Hello, World!");
        assert_eq!(text_value.cast_to(CoreValueType::Integer), None);
        assert_eq!(text_value.cast_to(CoreValueType::I16), None);
        assert_eq!(text_value.cast_to(CoreValueType::I32), None);
        assert_eq!(text_value.cast_to(CoreValueType::I64), None);
        assert_eq!(text_value.cast_to(CoreValueType::F32), None);
        assert_eq!(text_value.cast_to(CoreValueType::F64), None);

        let int_value = CoreValue::from(42);
        assert_eq!(int_value.cast_to(CoreValueType::Endpoint), None);
        assert_eq!(int_value.cast_to(CoreValueType::Array), None);
        assert_eq!(int_value.cast_to(CoreValueType::Object), None);
        assert_eq!(int_value.cast_to(CoreValueType::Tuple), None);
    }
}
