use serde::ser::Error as SerError;

#[derive(Default)]
pub struct ByteSerializer {
    out: Vec<u8>,
}
use std::fmt;

pub enum TestError {
    Other,
}

impl fmt::Debug for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestError")
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestError")
    }
}

impl std::error::Error for TestError {}

impl SerError for TestError {
    fn custom<T: std::fmt::Display>(_msg: T) -> Self {
        TestError::Other
    }
}
// TODO
// impl ByteSerializer {
//     pub fn into_inner(self) -> Vec<u8> {
//         self.out
//     }
// }
// impl<'a> ser::Serializer for &'a mut ByteSerializer {
//     type Ok = Vec<u8>;
//     type Error = TestError;

//     type SerializeSeq = Self;
//     type SerializeTuple = Self;
//     type SerializeTupleStruct = Self;
//     type SerializeTupleVariant = Self;
//     type SerializeMap = Self;
//     type SerializeStruct = Self;
//     type SerializeStructVariant = Self;

//     fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
//     where
//         T: ?Sized + Serialize,
//     {
//         todo!()
//     }

//     fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_unit_struct(
//         self,
//         name: &'static str,
//     ) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_unit_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//     ) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_newtype_struct<T>(
//         self,
//         name: &'static str,
//         value: &T,
//     ) -> Result<Self::Ok, Self::Error>
//     where
//         T: ?Sized + Serialize,
//     {
//         todo!()
//     }

//     fn serialize_newtype_variant<T>(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         value: &T,
//     ) -> Result<Self::Ok, Self::Error>
//     where
//         T: ?Sized + Serialize,
//     {
//         todo!()
//     }

//     fn serialize_seq(
//         self,
//         len: Option<usize>,
//     ) -> Result<Self::SerializeSeq, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple(
//         self,
//         len: usize,
//     ) -> Result<Self::SerializeTuple, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleStruct, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleVariant, Self::Error> {
//         todo!()
//     }

//     fn serialize_map(
//         self,
//         len: Option<usize>,
//     ) -> Result<Self::SerializeMap, Self::Error> {
//         todo!()
//     }

//     fn serialize_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStruct, Self::Error> {
//         todo!()
//     }

//     fn serialize_struct_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStructVariant, Self::Error> {
//         todo!()
//     }

//     fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
//         let _ = v;
//         Err(SerError::custom("i128 is not supported"))
//     }

//     fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
//         let _ = v;
//         Err(SerError::custom("u128 is not supported"))
//     }

//     fn collect_seq<I>(self, iter: I) -> Result<Self::Ok, Self::Error>
//     where
//         I: IntoIterator,
//         <I as IntoIterator>::Item: Serialize,
//     {
//         let mut iter = std::iter.into_iter();
//         let mut serializer = tri!(self.serialize_seq(iterator_len_hint(&iter)));
//         tri!(iter.try_for_each(|item| serializer.serialize_element(&item)));
//         serializer.end()
//     }

//     fn collect_map<K, V, I>(self, iter: I) -> Result<Self::Ok, Self::Error>
//     where
//         K: Serialize,
//         V: Serialize,
//         I: IntoIterator<Item = (K, V)>,
//     {
//         let mut iter = std::iter.into_iter();
//         let mut serializer = tri!(self.serialize_map(iterator_len_hint(&iter)));
//         tri!(iter.try_for_each(|(key, value)| {
//             serializer.serialize_entry(&key, &value)
//         }));
//         serializer.end()
//     }

//     fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
//     where
//         T: ?Sized + fmt::Display,
//     {
//         self.serialize_str(&value.to_string())
//     }

//     fn is_human_readable(&self) -> bool {
//         true
//     }
// }
