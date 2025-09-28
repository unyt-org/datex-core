use datex_macros::FromCoreValue;

use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
use crate::types::type_container::TypeContainer;
use crate::values::core_values::array::Array;
use crate::values::core_values::boolean::Boolean;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::core_values::r#struct::Struct;
use crate::values::core_values::text::Text;
use crate::values::core_values::r#type::Type;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::value_container::{ValueContainer, ValueError};
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not, Sub};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeTag {
    // human-readable name of the type tag, e.g. "integer"
    pub name: String,
    // allowed variants for this type tag, e.g. ["i8", "i16", ...]
    pub variants: Vec<String>,
}

impl TypeTag {
    pub fn new(name: &str, variants: &[&str]) -> Self {
        TypeTag {
            name: name.to_string(),
            variants: variants.iter().map(|s| s.to_string()).collect(),
        }
    }
}

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
    List(List),
    Map(Map),
    Array(Array),
    Struct(Struct),
    Type(Type),
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
            (CoreValue::List(a), CoreValue::List(b)) => a.structural_eq(b),
            (CoreValue::Map(a), CoreValue::Map(b)) => a.structural_eq(b),
            (CoreValue::Array(a), CoreValue::Array(b)) => a.structural_eq(b),
            (CoreValue::Struct(a), CoreValue::Struct(b)) => a.structural_eq(b),
            _ => false,
        }
    }
}

/// Value equality corresponds to partial equality for all values,
/// except for decimals, where partial equality is also given for NaN values and +0.0 and -0.0.
/// Therefore, we ValueEq is used instead for decimals
impl ValueEq for CoreValue {
    fn value_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CoreValue::Decimal(a), CoreValue::Decimal(b)) => a.value_eq(b),
            (CoreValue::TypedDecimal(a), CoreValue::TypedDecimal(b)) => {
                a.value_eq(b)
            }
            _ => self == other,
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
        CoreValue::Array(Array::new(iter.into_iter().map(Into::into).collect()))
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

impl From<&CoreValue> for CoreLibPointerId {
    fn from(value: &CoreValue) -> Self {
        match value {
            CoreValue::Map(_) => CoreLibPointerId::Map,
            CoreValue::List(_) => CoreLibPointerId::List,
            CoreValue::Struct(_) => CoreLibPointerId::Struct,
            CoreValue::Array(_) => CoreLibPointerId::Array,
            CoreValue::Text(_) => CoreLibPointerId::Text,
            CoreValue::Boolean(_) => CoreLibPointerId::Boolean,
            CoreValue::TypedInteger(i) => {
                CoreLibPointerId::Integer(Some(i.variant()))
            }
            CoreValue::TypedDecimal(d) => {
                CoreLibPointerId::Decimal(Some(d.variant()))
            }
            CoreValue::Integer(_) => CoreLibPointerId::Integer(None),
            CoreValue::Decimal(_) => CoreLibPointerId::Decimal(None),
            CoreValue::Endpoint(_) => CoreLibPointerId::Endpoint,
            CoreValue::Null => CoreLibPointerId::Null,
            CoreValue::Type(_) => CoreLibPointerId::Type,
        }
    }
}

impl CoreValue {
    pub fn new<T>(value: T) -> CoreValue
    where
        CoreValue: From<T>,
    {
        value.into()
    }

    /// Check if the CoreValue is a combined value type (Array, List, Struct, Map)
    /// that consists of multiple CoreValues.
    pub fn is_collection_value(&self) -> bool {
        matches!(
            self,
            CoreValue::List(_)
                | CoreValue::Map(_)
                | CoreValue::Struct(_)
                | CoreValue::Array(_)
        )
    }

    /// Get the default type of the CoreValue as a TypeContainer.
    /// This method uses the CoreLibPointerId to retrieve the corresponding
    /// type reference from the core library.
    /// For example, a CoreValue::TypedInteger(i32) will return the type ref integer/i32
    pub fn get_default_type(&self) -> TypeContainer {
        get_core_lib_type(CoreLibPointerId::from(self))
    }

    pub fn cast_to_type(&self) -> Option<&Type> {
        match self {
            CoreValue::Type(ty) => Some(ty),
            _ => None,
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

    pub fn cast_to_list(&self) -> Option<List> {
        match self {
            CoreValue::List(list) => Some(list.clone()),
            _ => None,
        }
    }

    pub fn cast_to_array(&self) -> Option<Array> {
        match self {
            CoreValue::Array(array) => Some(array.clone()),
            _ => None,
        }
    }

    pub fn cast_to_map(&self) -> Option<Map> {
        match self {
            CoreValue::Map(map) => Some(map.clone()),
            _ => None,
        }
    }
    pub fn cast_to_struct(&self) -> Option<Struct> {
        match self {
            CoreValue::Struct(structure) => Some(structure.clone()),
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
                CoreValue::TypedInteger(rhs) => {
                    Ok(CoreValue::Integer((lhs.clone() + rhs.as_integer())))
                }
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        .cast_to_integer()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::Integer((lhs.clone() + integer.as_integer())))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::Integer((lhs.clone() + integer.as_integer())))
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
                CoreValue::TypedInteger(rhs) => {
                    Ok(CoreValue::Integer((lhs - &rhs.as_integer())))
                }
                CoreValue::Decimal(_) => {
                    let integer = rhs
                        .cast_to_integer()
                        .ok_or(ValueError::InvalidOperation)?;
                    Ok(CoreValue::Integer((lhs - &integer.as_integer())))
                }
                CoreValue::TypedDecimal(rhs) => {
                    let decimal = rhs.as_f64();
                    let integer = TypedInteger::from(decimal as i128);
                    Ok(CoreValue::Integer((lhs - &integer.as_integer())))
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
            CoreValue::Type(ty) => write!(f, "{ty}"),
            CoreValue::Boolean(bool) => write!(f, "{bool}"),
            CoreValue::TypedInteger(int) => write!(f, "{int}"),
            CoreValue::TypedDecimal(decimal) => write!(f, "{decimal}"),
            CoreValue::Text(text) => write!(f, "{text}"),
            CoreValue::Null => write!(f, "null"),
            CoreValue::Endpoint(endpoint) => write!(f, "{endpoint}"),
            CoreValue::Map(map) => write!(f, "{map}"),
            CoreValue::Integer(integer) => write!(f, "{integer}"),
            CoreValue::Decimal(decimal) => write!(f, "{decimal}"),

            CoreValue::List(list) => write!(f, "{list}"),
            CoreValue::Struct(structure) => write!(f, "{structure}"),
            CoreValue::Array(array) => write!(f, "{array}"),
            CoreValue::Map(map) => write!(f, "{map}"),
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
    // WIP
    fn type_construct() {
        init_logger_debug();
        let a = CoreValue::from(42i32);
        assert_eq!(a.get_default_type().to_string(), "integer/i32");
        assert_eq!(a.get_default_type().base_type().to_string(), "integer");
    }

    #[test]
    fn addition() {
        init_logger_debug();
        let a = CoreValue::from(42i32);
        let b = CoreValue::from(11i32);
        let c = CoreValue::from("11");

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.clone(), CoreValue::from(53));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn endpoint() {
        let endpoint: Endpoint = CoreValue::from("@test").try_into().unwrap();
        debug!("Endpoint: {endpoint}");
        assert_eq!(endpoint.to_string(), "@test");
    }
}
