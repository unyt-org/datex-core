use crate::values::core_values::map::{Map, OwnedMapKey};
use crate::values::value::Value;
use crate::{
    compiler::{
        CompileOptions, compile_script, extract_static_value_from_script,
    },
    runtime::execution::{ExecutionInput, ExecutionOptions, execute_dxb_sync},
    serde::error::DeserializationError,
    values::{
        core_value::CoreValue,
        core_values::{
            decimal::typed_decimal::TypedDecimal,
            integer::typed_integer::TypedInteger,
        },
        value,
        value_container::ValueContainer,
    },
};
use serde::de::{EnumAccess, VariantAccess, Visitor};
use serde::{Deserializer, de::IntoDeserializer, forward_to_deserialize_any};
use std::path::PathBuf;

/// Deserialize a value of type T from a byte slice containing DXB data
pub fn from_bytes<'de, T>(input: &'de [u8]) -> Result<T, DeserializationError>
where
    T: serde::Deserialize<'de>,
{
    let deserializer = DatexDeserializer::from_bytes(input)?;
    T::deserialize(deserializer)
}

/// Deserialize a value of type T from a ValueContainer
pub fn from_value_container<'de, T>(
    value: ValueContainer,
) -> Result<T, DeserializationError>
where
    T: serde::Deserialize<'de>,
{
    let deserializer = DatexDeserializer::from_value(value);
    T::deserialize(deserializer)
}

#[derive(Clone)]
pub struct DatexDeserializer {
    pub value: ValueContainer,
}

impl<'de> DatexDeserializer {
    /// Create a deserializer from a byte slice containing DXB data
    /// This will execute the DXB and extract the resulting value for deserialization
    pub fn from_bytes(input: &'de [u8]) -> Result<Self, DeserializationError> {
        let context = ExecutionInput::new_with_dxb_and_options(
            input,
            ExecutionOptions { verbose: true },
        );
        let value = execute_dxb_sync(context)
            .map_err(DeserializationError::ExecutionError)?
            .expect("DXB execution returned no value");
        Ok(Self { value })
    }

    /// Create a deserializer from a DX file path
    /// This will read the file, compile it to DXB, execute it and extract the
    pub fn from_dx_file(path: PathBuf) -> Result<Self, DeserializationError> {
        let input = std::fs::read_to_string(path).map_err(|err| {
            DeserializationError::CanNotReadFile(err.to_string())
        })?;
        DatexDeserializer::from_script(&input)
    }

    /// Create a deserializer from a DXB file path
    /// This will read the file, execute it and extract the resulting value for deserialization
    pub fn from_dxb_file(path: PathBuf) -> Result<Self, DeserializationError> {
        let input = std::fs::read(path).map_err(|err| {
            DeserializationError::CanNotReadFile(err.to_string())
        })?;
        DatexDeserializer::from_bytes(&input)
    }

    /// Create a deserializer from a DX script string
    /// This will compile the script to DXB, execute it and extract the resulting value for deserialization
    pub fn from_script(script: &'de str) -> Result<Self, DeserializationError> {
        let (dxb, _) = compile_script(script, CompileOptions::default())
            .map_err(|err| {
                DeserializationError::CanNotReadFile(err.to_string())
            })?;
        DatexDeserializer::from_bytes(&dxb)
    }

    /// Create a deserializer from a DX script string
    /// This will extract a static value from the script without executing it
    /// and use that value for deserialization
    /// If no static value is found, an error is returned
    /// This is useful for deserializing simple values like integer, text, map and list
    /// without the need to execute the script
    /// Note: This does not support expressions or computations in the script
    /// For example, the script `{ "key": 42 }` will work, but the script `{ "key": 40 + 2 }` will not
    /// because the latter requires execution to evaluate the expression
    /// and extract the value
    pub fn from_static_script(
        script: &'de str,
    ) -> Result<Self, DeserializationError> {
        let value = extract_static_value_from_script(script)
            .map_err(DeserializationError::CompilerError)?;
        if value.is_none() {
            return Err(DeserializationError::NoStaticValueFound);
        }
        Ok(DatexDeserializer::from_value(value.unwrap()))
    }

    fn from_value(value: ValueContainer) -> Self {
        Self { value }
    }
}

