use std::{
    fmt,
    ops::{AddAssign, Index},
};
use crate::datex_values::value_container::ValueContainer;
use super::super::{
    core_value::CoreValue,
    datex_type::CoreValueType,
    typed_value::TypedValue,
    value::Value,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Array(pub Vec<ValueContainer>);
impl Array {
    pub fn length(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Option<&ValueContainer> {
        self.0.get(index)
    }

    pub fn push<T: Into<ValueContainer>>(&mut self, value: T) {
        self.0.push(value.into());
    }
}
impl CoreValue for Array {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn cast_to(&self, target: CoreValueType) -> Option<Value> {
        match target {
            CoreValueType::Array => Some(self.as_datex_value()),
            _ => None,
        }
    }

    fn as_datex_value(&self) -> Value {
        Value::boxed(self.clone())
    }

    fn get_type(&self) -> CoreValueType {
        Self::static_type()
    }

    fn static_type() -> CoreValueType {
        CoreValueType::Array
    }
}

impl fmt::Display for Array {
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

impl<T> From<Vec<T>> for Array
where
    T: Into<ValueContainer>,
{
    fn from(vec: Vec<T>) -> Self {
        Array(vec.into_iter().map(Into::into).collect())
    }
}

impl<T> FromIterator<T> for Array
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Array(iter.into_iter().map(Into::into).collect())
    }
}

#[macro_export]
macro_rules! datex_array {
    ( $( $x:expr ),* ) => {
        {
            let arr = vec![$( $crate::datex_values::value::ValueContainer::from($x) ),*];
            Array(arr)
        }
    };
}

impl<T> AddAssign<T> for TypedValue<Array>
where
    Value: From<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.0.push(Value::from(rhs));
    }
}

impl Index<usize> for Array {
    type Output = ValueContainer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Index<usize> for TypedValue<Array> {
    type Output = ValueContainer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IntoIterator for Array {
    type Item = ValueContainer;
    type IntoIter = std::vec::IntoIter<ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl <'a> IntoIterator for &'a Array {
    type Item = &'a ValueContainer;
    type IntoIter = std::slice::Iter<'a, ValueContainer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
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
