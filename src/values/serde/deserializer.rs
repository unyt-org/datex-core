use serde::{Deserializer, de::IntoDeserializer, forward_to_deserialize_any};

use crate::{
    compiler::{
        CompileOptions, compile_script, extract_static_value_from_script,
    },
    runtime::execution::{ExecutionInput, ExecutionOptions, execute_dxb_sync},
    values::{
        core_value::CoreValue,
        core_values::integer::{integer::Integer, typed_integer::TypedInteger},
        serde::error::SerializationError,
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

    pub fn from_dx_file(path: &str) -> Result<Self, SerializationError> {
        let input = std::fs::read_to_string(path)
            .map_err(|err| SerializationError(err.to_string()))?;
        DatexDeserializer::from_script(&input)
    }
    pub fn from_dxb_file(path: &str) -> Result<Self, SerializationError> {
        let input = std::fs::read(path)
            .map_err(|err| SerializationError(err.to_string()))?;
        DatexDeserializer::from_bytes(&input)
    }

    pub fn from_script(script: &'de str) -> Result<Self, SerializationError> {
        let (dxb, _) = compile_script(script, CompileOptions::default())
            .map_err(|err| SerializationError(err.to_string()))?;
        DatexDeserializer::from_bytes(&dxb)
    }
    pub fn from_static_script(
        script: &'de str,
    ) -> Result<Self, SerializationError> {
        let value = extract_static_value_from_script(script)
            .map_err(|err| SerializationError(err.to_string()))?;
        if value.is_none() {
            return Err(SerializationError(
                "No static value found in script".to_string(),
            ));
        }
        Ok(DatexDeserializer::from_value(value.unwrap()))
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
            // TODO implement missing mapping
            ValueContainer::Value(value::Value { inner, .. }) => match inner {
                CoreValue::Null => visitor.visit_unit(),
                CoreValue::Bool(b) => visitor.visit_bool(b.0),
                CoreValue::TypedInteger(i) => match i {
                    TypedInteger::I128(i) => visitor.visit_i128(i),
                    TypedInteger::U128(u) => visitor.visit_u128(u),
                    TypedInteger::I64(i) => visitor.visit_i64(i),
                    TypedInteger::U64(u) => visitor.visit_u64(u),
                    TypedInteger::I32(i) => visitor.visit_i32(i),
                    TypedInteger::U32(u) => visitor.visit_u32(u),
                    TypedInteger::I16(i) => visitor.visit_i16(i),
                    TypedInteger::U16(u) => visitor.visit_u16(u),
                    TypedInteger::I8(i) => visitor.visit_i8(i),
                    TypedInteger::U8(u) => visitor.visit_u8(u),
                    _ => unreachable!(),
                },
                CoreValue::Integer(Integer {
                    0: TypedInteger::I128(i),
                }) => visitor.visit_i128(i),
                CoreValue::Integer(Integer {
                    0: TypedInteger::U128(u),
                }) => visitor.visit_u128(u),
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

pub fn from_value_container<'de, T>(
    value: ValueContainer,
) -> Result<T, SerializationError>
where
    T: serde::Deserialize<'de>,
{
    let deserializer = DatexDeserializer::from_value(value);
    T::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::serde::serializer::to_bytes;
    use serde::{Deserialize, Serialize};

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

    #[test]
    fn test_from_script() {
        let script = r#"
            {
                field1: "Hello",
                field2: 42 + 5 // This will be evaluated to 47
            }
        "#;
        let deserializer = DatexDeserializer::from_script(script).unwrap();
        let result: TestStruct =
            Deserialize::deserialize(deserializer).unwrap();
        assert!(!result.field1.is_empty());
        println!("Deserialized from script: {result:?}");
    }

    // FIXME we are loosing the type information for integers here (i128 instead of i32 as in structure)
    // what causes a invalid type: integer error on the serde deserialization
    #[test]
    #[ignore = "This test is currently failing due to type mismatch (i128 instead of i32)"]
    fn test_from_static_script() {
        let script = r#"
            {
                field1: "Hello",
                field2: 42
            }
        "#;
        let deserializer =
            DatexDeserializer::from_static_script(script).unwrap();
        let result: TestStruct =
            Deserialize::deserialize(deserializer).unwrap();
        assert!(!result.field1.is_empty());
        println!("Deserialized from script: {result:?}");
    }
}
