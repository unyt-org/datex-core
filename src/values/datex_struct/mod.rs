use core::fmt;
use serde::ser::StdError;
use serde::ser::{self, Serialize, SerializeStruct, Serializer};
use std::fmt::Display;
use std::{error::Error, io};
#[derive(Debug)]
pub struct SerializationError(String);
impl ser::Error for SerializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerializationError(msg.to_string())
    }
}
impl From<io::Error> for SerializationError {
    fn from(e: io::Error) -> Self {
        SerializationError(e.to_string())
    }
}
impl StdError for SerializationError {}
impl Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SerializationError: {}", self.0)
    }
}

struct DatexSerializer {
    output: Vec<u8>,
}

impl DatexSerializer {
    pub fn new() -> Self {
        DatexSerializer { output: Vec::new() }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.output
    }
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>, SerializationError>
where
    T: Serialize,
{
    let mut serializer = DatexSerializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

impl<'a> Serializer for &'a mut DatexSerializer {
    type Ok = &'a [u8];
    type Error = SerializationError;

    type SerializeSeq = serde::ser::Impossible<Self::Ok, SerializationError>;

    type SerializeTuple = serde::ser::Impossible<Self::Ok, SerializationError>;

    type SerializeTupleStruct =
        serde::ser::Impossible<Self::Ok, SerializationError>;

    type SerializeTupleVariant =
        serde::ser::Impossible<Self::Ok, SerializationError>;

    type SerializeMap = serde::ser::Impossible<Self::Ok, SerializationError>;

    type SerializeStruct = serde::ser::Impossible<Self::Ok, SerializationError>;

    type SerializeStructVariant =
        serde::ser::Impossible<Self::Ok, SerializationError>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!()
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

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
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
}

#[cfg(test)]
mod tests {
    use crate::{macros::DatexStruct, values::datex_struct::DatexSerializer};
    use serde::Serialize;
    #[derive(Serialize, DatexStruct)]
    struct TestStruct {
        field1: String,
        field2: i32,
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
