use crate::values::core_values::tuple::Tuple;
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
use chumsky::prelude::todo;
use datex_core::values::core_values::endpoint::Endpoint;
use log::info;
use serde::de::value::MapDeserializer;
use serde::de::{DeserializeSeed, EnumAccess, VariantAccess, Visitor};
use serde::{
    Deserialize, Deserializer, de::IntoDeserializer, forward_to_deserialize_any,
};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone)]
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
                CoreValue::Null => visitor.visit_none(),
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
                CoreValue::Endpoint(endpoint) => {
                    let endpoint_str = endpoint.to_string();
                    visitor.visit_string(endpoint_str)
                }
                CoreValue::Object(obj) => {
                    let map = obj
                        .into_iter()
                        .map(|(k, v)| (k, DatexDeserializer::from_value(v)));
                    visitor
                        .visit_map(serde::de::value::MapDeserializer::new(map))
                }
                CoreValue::Array(arr) => {
                    let vec =
                        arr.into_iter().map(DatexDeserializer::from_value);
                    visitor
                        .visit_seq(serde::de::value::SeqDeserializer::new(vec))
                }
                CoreValue::Tuple(tuple) => {
                    let vec = tuple
                        .into_iter()
                        .map(|(_, v)| DatexDeserializer::from_value(v));
                    visitor
                        .visit_seq(serde::de::value::SeqDeserializer::new(vec))
                }
                e => unreachable!("Unsupported core value: {:?}", e),
            },
            _ => unreachable!("Refs are not supported in deserialization"),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.value.to_value().borrow().is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        tuple seq unit unit_struct
         ignored_any
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("Deserializing struct: {} ({})", name, fields.join(", "));
        if let ValueContainer::Value(value::Value {
            inner: CoreValue::Object(t),
            ..
        }) = &self.value
        {
            println!("Object: {:?}", t.0.keys());
            let values = t
                .into_iter()
                .map(|(s, v)| {
                    (s.clone(), DatexDeserializer::from_value(v.clone()))
                })
                .collect::<HashMap<_, _>>();

            // Provide it to Serde as a map
            let map = MapDeserializer::new(values.into_iter());
            visitor.visit_map(map)
            // let map: HashMap<String, DatexDeserializer> = t
            //     .iter()
            //     .map(|(k, v)| {
            //         (k.clone(), DatexDeserializer::from_value(v.clone()))
            //     })
            //     .collect();

            // println!("map: {:?}", map.keys());

            // let deserializer = MapDeserializer::new(map.into_iter());
            // visitor.visit_map(deserializer)
        } else {
            // println!("Fields: {:?} ---> {}", fields, t);
            // self.deserialize_newtype_struct(name, visitor)
            // self.deserialize_struct(name, fields, visitor)
            // unreachable!("Deserialization of structs is not implemented yet");
            // visitor.visit_seq(serde::de::value::SeqDeserializer::new(
            //     vec![self.value.clone()]
            //         .into_iter()
            //         .map(DatexDeserializer::from_value),
            // ))
            unreachable!("Deserialization of structs is not implemented yet");

            // self.deserialize_any(visitor)
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let ValueContainer::Value(value::Value {
            inner: CoreValue::Tuple(t),
            ..
        }) = self.value
        {
            let values =
                t.into_iter().map(|(_, v)| DatexDeserializer::from_value(v));
            visitor.visit_seq(serde::de::value::SeqDeserializer::new(values))
        } else {
            visitor.visit_seq(serde::de::value::SeqDeserializer::new(
                vec![self.value.clone()]
                    .into_iter()
                    .map(DatexDeserializer::from_value),
            ))
        }
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let ValueContainer::Value(value::Value {
            inner: CoreValue::Tuple(t),
            ..
        }) = self.value
        {
            let values =
                t.into_iter().map(|(_, v)| DatexDeserializer::from_value(v));
            visitor.visit_seq(serde::de::value::SeqDeserializer::new(values))
        } else {
            visitor.visit_seq(serde::de::value::SeqDeserializer::new(
                vec![self.value.clone()]
                    .into_iter()
                    .map(DatexDeserializer::from_value),
            ))
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!("map")
    }
    fn deserialize_identifier<V>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("Deserializing identifier: {:?}", self.value);
        // match tuple (Identifier, ValueContainer)
        if let ValueContainer::Value(value::Value {
            inner: CoreValue::Tuple(t),
            ..
        }) = self.value
        {
            let identifier = t
                .at(0)
                .ok_or(SerializationError("Invalid tuple".to_string()))?
                .1;
            visitor
                .visit_string(identifier.to_value().borrow().cast_to_text().0)
        }
        // match string
        else if let ValueContainer::Value(value::Value {
            inner: CoreValue::Text(s),
            ..
        }) = self.value
        {
            visitor.visit_string(s.0)
        } else {
            Err(SerializationError("Expected identifier tuple".to_string()))
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        println!("Deserializing enum: {:?}", self.value);
        visitor.visit_enum(DatexEnumAccess { de: self })
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct DatexEnumAccess {
    de: DatexDeserializer,
}

impl<'de> EnumAccess<'de> for DatexEnumAccess {
    type Error = SerializationError;
    type Variant = DatexVariantAccess;

    fn variant_seed<V>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.de.clone())?;
        Ok((variant, DatexVariantAccess { de: self.de }))
    }
}

