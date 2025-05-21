use super::{pointer::Pointer, value::Value};

#[derive(Clone)]
pub enum ValueContainer {
    Value(Value),
    Pointer(Pointer),
}


// impl From<Value> for ValueContainer {
//     fn from(value: Value) -> Self {
//         ValueContainer::Value(value)
//     }
// }

impl<T: Into<Value>> From<T> for ValueContainer {
    fn from(value: T) -> Self {
        ValueContainer::Value(value.into())
    }
}