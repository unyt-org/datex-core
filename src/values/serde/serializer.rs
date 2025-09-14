use crate::compiler::compile_value;
use crate::runtime::execution::{
    ExecutionInput, ExecutionOptions, execute_dxb_sync,
};
use crate::values::core_value::CoreValue;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::core_values::r#struct::Struct;
use crate::values::serde::error::SerializationError;
use crate::values::value_container::ValueContainer;
use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct,
    SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
    SerializeTupleVariant, Serializer,
};
use std::vec;
pub struct DatexSerializer {}

impl Default for DatexSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl DatexSerializer {
    pub fn new() -> Self {
        DatexSerializer {}
    }
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>, SerializationError>
where
    T: Serialize,
{
    let value_container = to_value_container(value)?;
    // println!("Value container: {value_container}");
    compile_value(&value_container).map_err(|e| e.into())
}
pub fn to_value_container<T>(
    value: &T,
) -> Result<ValueContainer, SerializationError>
where
    T: Serialize,
{
    let mut serializer = DatexSerializer::new();
    let container = value.serialize(&mut serializer)?;
    Ok(container)
}

/// Serializer for structs
/// For example:
/// struct MyStruct {
///     field1: String,
///     field2: i32,
/// }
/// will be serialized as:
/// {"field1": String, "field2": i32}
#[derive(Default)]
pub struct StructSerializer {
    fields: Vec<(String, ValueContainer)>,
}
impl StructSerializer {
    pub fn new() -> Self {
        Self::default()
    }
}
impl SerializeStruct for StructSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let vc = value.serialize(&mut DatexSerializer::new())?;
        self.fields.push((key.to_string(), vc));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // rational: we want to map to json representation
        // so other JSON serde still works. Otherwise we'd
        // use a Struct here (see setup data transfer from JS)
        // let mut map = Map::default();
        // for (field, value) in self.fields.into_iter() {
        //     map.set(ValueContainer::from(field), value);
        // }
        // Ok(ValueContainer::from(CoreValue::Map(map)))
        Ok(Struct::new(self.fields).into())
    }
}

/// Serializer for tuples
/// For example:
/// (i32, String)
/// will be serialized as:
/// (i32, String)
#[derive(Default)]
pub struct TupleSerializer {
    elements: Vec<ValueContainer>,
}
impl TupleSerializer {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }
}
impl SerializeTuple for TupleSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_element<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let vc = value.serialize(&mut DatexSerializer::new())?;
        self.elements.push(vc);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut list = List::default();
        for element in self.elements.into_iter() {
            list.push(element);
        }
        Ok(ValueContainer::from(CoreValue::List(list)))
    }
}

/// Serializer for tuple structs
/// For example:
/// struct MyStruct(i32, String);
/// will be serialized as:
/// {"MyStruct": [i32, String]}
pub struct TupleStructSerializer {
    name: &'static str,
    fields: List,
}
impl TupleStructSerializer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            fields: List::default(),
        }
    }
}
impl SerializeTupleStruct for TupleStructSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let field = value.serialize(&mut DatexSerializer::new())?;
        self.fields.push(field);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(self.fields))
    }
}

/// Serializer for enum variants with tuple fields
/// For example:
/// enum MyEnum {
///     Variant1(i32, String),
///     Variant2(bool),
/// }
/// will be serialized as:
/// {"Variant1": [i32, String]}
pub struct TupleVariantSerializer {
    variant: &'static str,
    fields: List,
}
impl TupleVariantSerializer {
    pub fn new(variant: &'static str) -> Self {
        Self {
            variant,
            fields: List::default(),
        }
    }
}
impl SerializeTupleVariant for TupleVariantSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let field = value.serialize(&mut DatexSerializer::new())?;
        self.fields.push(field);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(CoreValue::Struct(Struct::from(vec![
            (self.variant.to_string(), self.fields),
        ]))))
    }
}

