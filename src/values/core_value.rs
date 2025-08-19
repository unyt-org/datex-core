use datex_macros::FromCoreValue;

use crate::values::core_values::array::Array;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::object::Object;
use crate::values::core_values::text::Text;
use crate::values::core_values::tuple::Tuple;
use crate::values::datex_type::CoreValueType;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::value_container::{ValueContainer, ValueError};
use serde::Deserialize;
use serde_with::serde_derive::Serialize;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not, Sub};

#[derive(Clone, Debug, PartialEq, Eq, Hash, FromCoreValue)]
pub enum CoreValue {
    Null,
    Boolean(Boolean),
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
impl StructuralEq for CoreValue {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CoreValue::Boolean(a), CoreValue::Boolean(b)) => {
                a.structural_eq(b)
            }

            // Integers
            (CoreValue::Integer(a), CoreValue::Integer(b)) => {
                a.structural_eq(b)
            }

            // TypedIntegers
            (CoreValue::TypedInteger(a), CoreValue::TypedInteger(b)) => {
                a.structural_eq(b)
            }

            // Integers + TypedIntegers
            (CoreValue::Integer(a), CoreValue::TypedInteger(b))
            | (CoreValue::TypedInteger(b), CoreValue::Integer(a)) => {
                TypedInteger::Big(a.clone()).structural_eq(b)
            }

            // Decimals
            (CoreValue::Decimal(a), CoreValue::Decimal(b)) => {
                a.structural_eq(b)
            }

            // TypedDecimals
            (CoreValue::TypedDecimal(a), CoreValue::TypedDecimal(b)) => {
                a.structural_eq(b)
            }

            // Decimal + TypedDecimal
            (CoreValue::Decimal(a), CoreValue::TypedDecimal(b))
            | (CoreValue::TypedDecimal(b), CoreValue::Decimal(a)) => {
                TypedDecimal::Decimal(a.clone()).structural_eq(b)
            }

            (CoreValue::Text(a), CoreValue::Text(b)) => a.structural_eq(b),
            (CoreValue::Null, CoreValue::Null) => true,
            (CoreValue::Endpoint(a), CoreValue::Endpoint(b)) => {
                a.structural_eq(b)
            }
            (CoreValue::Array(a), CoreValue::Array(b)) => a.structural_eq(b),
            (CoreValue::Object(a), CoreValue::Object(b)) => a.structural_eq(b),
            (CoreValue::Tuple(a), CoreValue::Tuple(b)) => a.structural_eq(b),

            _ => false,
        }
    }
}

