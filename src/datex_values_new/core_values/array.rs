use std::{
    fmt,
    ops::{AddAssign, Index},
};

use super::super::{
    core_value::CoreValue,
    datex_type::Type,
    typed_value::TypedValue,
    value::{SerializableDatexValue, Value},
};

#[derive(Clone, Debug, Default)]
pub struct DatexArray(pub Vec<Value>);
impl DatexArray {
    pub fn length(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.0.get(index)
    }

    pub fn push<T: Into<Value>>(&mut self, value: T) {
        self.0.push(value.into());
    }
}
impl CoreValue for DatexArray {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: Type) -> Option<Value> {
        match target {
            Type::Array => Some(self.as_datex_value()),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(self.clone())
    }

    fn static_type() -> Type {
        Type::Array
    }

    fn get_type(&self) -> Type {
        Self::static_type()
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        for value in &self.0 {
            let repr: SerializableDatexValue = value.into();
            bytes.extend(repr.to_bytes());
        }
        bytes
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        // let mut values = vec![];
        // let mut offset = 0;
        // while offset < bytes.len() {
        //     let (value, size) =
        //         SerializableDatexValue::from_bytes(&bytes[offset..]);
        //     values.push(value);
        //     offset += size;
        // }
        // DatexArray(values)
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
            write!(f, "{value}")?;
        }
        write!(f, "]")
    }
}

impl<T> From<Vec<T>> for DatexArray
where
    T: Into<Value>,
{
    fn from(vec: Vec<T>) -> Self {
        DatexArray(vec.into_iter().map(Into::into).collect())
    }
}

#[macro_export]
macro_rules! datex_array {
    ( $( $x:expr ),* ) => {
        {
            let arr = vec![$( $crate::datex_values_new::value::Value::from($x) ),*];
            DatexArray(arr)
        }
    };
}

impl<T> AddAssign<T> for TypedValue<DatexArray>
where
    Value: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.0.push(Value::from(rhs));
    }
}

impl Index<usize> for DatexArray {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Index<usize> for TypedValue<DatexArray> {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

// FIXME: Deref and DerefMut are not implemented for DatexArray.
// If we implement these two traits, we can use all the methods of Vec<DatexValue> directly on DatexArray.
// This is not recommended most probably, but it is possible.
// Since we want to listen for changes in the array, we should not implement these traits and the spec
// shall also just mention the methods that are available on DatexArray, not all rust magic since not
// all will be implemented by runtime nor on any other platform.
// impl Deref for DatexArray {
//     type Target = Vec<DatexValue>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl DerefMut for DatexArray {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }
