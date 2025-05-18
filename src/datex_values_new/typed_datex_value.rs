use super::{
    datex_type::DatexType,
    datex_value::{DatexValue, Value},
    primitive::PrimitiveI8,
    text::Text,
};

#[derive(Debug, Clone)]
pub struct TypedDatexValue<T: Value>(pub T);

impl<T: Value + 'static> TypedDatexValue<T> {
    pub fn into_erased(self) -> DatexValue {
        DatexValue::boxed(self.0)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn get_type(&self) -> DatexType {
        self.0.get_type()
    }
}

impl From<PrimitiveI8> for TypedDatexValue<PrimitiveI8> {
    fn from(p: PrimitiveI8) -> Self {
        TypedDatexValue(p)
    }
}

impl From<i8> for TypedDatexValue<PrimitiveI8> {
    fn from(v: i8) -> Self {
        TypedDatexValue(PrimitiveI8(v))
    }
}

impl From<String> for TypedDatexValue<Text> {
    fn from(v: String) -> Self {
        TypedDatexValue(Text(v))
    }
}
impl From<&str> for TypedDatexValue<Text> {
    fn from(v: &str) -> Self {
        TypedDatexValue(Text(v.to_string()))
    }
}
