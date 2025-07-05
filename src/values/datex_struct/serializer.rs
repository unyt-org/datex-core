use core::fmt;
use serde::ser::StdError;
use serde::ser::{self, Serialize, SerializeStruct, Serializer};
use std::fmt::Display;
use std::{error::Error, io};

use crate::compiler::compile_value;
use crate::values::core_value::CoreValue;
use crate::values::core_values::object::Object;
use crate::values::datex_struct::error::SerializationError;
use crate::values::value;
use crate::values::value_container::ValueContainer;
pub struct DatexSerializer {
    object: Object,
}

impl DatexSerializer {
    pub fn new() -> Self {
        DatexSerializer {
            object: Object::new(),
        }
    }

    pub fn into_inner(self) -> Object {
        self.object
    }
    pub fn into_value_container(self) -> ValueContainer {
        ValueContainer::from(self.object)
    }
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>, SerializationError>
where
    T: Serialize,
{
    let value_container = to_value_container(value)?;
    Ok(compile_value(&value_container).map_err(|e| {
        SerializationError(format!("Failed to compile value: {}", e))
    })?)
}
pub fn to_value_container<T>(
    value: &T,
) -> Result<ValueContainer, SerializationError>
where
    T: Serialize,
{
    let mut serializer = DatexSerializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_value_container())
}

impl<'a> SerializeStruct for &'a mut DatexSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value_container = value.serialize(&mut **self)?;
        self.object.set(key, value_container);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(self.object.clone()))
    }
}

impl<'a> Serializer for &'a mut DatexSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    type SerializeSeq = serde::ser::Impossible<Self::Ok, Self::Error>;

    type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;

    type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;

    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;

    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    // Should be Self
    type SerializeStruct = Self;
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

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
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(CoreValue::Null.into())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_struct(
        self,
        name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

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
        todo!()
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        todo!()
    }

    fn serialize_tuple(
        self,
        len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        let _ = v;
        Err(ser::Error::custom("i128 is not supported"))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        let _ = v;
        Err(ser::Error::custom("u128 is not supported"))
    }

    fn collect_seq<I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Serialize,
    {
        todo!()
    }

    fn collect_map<K, V, I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        K: Serialize,
        V: Serialize,
        I: IntoIterator<Item = (K, V)>,
    {
        todo!()
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Display,
    {
        self.serialize_str(&value.to_string())
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::values::datex_struct::serializer::{
        DatexSerializer, to_bytes, to_value_container,
    };
    use serde::Serialize;
    #[derive(Serialize)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_to_value_container() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let result = to_value_container(&test_struct);
        assert!(result.is_ok());
        println!("{:?}", result.unwrap());
    }

    #[test]
    fn test_to_bytes() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let result = to_bytes(&test_struct);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_datex_serializer() {
        let mut serializer = DatexSerializer::new();
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let _ = test_struct.serialize(&mut serializer);
        let result = serializer.into_inner();
        assert!(!result.is_empty());
    }
}