/// Serializer for enum variants with struct fields
/// For example:
/// enum MyEnum {
///     Variant1 { x: i32, y: String },
///     Variant2 { a: bool },
/// }
/// will be serialized as:
/// {"Variant1": {"x": i32, "y": String}}
pub struct StructVariantSerializer {
    variant: &'static str,
    fields: Vec<(String, ValueContainer)>,
}
impl StructVariantSerializer {
    pub fn new(variant: &'static str) -> Self {
        Self {
            variant,
            fields: Vec::new(),
        }
    }
}
impl SerializeStructVariant for StructVariantSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let field = value.serialize(&mut DatexSerializer::new())?;
        self.fields.push((key.to_string(), field));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Struct::from(vec![(
            self.variant.to_string(),
            Struct::new(self.fields),
        )])
        .into())
    }
}

/// Serializer for sequences
/// For example:
/// vec![1, 2, 3]
/// will be serialized as:
/// [1, 2, 3]
#[derive(Default)]
pub struct SeqSerializer {
    elements: List,
}
impl SeqSerializer {
    pub fn new() -> Self {
        Self {
            elements: List::default(),
        }
    }
}
impl SerializeSeq for SeqSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_element<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let vc = value.serialize(&mut DatexSerializer::new())?;
        self.elements.push(vc);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(self.elements))
    }
}

/// Serializer for maps
/// For example:
///     HashMap<String, i32>
/// will be serialized as:
///     {"key": 1, "key2": 2, "key3": 3}
#[derive(Default)]
pub struct MapSerializer {
    entries: Vec<(ValueContainer, Option<ValueContainer>)>,
}
impl MapSerializer {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl SerializeMap for MapSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = key.serialize(&mut DatexSerializer::new())?;
        self.entries.push((key, None));
        Ok(())
    }

    fn serialize_value<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let vc = value.serialize(&mut DatexSerializer::new())?;
        if let Some(last) = self.entries.last_mut() {
            last.1 = Some(vc);
            Ok(())
        } else {
            Err(SerializationError::Custom(
                "serialize_value called before serialize_key".to_string(),
            ))
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = Map::default();
        for (key, value) in self.entries.iter() {
            if let Some(value) = value {
                map.set(key.clone(), value.clone());
            } else {
                return Err(SerializationError::Custom(
                    "Map entry without value".to_string(),
                ));
            }
        }
        Ok(map.into())
    }
}

