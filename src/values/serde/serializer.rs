use serde::ser::{
    Serialize, SerializeStruct, SerializeTuple, SerializeTupleStruct,
    Serializer,
};
use std::fmt::Display;

use crate::compiler::compile_value;
use crate::values::core_value::CoreValue;
use crate::values::core_values::object::Object;
use crate::values::core_values::tuple::Tuple;
use crate::values::serde::error::SerializationError;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
pub struct DatexSerializer {
    container: ValueContainer,
}

impl Default for DatexSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl DatexSerializer {
    pub fn new() -> Self {
        DatexSerializer {
            container: CoreValue::Null.into(),
        }
    }

    pub fn into_inner(self) -> ValueContainer {
        self.container
    }
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>, SerializationError>
where
    T: Serialize,
{
    let value_container = to_value_container(value)?;
    println!("Value container: {value_container:?}");
    compile_value(&value_container).map_err(|e| {
        SerializationError(format!("Failed to compile value: {e}"))
    })
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

impl SerializeStruct for &mut DatexSerializer {
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
        match self.container {
            ValueContainer::Value(Value {
                inner: CoreValue::Object(ref mut obj),
                ..
            }) => {
                obj.set(key, value_container);
            }
            _ => {
                return Err(SerializationError(
                    "Cannot serialize field into non-object container"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.container.clone())
    }
}

impl SerializeTuple for &mut DatexSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_element<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value_container = value.serialize(&mut **self)?;
        match self.container {
            ValueContainer::Value(Value {
                inner: CoreValue::Tuple(ref mut tuple),
                ..
            }) => {
                tuple.insert(value_container);
            }
            _ => {
                return Err(SerializationError(
                    "Cannot serialize element into non-tuple container"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.container.clone())
    }
}

impl SerializeTupleStruct for &mut DatexSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value_container = value.serialize(&mut **self)?;
        match self.container {
            ValueContainer::Value(Value {
                inner: CoreValue::Tuple(ref mut tuple),
                ..
            }) => {
                tuple.insert(value_container);
            }
            _ => {
                return Err(SerializationError(
                    "Cannot serialize element into non-tuple container"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.container.clone())
    }
}

impl Serializer for &mut DatexSerializer {
    type Ok = ValueContainer;
    type Error = SerializationError;

    // Non implemented types
    type SerializeSeq = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    // Implemented types
    type SerializeStruct = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.container = Object::new().into();
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
        println!("Serializing str: {} {}", v, ValueContainer::from(v));
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
        self.container = Tuple::default().into();
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.container = Tuple::default().into();
        Ok(self)
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
        Ok(ValueContainer::from(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueContainer::from(v))
    }

    fn collect_seq<I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Serialize,
    {
        let mut seq = Vec::new();
        for item in iter {
            let value_container = item.serialize(&mut *self)?;
            seq.push(value_container);
        }
        Ok(ValueContainer::from(seq))
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
        println!("Collecting str: {value}");
        self.serialize_str(&value.to_string())
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use crate::values::{
        core_value::CoreValue,
        serde::serializer::{DatexSerializer, to_bytes, to_value_container},
        value::Value,
        value_container::ValueContainer,
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
        assert_matches!(
            result,
            ValueContainer::Value(Value {
                inner: CoreValue::Object(_),
                ..
            })
        );
    }
}