struct DatexVariantAccess {
    de: DatexDeserializer,
}
impl<'de> VariantAccess<'de> for DatexVariantAccess {
    type Error = SerializationError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(
        mut self,
        seed: T,
    ) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if let ValueContainer::Value(value::Value {
            inner: CoreValue::Tuple(t),
            ..
        }) = self.de.value
        {
            let value = t
                .at(1)
                .ok_or(SerializationError("Invalid tuple".to_string()))?
                .1;
            self.de.value = value.clone();
            Ok(seed.deserialize(self.de)?)
        } else {
            Err(SerializationError("Expected identifier tuple".to_string()))
        }
    }

    fn tuple_variant<V>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
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

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[derive(Deserialize, Serialize, Debug)]
    enum TestEnum {
        Variant1,
        Variant2,
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct TestStruct2 {
        test_enum: TestEnum,
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct TestWithOptionalField {
        optional_field: Option<String>,
    }

    #[derive(Deserialize)]
    struct TestStructWithEndpoint {
        endpoint: Endpoint,
    }

    #[derive(Deserialize)]
    struct TestStructWithOptionalEndpoint {
        endpoint: Option<Endpoint>,
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct TestNestedStruct {
        nested: TestStruct,
    }

    #[test]
    fn test_nested_struct_serde() {
        let script = r#"
            {
                nested: {
                    field1: "Hello",
                    field2: 47
                }
            }
        "#;
        let deserializer = DatexDeserializer::from_script(script).unwrap();
        let result: TestNestedStruct =
            Deserialize::deserialize(deserializer).unwrap();
        assert_eq!(
            result,
            TestNestedStruct {
                nested: TestStruct {
                    field1: "Hello".to_string(),
                    field2: 47
                }
            }
        );
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
    }

    #[test]
    fn test_enum_1() {
        let script = r#""Variant1""#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestEnum = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize TestEnum");
        assert!(matches!(result, TestEnum::Variant1));
    }

    #[test]
    fn test_enum_2() {
        let script = r#""Variant2""#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestEnum = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize TestEnum");
        assert!(matches!(result, TestEnum::Variant2));
    }

    #[test]
    fn test_struct_with_enum() {
        let script = r#"
            {
                test_enum: "Variant1"
            }
        "#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestStruct2 = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize TestStruct2");
        assert!(matches!(result.test_enum, TestEnum::Variant1));
    }

    #[test]
    fn test_endpoint() {
        let script = r#"
            {
                endpoint: @jonas
            }
        "#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestStructWithEndpoint =
            Deserialize::deserialize(deserializer)
                .expect("Failed to deserialize TestStructWithEndpoint");
        assert_eq!(result.endpoint.to_string(), "@jonas");
    }

    #[test]
    fn test_optional_field() {
        let script = r#"
            {
                optional_field: "Optional Value"
            }
        "#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestWithOptionalField =
            Deserialize::deserialize(deserializer)
                .expect("Failed to deserialize TestWithOptionalField");
        assert!(result.optional_field.is_some());
        assert_eq!(result.optional_field.unwrap(), "Optional Value");
    }

    #[test]
    fn test_optional_field_empty() {
        let script = r#"
            {
                optional_field: null
            }
        "#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestWithOptionalField =
            Deserialize::deserialize(deserializer)
                .expect("Failed to deserialize TestWithOptionalField");
        assert!(result.optional_field.is_none());
    }

    #[test]
    fn test_optional_endpoint() {
        let script = r#"
            {
                endpoint: @jonas
            }
        "#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: TestStructWithOptionalEndpoint =
            Deserialize::deserialize(deserializer)
                .expect("Failed to deserialize TestStructWithOptionalEndpoint");
        assert!(result.endpoint.is_some());
        assert_eq!(result.endpoint.unwrap().to_string(), "@jonas");
    }

    #[derive(Deserialize, Serialize, Debug)]
    enum ExampleEnum {
        Variant1(String),
        Variant2(i32),
    }

    #[test]
    fn test_map() {
        let script = r#"("Variant1", "xy")"#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: ExampleEnum = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize ExampleEnum");
        match result {
            ExampleEnum::Variant1(s) => assert_eq!(s, "xy"),
            _ => panic!("Expected Variant1 with value 'xy'"),
        }

        let script = r#"("Variant2", 42)"#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: ExampleEnum = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize ExampleEnum");
        match result {
            ExampleEnum::Variant2(i) => assert_eq!(i, 42),
            _ => panic!("Expected Variant2 with value 42"),
        }
    }
}