impl<'de> IntoDeserializer<'de, DeserializationError> for DatexDeserializer {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
impl<'de> Deserializer<'de> for DatexDeserializer {
    type Error = DeserializationError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        tuple seq unit struct ignored_any
    }

    /// Deserialize any value from the value container
    /// This is the main entry point for deserialization
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            // TODO #148 implement missing mapping
            ValueContainer::Value(value::Value { inner, .. }) => match inner {
                CoreValue::Null => visitor.visit_none(),
                CoreValue::Boolean(b) => visitor.visit_bool(b.0),
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
                    TypedInteger::Big(i) => {
                        visitor.visit_i128(i.as_i128().unwrap())
                    }
                    e => todo!("Unsupported typed integer: {:?}", e),
                },
                CoreValue::Integer(i) => {
                    if let Some(v) = i.as_i8() {
                        visitor.visit_i8(v)
                    } else if let Some(v) = i.as_i16() {
                        visitor.visit_i16(v)
                    } else if let Some(v) = i.as_i32() {
                        visitor.visit_i32(v)
                    } else if let Some(v) = i.as_i64() {
                        visitor.visit_i64(v)
                    } else {
                        visitor.visit_i128(i.as_i128().unwrap())
                    }
                }
                CoreValue::Decimal(d) => todo!("Unsupported decimal: {:?}", d),
                CoreValue::TypedDecimal(d) => match d {
                    TypedDecimal::F32(v) => visitor.visit_f32(v.0),
                    TypedDecimal::F64(v) => visitor.visit_f64(v.0),
                    TypedDecimal::Decimal(v) => {
                        visitor.visit_str(&v.to_string())
                    }
                },
                CoreValue::Text(s) => visitor.visit_string(s.0),
                CoreValue::Endpoint(endpoint) => {
                    let endpoint_str = endpoint.to_string();
                    visitor.visit_string(endpoint_str)
                }
                CoreValue::Map(obj) => {
                    let map = obj.into_iter().map(|(k, v)| {
                        (
                            DatexDeserializer::from_value(
                                ValueContainer::from(k),
                            ),
                            DatexDeserializer::from_value(v),
                        )
                    });
                    visitor
                        .visit_map(serde::de::value::MapDeserializer::new(map))
                }
                CoreValue::List(list) => {
                    let vec =
                        list.into_iter().map(DatexDeserializer::from_value);
                    visitor
                        .visit_seq(serde::de::value::SeqDeserializer::new(vec))
                }
                e => unreachable!("Unsupported core value: {:?}", e),
            },
            _ => unreachable!("Refs are not supported in deserialization"),
        }
    }

    /// Deserialize unit structs from the value container
    /// For example:
    ///     struct MyUnitStruct;
    /// will be deserialized from:
    ///     ()
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    /// Deserialize options from null or some value in the value container
    /// For example:
    ///     Some(42) will be deserialized from 42
    ///     None will be deserialized from null
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

    /// Deserialize newtype structs from single values or tuples in the value container
    /// For example:
    ///     struct MyNewtypeStruct(i32);
    /// will be deserialized from:
    ///     42
    /// or
    ///     (42,)
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // TODO: handle structurally typed maps and lists
        // if let ValueContainer::Value(Value {
        //     inner: CoreValue::Array(array),
        //     ..
        // }) = self.value
        // {
        //     let values = array.into_iter().map(DatexDeserializer::from_value);
        //     visitor.visit_seq(serde::de::value::SeqDeserializer::new(values))
        // } else if let ValueContainer::Value(Value {
        //     inner: CoreValue::Struct(structure),
        //     ..
        // }) = &self.value
        // {
        //     if structure.size() == 2 {
        //         let first_entry = structure.at_unchecked(0);
        //         if let ValueContainer::Value(Value {
        //             inner: CoreValue::Text(text),
        //             ..
        //         }) = first_entry
        //             && text.0.starts_with("datex::")
        //         {
        //             let second_entry = structure.at_unchecked(1);
        //             return visitor.visit_newtype_struct(
        //                 DatexDeserializer::from_value(second_entry.clone()),
        //             );
        //         }
        //     }
        //     visitor
        //         .visit_newtype_struct(DatexDeserializer::from_value(self.value))
        // } else {
        //
        // }

        visitor.visit_seq(serde::de::value::SeqDeserializer::new(
            vec![self.value.clone()]
                .into_iter()
                .map(DatexDeserializer::from_value),
        ))
    }

    /// Deserialize tuple structs from a list in the value container
    /// For example:
    ///     struct MyTupleStruct(i32, String);
    /// will be deserialized from:
    ///     [42, "Hello"]
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let ValueContainer::Value(Value {
            inner: CoreValue::List(list),
            ..
        }) = self.value
        {
            visitor.visit_seq(serde::de::value::SeqDeserializer::new(
                list.into_iter().map(DatexDeserializer::from_value),
            ))
        } else {
            Err(DeserializationError::Custom(
                "expected map for tuple struct".to_string(),
            ))
        }
    }

    /// Deserialize maps from tuples of key-value pairs
    /// For example:
    ///     ("key1": value1, "key2": value2)
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let ValueContainer::Value(Value {
            inner: CoreValue::Map(map),
            ..
        }) = self.value
        {
            let entries = map.into_iter().map(|(k, v)| {
                (
                    DatexDeserializer::from_value(ValueContainer::from(k)),
                    DatexDeserializer::from_value(v),
                )
            });
            visitor.visit_map(serde::de::value::MapDeserializer::new(entries))
        } else {
            Err(DeserializationError::Custom("expected map".to_string()))
        }
    }

    /// Deserialize identifiers from various formats:
    /// - Direct text: "identifier"
    /// - Single-key map: {"Identifier": ...}
    /// - Tuple with single text element: ("identifier", ...)
    fn deserialize_identifier<V>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            // Direct text
            ValueContainer::Value(Value {
                inner: CoreValue::Text(s),
                ..
            }) => visitor.visit_string(s.0),

            // Single-key map {"Identifier": ...}
            ValueContainer::Value(Value {
                inner: CoreValue::Map(o),
                ..
            }) => {
                if o.size() == 1 {
                    let (key, _) = o.into_iter().next().unwrap();
                    if let OwnedMapKey::Text(string) = key {
                        visitor.visit_string(string)
                    } else {
                        Err(DeserializationError::Custom(
                            "Expected text key for identifier".to_string(),
                        ))
                    }
                } else {
                    Err(DeserializationError::Custom(
                        "Expected single-key map for identifier".to_string(),
                    ))
                }
            }

            _ => Err(DeserializationError::Custom(
                "Expected identifier".to_string(),
            )),
        }
    }

    /// Deserialize enums from various formats:
    /// - Unit variants: "Variant"
    /// - Newtype variants: {"Variant": value}
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            // Default representation: ("Variant", value)
            ValueContainer::Value(Value {
                inner: CoreValue::List(t),
                ..
            }) => {
                if t.is_empty() {
                    return Err(DeserializationError::Custom(
                        "Expected non-empty tuple for enum".to_string(),
                    ));
                }
                let deserializer = DatexDeserializer::from_value(t.into());
                visitor.visit_enum(EnumDeserializer {
                    variant: "_tuple".to_string(),
                    value: deserializer,
                })
            }

            // Map with single key = variant name
            ValueContainer::Value(Value {
                inner: CoreValue::Map(o),
                ..
            }) => {
                if o.size() != 1 {
                    return Err(DeserializationError::Custom(
                        "Expected single-key map for enum".to_string(),
                    ));
                }

                let (variant_name, value) = o.into_iter().next().unwrap();
                if let OwnedMapKey::Text(variant) = variant_name {
                    let deserializer = DatexDeserializer::from_value(value);
                    visitor.visit_enum(EnumDeserializer {
                        variant,
                        value: deserializer,
                    })
                } else {
                    Err(DeserializationError::Custom(
                        "Expected text variant name".to_string(),
                    ))
                }
            }
            // TODO: handle structurally typed maps
            // ValueContainer::Value(Value {
            //     inner: CoreValue::Struct(o),
            //     ..
            // }) => {
            //     if o.size() != 1 {
            //         return Err(DeserializationError::Custom(
            //             "Expected single-key object for enum".to_string(),
            //         ));
            //     }
            //
            //     let (variant_name, value) = o.into_iter().next().unwrap();
            //
            //     let deserializer = DatexDeserializer::from_value(value);
            //     visitor.visit_enum(EnumDeserializer {
            //         variant: variant_name,
            //         value: deserializer,
            //     })
            // }

            // unit variants stored directly as text
            ValueContainer::Value(Value {
                inner: CoreValue::Text(s),
                ..
            }) => visitor.visit_enum(EnumDeserializer {
                variant: s.0,
                value: DatexDeserializer::from_value(Map::default().into()),
            }),

            e => Err(DeserializationError::Custom(format!(
                "Expected enum representation, found: {}",
                e
            ))),
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Enum deserializer helper
/// Used to deserialize enum variants
/// For example:
///     enum MyEnum {
///         Variant1,
///         Variant2(i32),
///     }
/// will be deserialized from:
///     "Variant1" or {"Variant2": 42}
struct EnumDeserializer {
    variant: String,
    value: DatexDeserializer,
}
impl<'de> EnumAccess<'de> for EnumDeserializer {
    type Error = DeserializationError;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(DatexDeserializer::from_value(
            ValueContainer::from(self.variant),
        ))?;
        Ok((variant, VariantDeserializer { value: self.value }))
    }
}