/// value equality corresponds to partial equality for values
impl ValueEq for CoreValue {
    fn value_eq(&self, other: &Self) -> bool {
        self == other
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
        CoreValue::Boolean(value.into())
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

    /// Check if the CoreValue is a combined value type (Array, Object, Tuple)
    /// that consists of multiple CoreValues.
    pub fn is_combined_value(&self) -> bool {
        matches!(
            self,
            CoreValue::Array(_) | CoreValue::Object(_) | CoreValue::Tuple(_)
        )
    }

    pub fn get_default_type(&self) -> CoreValueType {
        match self {
            CoreValue::Boolean(_) => CoreValueType::Boolean,
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

                TypedInteger::Big(_) => CoreValueType::Integer,
            },
            CoreValue::TypedDecimal(decimal) => match decimal {
                TypedDecimal::F32(_) => CoreValueType::F32,
                TypedDecimal::F64(_) => CoreValueType::F64,
                TypedDecimal::Decimal(_) => CoreValueType::Decimal,
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
            CoreValueType::Boolean => {
                Some(CoreValue::Boolean(self.cast_to_bool()?))
            }
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
                Some(CoreValue::TypedDecimal(self.cast_to_float()?))
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
            CoreValueType::Decimal => Some(CoreValue::Decimal(
                Decimal::from_string(self.cast_to_text().as_str()),
            )),
            _ => todo!("#116 Undescribed by author."),
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
            CoreValue::Text(text) => Some(Boolean(!text.0.is_empty())),
            CoreValue::Boolean(bool) => Some(bool.clone()),
            CoreValue::TypedInteger(int) => Some(Boolean(int.as_i128()? != 0)),
            CoreValue::Null => Some(Boolean(false)),
            _ => None,
        }
    }

    pub fn cast_to_float(&self) -> Option<TypedDecimal> {
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

    // FIXME discuss here - shall we fit the integer in the minimum viable type?
    pub fn cast_to_integer(&self) -> Option<TypedInteger> {
        match self {
            CoreValue::Text(text) => Integer::from_string(&text.to_string())
                .map(|x| Some(x.to_smallest_fitting()))
                .unwrap_or(None),
            CoreValue::TypedInteger(int) => {
                Some(int.to_smallest_fitting().clone())
            }
            CoreValue::Integer(int) => {
                Some(TypedInteger::Big(int.clone()).to_smallest_fitting())
            }
            CoreValue::Decimal(decimal) => Some(
                TypedInteger::from(decimal.try_into_f64()? as i128)
                    .to_smallest_fitting(),
            ),
            CoreValue::TypedDecimal(decimal) => Some(
                TypedInteger::from(decimal.as_f64() as i64)
                    .to_smallest_fitting(),
            ),
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
            // x + text or text + x (order does not matter)
            (CoreValue::Text(text), other) => {
                let other = other.cast_to_text();
                return Ok(CoreValue::Text(text + other));
            }
            (other, CoreValue::Text(text)) => {
                let other = other.cast_to_text();
                return Ok(CoreValue::Text(other + text));
            }

            // same type additions
            (CoreValue::TypedInteger(lhs), CoreValue::TypedInteger(rhs)) => {
                return Ok(CoreValue::TypedInteger(
                    (lhs + rhs).ok_or(ValueError::IntegerOverflow)?,
                ));
            }
            (CoreValue::Integer(lhs), CoreValue::Integer(rhs)) => {
                return Ok(CoreValue::Integer(lhs + rhs));
            }
            (CoreValue::TypedDecimal(lhs), CoreValue::TypedDecimal(rhs)) => {
                return Ok(CoreValue::TypedDecimal(lhs + rhs));
            }
            (CoreValue::Decimal(lhs), CoreValue::Decimal(rhs)) => {
                return Ok(CoreValue::Decimal(lhs + rhs));
            }

            _ => {}
        }

        // other cases
        match &self {
            // integer
            CoreValue::Integer(lhs) => match &rhs {
                CoreValue::TypedInteger(rhs) => Ok(CoreValue::Integer(
                    Integer::from(lhs.clone() + rhs.as_integer()),
                )),
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        .cast_to_integer()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::Integer(Integer::from(
                        lhs.clone() + integer.as_integer(),
                    )))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::Integer(Integer::from(
                        lhs.clone() + integer.as_integer(),
                    )))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // typed integer
            CoreValue::TypedInteger(lhs) => match &rhs {
                CoreValue::Integer(rhs) => {
                    todo!("TypedInteger + Integer not implemented yet");
                    //Ok(CoreValue::TypedInteger(lhs.as_integer() + rhs.clone()))
                }
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        .cast_to_integer()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::TypedInteger(
                        (lhs + &integer).ok_or(ValueError::IntegerOverflow)?,
                    ))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::TypedInteger(
                        (lhs + &integer).ok_or(ValueError::IntegerOverflow)?,
                    ))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // decimal
            CoreValue::Decimal(lhs) => match rhs {
                CoreValue::TypedDecimal(rhs) => {
                    Ok(CoreValue::Decimal(lhs + &Decimal::from(rhs)))
                }
                CoreValue::TypedInteger(rhs) => {
                    let decimal = Decimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::Decimal(lhs + &decimal))
                }
                CoreValue::Integer(rhs) => {
                    let decimal = Decimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::Decimal(lhs + &decimal))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // typed decimal
            CoreValue::TypedDecimal(lhs) => match rhs {
                CoreValue::Decimal(rhs) => Ok(CoreValue::TypedDecimal(
                    lhs + &TypedDecimal::Decimal(rhs),
                )),
                CoreValue::TypedInteger(rhs) => {
                    let decimal = TypedDecimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::TypedDecimal(lhs + &decimal))
                }
                CoreValue::Integer(rhs) => {
                    let decimal = TypedDecimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::TypedDecimal(lhs + &decimal))
                }
                _ => Err(ValueError::InvalidOperation),
            },

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
        // same type subtractions
        match (&self, &rhs) {
            (CoreValue::TypedInteger(lhs), CoreValue::TypedInteger(rhs)) => {
                return Ok(CoreValue::TypedInteger(
                    (lhs - rhs).ok_or(ValueError::IntegerOverflow)?,
                ));
            }
            (CoreValue::Integer(lhs), CoreValue::Integer(rhs)) => {
                return Ok(CoreValue::Integer(lhs - rhs));
            }
            (CoreValue::TypedDecimal(lhs), CoreValue::TypedDecimal(rhs)) => {
                return Ok(CoreValue::TypedDecimal(lhs - rhs));
            }
            (CoreValue::Decimal(lhs), CoreValue::Decimal(rhs)) => {
                return Ok(CoreValue::Decimal(lhs - rhs));
            }

            _ => {}
        }

