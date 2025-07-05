use serde::{
    Deserialize, Deserializer,
    de::{IntoDeserializer, Visitor},
    forward_to_deserialize_any,
};

use crate::{
    runtime::execution::{ExecutionInput, ExecutionOptions, execute_dxb_sync},
    values::{
        core_value::CoreValue,
        core_values::integer::{integer::Integer, typed_integer::TypedInteger},
        datex_struct::error::SerializationError,
        value,
        value_container::ValueContainer,
    },
};

pub struct DatexDeserializer {
    value: ValueContainer,
}

impl<'de> DatexDeserializer {
    pub fn from_bytes(input: &'de [u8]) -> Result<Self, SerializationError> {
        let context = ExecutionInput::new_with_dxb_and_options(
            input,
            ExecutionOptions { verbose: true },
        );
        let value = execute_dxb_sync(context)
            .unwrap_or_else(|err| {
                panic!("Execution failed: {err}");
            })
            .unwrap();
        Ok(Self { value })
    }

    fn from_value(value: ValueContainer) -> Self {
        Self { value }
    }
}
impl<'de> IntoDeserializer<'de, SerializationError> for DatexDeserializer {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
impl<'de> Deserializer<'de> for DatexDeserializer {
    type Error = SerializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            ValueContainer::Value(value::Value { inner, .. }) => match inner {
                CoreValue::Null => visitor.visit_unit(),
                CoreValue::Bool(b) => visitor.visit_bool(b.0),
                CoreValue::Integer(Integer {
                    0: TypedInteger::I64(i),
                }) => visitor.visit_i64(i),
                CoreValue::Integer(Integer {
                    0: TypedInteger::U64(u),
                }) => visitor.visit_u64(u),
                CoreValue::Integer(Integer {
                    0: TypedInteger::I32(i),
                }) => visitor.visit_i32(i),
                CoreValue::Integer(Integer {
                    0: TypedInteger::U32(u),
                }) => visitor.visit_u32(u),
                CoreValue::Integer(Integer {
                    0: TypedInteger::I16(i),
                }) => visitor.visit_i16(i),
                CoreValue::Integer(Integer {
                    0: TypedInteger::U16(u),
                }) => visitor.visit_u16(u),
                CoreValue::Integer(Integer {
                    0: TypedInteger::I8(i),
                }) => visitor.visit_i8(i),
                CoreValue::Integer(Integer {
                    0: TypedInteger::U8(u),
                }) => visitor.visit_u8(u),
                CoreValue::Text(s) => visitor.visit_string(s.0),
                CoreValue::Object(obj) => {
                    let map = obj
                        .into_iter()
                        .map(|(k, v)| (k, DatexDeserializer::from_value(v)));
                    visitor
                        .visit_map(serde::de::value::MapDeserializer::new(map))
                }
                e => unreachable!("Unsupported core value: {:?}", e),
            },
            _ => unreachable!("Refs are not supported in deserialization"),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub fn from_bytes<'de, T>(input: &'de [u8]) -> Result<T, SerializationError>
where
    T: serde::Deserialize<'de>,
{
    let deserializer = DatexDeserializer::from_bytes(input)?;
    T::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use crate::values::datex_struct::serializer::to_bytes;

    use super::*;

    #[derive(Deserialize, Serialize, Debug)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_from_bytes() {
        let data = to_bytes(&TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        })
        .unwrap();
        let result: TestStruct = from_bytes(&data).unwrap();
        assert!(!result.field1.is_empty());
        println!("Deserialized: {result:?}");
    }
}
