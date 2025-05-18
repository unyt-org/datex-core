use std::{fmt, ops::AddAssign};

use super::{
    datex_type::DatexType, datex_value::DatexValue,
    typed_datex_value::TypedDatexValue, value::Value,
};

#[derive(Clone, Debug, Default)]
pub struct DatexArray(pub Vec<DatexValue>);
impl DatexArray {
    pub fn length(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Option<&DatexValue> {
        self.0.get(index)
    }

    pub fn push<T: Into<DatexValue>>(&mut self, value: T) {
        self.0.push(value.into());
    }
}
impl Value for DatexArray {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: DatexType) -> Option<DatexValue> {
        match target {
            DatexType::Array => Some(self.as_datex_value()),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> DatexValue {
        DatexValue::boxed(self.clone())
    }

    fn add(&self, other: &dyn Value) -> Option<DatexValue> {
        None
    }

    fn static_type() -> DatexType {
        DatexType::Array
    }

    fn get_type(&self) -> DatexType {
        Self::static_type()
    }
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        DatexArray(vec![])
    }
}

impl fmt::Display for DatexArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, value) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", value)?;
        }
        write!(f, "]")
    }
}

impl<T> From<Vec<T>> for DatexArray
where
    T: Into<DatexValue>,
{
    fn from(vec: Vec<T>) -> Self {
        DatexArray(vec.into_iter().map(Into::into).collect())
    }
}

#[macro_export]
macro_rules! datex_array {
    ( $( $x:expr ),* ) => {
        {
            let arr = vec![$( DatexValue::from($x) ),*];
            DatexArray(arr)
        }
    };
}

impl<T> AddAssign<T> for TypedDatexValue<DatexArray>
where
    DatexValue: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.0.push(DatexValue::from(rhs));
    }
}