        // other cases
        match &self {
            // integer
            CoreValue::Integer(lhs) => match &rhs {
                CoreValue::TypedInteger(rhs) => Ok(CoreValue::Integer(
                    Integer::from(lhs - &rhs.as_integer()),
                )),
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        .cast_to_integer()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::Integer(Integer::from(
                        lhs - &integer.as_integer(),
                    )))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::Integer(Integer::from(
                        lhs - &integer.as_integer(),
                    )))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // typed integer
            CoreValue::TypedInteger(lhs) => match &rhs {
                CoreValue::Integer(rhs) => {
                    todo!("TypedInteger - Integer not implemented yet");
                    //Ok(CoreValue::TypedInteger(lhs.as_integer() - rhs.clone()))
                }
                //     Ok(CoreValue::TypedInteger(
                //     (lhs - &rhs.0).ok_or(ValueError::IntegerOverflow)?,
                // ))
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        .cast_to_integer()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::TypedInteger(
                        (lhs - &integer).ok_or(ValueError::IntegerOverflow)?,
                    ))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::TypedInteger(
                        (lhs - &integer).ok_or(ValueError::IntegerOverflow)?,
                    ))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // decimal
            CoreValue::Decimal(lhs) => match rhs {
                CoreValue::TypedDecimal(rhs) => {
                    Ok(CoreValue::Decimal(lhs - &Decimal::from(rhs)))
                }
                CoreValue::TypedInteger(rhs) => {
                    let decimal = Decimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::Decimal(lhs - &decimal))
                }
                CoreValue::Integer(rhs) => {
                    let decimal = Decimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::Decimal(lhs - &decimal))
                }
                _ => Err(ValueError::InvalidOperation),
            },

            // typed decimal
            CoreValue::TypedDecimal(lhs) => match rhs {
                CoreValue::Decimal(rhs) => Ok(CoreValue::TypedDecimal(
                    lhs - &TypedDecimal::Decimal(rhs),
                )),
                CoreValue::TypedInteger(rhs) => {
                    let decimal = TypedDecimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::TypedDecimal(lhs - &decimal))
                }
                CoreValue::Integer(rhs) => {
                    let decimal = TypedDecimal::from(
                        rhs.as_i128().ok_or(ValueError::IntegerOverflow)?
                            as f64,
                    );
                    Ok(CoreValue::TypedDecimal(lhs - &decimal))
                }
                _ => Err(ValueError::InvalidOperation),
            },
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
            CoreValue::Boolean(bool) => Some(CoreValue::Boolean(!bool)),
            _ => None, // Not applicable for other types
        }
    }
}

impl Display for CoreValue {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            CoreValue::Boolean(bool) => write!(f, "{bool}"),
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

    use crate::logger::init_logger_debug;

    use super::*;

    #[test]
    fn test_addition() {
        init_logger_debug();
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
        debug!("Endpoint: {endpoint}");
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
            bool_value.cast_to(CoreValueType::Boolean).unwrap(),
            CoreValue::from(true)
        );

        let int_value = CoreValue::from(1);
        assert_eq!(
            int_value.cast_to(CoreValueType::Boolean).unwrap(),
            CoreValue::from(true)
        );

        let zero_int_value = CoreValue::from(0);
        assert_eq!(
            zero_int_value.cast_to(CoreValueType::Boolean).unwrap(),
            CoreValue::from(false)
        );

        let invalid_text_value = CoreValue::from("sometext");
        assert_eq!(
            invalid_text_value.cast_to(CoreValueType::Boolean),
            Some(CoreValue::from(true))
        );
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