/// Variant deserializer helper
/// Used to deserialize enum variant contents
/// For example:
///     enum MyEnum {
///         Variant1,
///         Variant2(i32),
///     }
/// will be deserialized from:
///     "Variant1" or {"Variant2": 42}
struct VariantDeserializer {
    value: DatexDeserializer,
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = DeserializationError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.value.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.value.deserialize_struct("", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::serializer::to_bytes;
    use crate::{logger::init_logger, values::core_values::endpoint::Endpoint};
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
    fn nested_struct_serde() {
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

    // WIP
    #[test]
    fn struct_from_bytes() {
        let data = to_bytes(&TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        })
        .unwrap();
        let result: TestStruct = from_bytes(&data).unwrap();
        assert!(!result.field1.is_empty());
    }

    #[test]
    fn from_script() {
        init_logger();
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

    #[test]
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
    fn enum_1() {
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
    fn enum_2() {
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
    fn struct_with_enum() {
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
    fn endpoint() {
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
    fn optional_field() {
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
    fn optional_field_empty() {
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
    fn optional_endpoint() {
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
    fn map() {
        let script = "{Variant1: \"Hello\"}";
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: ExampleEnum = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize ExampleEnum");
        assert!(matches!(result, ExampleEnum::Variant1(_)));

        let script = r#"{"Variant2": 42}"#;
        let dxb = compile_script(script, CompileOptions::default())
            .expect("Failed to compile script")
            .0;
        let deserializer = DatexDeserializer::from_bytes(&dxb)
            .expect("Failed to create deserializer from DXB");
        let result: ExampleEnum = Deserialize::deserialize(deserializer)
            .expect("Failed to deserialize ExampleEnum");
        assert!(matches!(result, ExampleEnum::Variant2(_)));
    }
}
