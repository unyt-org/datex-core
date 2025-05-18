use super::{pointer::Pointer, value::Value};

#[derive(Clone)]
pub enum ValueContainer {
    Value(Value),
    Pointer(Pointer),
}