/// Main serializer implementation
impl Serializer for &mut DatexSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    type SerializeStruct = StructSerializer;
    type SerializeTuple = TupleSerializer;
    type SerializeTupleStruct = TupleStructSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeStructVariant = StructVariantSerializer;
    type SerializeSeq = SeqSerializer;
    type SerializeMap = MapSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!("#134 Undescribed by author.")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(CoreValue::Null.into())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut *self).map_err(|e| {
            SerializationError::CanNotSerialize(format!(
                "Failed to serialize Some value: {e}"
            ))
        })
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Struct::default().into())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer::new())
    }

    fn serialize_unit_struct(
        self,
        name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Struct::default().into())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(variant))
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        if name == "datex::endpoint" {
            let endpoint = value
                .serialize(&mut *self)?
                .to_value()
                .borrow()
                .cast_to_endpoint()
                .unwrap();
            Ok(ValueContainer::from(endpoint))
        } else if name == "datex::value" {
            // unsafe cast value to ValueContainer
            let bytes = unsafe { &*(value as *const T as *const Vec<u8>) };
            Ok(execute_dxb_sync(ExecutionInput::new_with_dxb_and_options(
                bytes,
                ExecutionOptions::default(),
            ))
            .unwrap()
            .unwrap())
        } else if name.starts_with("datex::") {
            // Serialize internal new type structs as normal structs
            // {"datex::field": value}
            // instead of
            // value
            let mut a = StructSerializer::new();
            a.serialize_field(name, value)?;
            a.end()
        } else {
            Ok(value.serialize(&mut *self)?)
        }
    }

    /// Serialize newtype enum variants as structs with one field
    /// For example:
    /// enum MyEnum {
    ///     Variant1(String),
    ///     Variant2(i32, String),
    /// }
    /// is serialized as
    /// {"Variant2": [100, "hello"]}
    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let field = value.serialize(&mut *self)?;
        Ok(ValueContainer::from(CoreValue::Struct(Struct::from(vec![
            (variant.to_string(), field),
        ]))))
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new())
    }

    fn serialize_tuple(
        self,
        len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::new())
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructSerializer::new(name))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer::new(variant))
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new())
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer::new(variant))
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_structural_eq;
    use crate::values::core_values::endpoint::Endpoint;
    use crate::values::core_values::r#struct::Struct;
    use crate::values::traits::structural_eq::StructuralEq;
    use crate::values::{
        core_value::CoreValue,
        serde::serializer::{DatexSerializer, to_bytes, to_value_container},
        value::Value,
        value_container::ValueContainer,
    };
    use serde::{Deserialize, Serialize};
    use std::assert_matches::assert_matches;

    #[derive(Serialize)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[derive(Serialize)]
    struct TestTupleStruct(String, i32);

    #[derive(Serialize)]
    struct UnitStruct;

    #[derive(Serialize)]
    enum TestEnum {
        Unit,
        Tuple(i32, String),
        Struct { x: bool, y: f64 },
    }

    #[derive(Serialize)]
    struct NestedStruct {
        nested: TestStruct,
        value: i32,
    }

    #[derive(Serialize)]
    struct StructWithOption {
        value: Option<i32>,
    }

    #[derive(Serialize)]
    struct StructWithVec {
        values: Vec<i32>,
    }

    #[derive(Serialize)]
    struct TestStructWithEndpoint {
        endpoint: Endpoint,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct StructWithUSize {
        pub usize: Option<usize>,
    }

    #[test]
    fn datex_serializer() {
        let mut serializer = DatexSerializer::new();
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let value_container = test_struct.serialize(&mut serializer).unwrap();
        assert_matches!(
            value_container,
            ValueContainer::Value(Value {
                inner: CoreValue::Struct(_),
                ..
            })
        );
    }

    #[test]
    fn r#struct() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let result = to_value_container(&test_struct).unwrap();
        assert_eq!(result.to_string(), r#"{"field1": "Hello", "field2": 42}"#);
    }

    #[test]
    fn tuple_struct() {
        let ts = TestTupleStruct("hi".to_string(), 99);
        let result = to_value_container(&ts).unwrap();
        assert_eq!(result.to_string(), r#"["hi", 99]"#);
    }

    #[test]
    fn unit_struct() {
        let us = UnitStruct;
        let result = to_value_container(&us).unwrap();
        assert_eq!(result.to_string(), r#"{}"#);
    }

    #[test]
    fn enum_unit_variant() {
        let e = TestEnum::Unit;
        let result = to_value_container(&e).unwrap();
        assert_eq!(result.to_string(), r#""Unit""#);
    }

    #[test]
    fn enum_tuple_variant() {
        let e = TestEnum::Tuple(42, "hello".to_string());
        let result = to_value_container(&e).unwrap();
        assert_eq!(result.to_string(), r#"{"Tuple": [42, "hello"]}"#);
    }

    #[test]
    fn enum_struct_variant() {
        let e = TestEnum::Struct { x: true, y: 3.5 };
        let result = to_value_container(&e).unwrap();
        assert_eq!(result.to_string(), r#"{"Struct": {"x": true, "y": 3.5}}"#);
    }

    #[test]
    fn vec() {
        let data = vec![10, 20, 30];
        let result = to_value_container(&data).unwrap();
        assert_eq!(result.to_string(), "[10, 20, 30]");
    }

    #[test]
    fn tuple_array() {
        let data = [1, 2, 3, 4];
        let result = to_value_container(&data).unwrap();
        assert_eq!(result.to_string(), "[1, 2, 3, 4]");
    }

    #[test]
    fn nested_struct() {
        let nested = NestedStruct {
            nested: TestStruct {
                field1: "A".to_string(),
                field2: 1,
            },
            value: 99,
        };
        let result = to_value_container(&nested).unwrap();
        assert_eq!(
            result.to_string(),
            r#"{"nested": {"field1": "A", "field2": 1}, "value": 99}"#
        );
    }

    #[test]
    fn struct_with_option_some() {
        let s = StructWithOption { value: Some(42) };
        let result = to_value_container(&s).unwrap();
        assert_eq!(result.to_string(), r#"{"value": 42}"#);
    }

    #[test]
    fn struct_with_option_none() {
        let s = StructWithOption { value: None };
        let result = to_value_container(&s).unwrap();
        // None can serialize as null
        assert_eq!(result.to_string(), r#"{"value": null}"#);
    }

    #[test]
    fn struct_with_vec() {
        let s = StructWithVec {
            values: vec![1, 2, 3],
        };
        let result = to_value_container(&s).unwrap();
        assert_eq!(result.to_string(), r#"{"values": [1, 2, 3]}"#);
    }

    #[test]
    fn primitive_values() {
        // integer
        let i = 42;
        let vc = to_value_container(&i).unwrap();
        assert_eq!(vc.to_string(), "42");

        // float
        let f = 3.4;
        let vc = to_value_container(&f).unwrap();
        assert_eq!(vc.to_string(), "3.4");

        // boolean
        let b = true;
        let vc = to_value_container(&b).unwrap();
        assert_eq!(vc.to_string(), "true");

        // string
        let s = "test";
        let vc = to_value_container(&s).unwrap();
        assert_eq!(vc.to_string(), r#""test""#);
    }

    #[test]
    fn array_serialization() {
        let arr = vec![1, 2, 3];
        let vc = to_value_container(&arr).unwrap();
        assert_eq!(vc.to_string(), "[1, 2, 3]");
    }

    #[test]
    fn serializer_into_inner_object() {
        let mut serializer = DatexSerializer::new();
        let s = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let value_container = s.serialize(&mut serializer).unwrap();
        assert_matches!(
            value_container,
            ValueContainer::Value(Value {
                inner: CoreValue::Struct(_),
                ..
            })
        );
    }

    #[test]
    fn struct_to_bytes() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let result = to_bytes(&test_struct);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn to_bytes_with_struct_with_usize() {
        let test_struct = StructWithUSize { usize: Some(42) };
        let result = to_value_container(&test_struct);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_structural_eq!(
            result
                .to_value()
                .borrow()
                .cast_to_struct()
                .unwrap()
                .get("usize")
                .unwrap(),
            ValueContainer::from(42)
        );
    }

    #[test]
    fn endpoint() {
        let script = "@test";
        let result = to_value_container(&script).unwrap();
        assert_eq!(result.to_string(), "\"@test\"");

        let test_struct = TestStructWithEndpoint {
            endpoint: Endpoint::new("@test"),
        };

        let result = to_value_container(&test_struct);
        assert!(result.is_ok());
        let result = result.unwrap();
        let r#struct = Struct::from(vec![(
            "endpoint".to_string(),
            ValueContainer::from(Endpoint::new("@test")),
        )]);
        assert_eq!(result, ValueContainer::from(r#struct));
    }

    #[derive(Serialize)]
    struct MyNewtype(i32);

    #[test]
    fn newtype_struct() {
        let my_newtype = MyNewtype(100);
        let result = to_value_container(&my_newtype).unwrap();
        assert_eq!(result.to_string(), r#"100"#);
    }

    #[derive(Serialize)]
    struct StructType(i32, String, bool);
    #[test]
    fn newtype_struct_multiple_fields() {
        let s = StructType(1, "test".to_string(), true);
        let result = to_value_container(&s).unwrap();
        assert_eq!(result.to_string(), r#"[1, "test", true]"#);
    }

    #[derive(Serialize)]
    enum MyTaggedEnum {
        Variant1 { x: i32, y: String },
        Variant2(i32, String),
        Empty,
    }

    #[test]
    fn tagged_enum() {
        let e = MyTaggedEnum::Variant1 {
            x: 42,
            y: "test".to_string(),
        };
        let result = to_value_container(&e).unwrap();
        assert_eq!(
            result.to_string(),
            r#"{"Variant1": {"x": 42, "y": "test"}}"#
        );

        let e = MyTaggedEnum::Variant2(100, "hello".to_string());
        let result = to_value_container(&e).unwrap();
        assert_eq!(result.to_string(), r#"{"Variant2": [100, "hello"]}"#);

        let e = MyTaggedEnum::Empty;
        let result = to_value_container(&e).unwrap();
        assert_eq!(result.to_string(), r#""Empty""#);
    }
}
