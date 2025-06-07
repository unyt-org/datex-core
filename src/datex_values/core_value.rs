use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not};
use crate::datex_values::core_values::array::Array;
use crate::datex_values::core_values::bool::Bool;
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::core_values::int::Integer;
use crate::datex_values::core_values::null::Null;
use crate::datex_values::core_values::object::Object;
use crate::datex_values::core_values::text::Text;
use crate::datex_values::core_values::tuple::Tuple;
use crate::datex_values::datex_type::CoreValueType;
use crate::datex_values::value_container::{ValueContainer, ValueError};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CoreValue {
    Bool(Bool),
    Integer(Integer),
    Text(Text),
    Null(Null),
    Endpoint(Endpoint),
    Array(Array),
    Object(Object),
    Tuple(Tuple),
}

impl From<Text> for CoreValue {
    fn from(value: Text) -> Self {
        CoreValue::Text(value)
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

//
impl From<Array> for CoreValue {
    fn from(value: Array) -> Self {
        CoreValue::Array(value)
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

impl From<Bool> for CoreValue {
    fn from(value: Bool) -> Self {
        CoreValue::Bool(value)
    }
}

impl From<bool> for CoreValue {
    fn from(value: bool) -> Self {
        CoreValue::Bool(value.into())
    }
}

impl From<Integer> for CoreValue {
    fn from(value: Integer) -> Self {
        CoreValue::Integer(value)
    }
}

impl From<i8> for CoreValue {
    fn from(value: i8) -> Self {
        CoreValue::Integer(value.into())
    }
}

impl From<Null> for CoreValue {
    fn from(value: Null) -> Self {
        CoreValue::Null(value)
    }
}

impl From<Endpoint> for CoreValue {
    fn from(value: Endpoint) -> Self {
        CoreValue::Endpoint(value)
    }
}

impl From<Object> for CoreValue {
    fn from(value: Object) -> Self {
        CoreValue::Object(value)
    }
}

impl From<Tuple> for CoreValue {
    fn from(value: Tuple) -> Self {
        CoreValue::Tuple(value)
    }
}

impl CoreValue {

    pub fn text<T: Into<Text>>(text: T) -> Self {
        CoreValue::Text(text.into())
    }

    pub fn get_default_type(&self) -> CoreValueType {
        match self {
            CoreValue::Bool(_) => CoreValueType::Bool,
            CoreValue::Integer(_) => CoreValueType::I8,
            CoreValue::Text(_) => CoreValueType::Text,
            CoreValue::Null(_) => CoreValueType::Null,
            CoreValue::Endpoint(_) => CoreValueType::Endpoint,
            CoreValue::Array(_) => CoreValueType::Array,
            CoreValue::Object(_) => CoreValueType::Object,
            CoreValue::Tuple(_) => CoreValueType::Tuple,
        }
    }

    pub fn cast_to(&self, target_type: CoreValueType) -> Option<CoreValue> {
        match target_type {
            CoreValueType::Bool => Some(CoreValue::Bool(self.cast_to_bool()?)),
            CoreValueType::I8 => Some(CoreValue::Integer(self.cast_to_integer()?)),
            CoreValueType::Text => Some(CoreValue::Text(self.cast_to_text()?)),
            CoreValueType::Null => Some(CoreValue::Null(Null)),
            CoreValueType::Endpoint => Some(CoreValue::Endpoint(self.cast_to_endpoint()?)),
            CoreValueType::Array => Some(CoreValue::Array(self.cast_to_array()?)),
            CoreValueType::Object => Some(CoreValue::Object(self.cast_to_object()?)),
            CoreValueType::Tuple => Some(CoreValue::Tuple(self.cast_to_tuple()?)),
            _ => todo!()
        }
    }

    pub fn cast_to_text(&self) -> Option<Text> {
        match self {
            CoreValue::Text(text) => Some(text.clone()),
            CoreValue::Integer(int) => Some(Text(int.to_string())),
            CoreValue::Bool(bool) => Some(Text(bool.to_string())),
            CoreValue::Null(_) => Some(Text("null".to_string())),
            _ => None,
        }
    }

    pub fn cast_to_bool(&self) -> Option<Bool> {
        match self {
            CoreValue::Bool(bool) => Some(bool.clone()),
            CoreValue::Integer(int) => Some(Bool(int.as_i128() != 0)), // TODO <- handle unsigned?
            CoreValue::Null(_) => Some(Bool(false)),
            _ => None,
        }
    }

    pub fn cast_to_integer(&self) -> Option<Integer> {
        match self {
            CoreValue::Integer(int) => Some(int.clone()),
            _ => None,
        }
    }

    pub fn cast_to_endpoint(&self) -> Option<Endpoint> {
        match self {
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
            CoreValue::Object(object) => Some(object.clone()),
            _ => None,
        }
    }

    pub fn cast_to_tuple(&self) -> Option<Tuple> {
        match self {
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
                let other = other.cast_to_text().ok_or(ValueError::TypeConversionError)?;
                Ok(CoreValue::Text(text + other))
            }
            (other, CoreValue::Text(text)) => {
                let other = other.cast_to_text().ok_or(ValueError::TypeConversionError)?;
                Ok(CoreValue::Text(other + text))
            }
            (CoreValue::Integer(lhs), CoreValue::Integer(rhs)) => {
                Ok(CoreValue::Integer(lhs + rhs))
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
            CoreValue::Bool(v) => write!(f, "{v}"),
            CoreValue::Integer(v) => write!(f, "{v}"),
            CoreValue::Text(v) => write!(f, "{v}"),
            CoreValue::Null(v) => write!(f, "{v}"),
            CoreValue::Endpoint(v) => write!(f, "{v}"),
            CoreValue::Array(v) => write!(f, "{v}"),
            CoreValue::Object(v) => write!(f, "{v}"),
            CoreValue::Tuple(v) => write!(f, "{v}"),
        }
    }
}
