use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
    forward_to_deserialize_any,
};

use crate::values::{
    datex_struct::error::SerializationError, value::Value,
    value_container::ValueContainer,
};

pub struct DatexDeserializer {
    object: ValueContainer,
    input: Vec<u8>,
}

impl Default for DatexDeserializer {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl DatexDeserializer {
    pub fn new(input: &[u8]) -> Self {
        DatexDeserializer {
            object: Value::null().into(),
            input: input.to_vec(),
        }
    }
}

impl<'de> Deserializer<'de> for DatexDeserializer {
    type Error = SerializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
        // use ValueContainer::*;

        // match self {
        //     Null => visitor.visit_unit(),
        //     Bool(b) => visitor.visit_bool(b),
        //     I64(i) => visitor.visit_i64(i),
        //     U64(u) => visitor.visit_u64(u),
        //     F64(f) => visitor.visit_f64(f),
        //     String(s) => visitor.visit_string(s),

        //     Array(vec) => {
        //         // Re‑wrap every element in its own DatexDeserializer
        //         let seq = vec.into_iter().map(DatexDeserializer::from_value);
        //         visitor.visit_seq(de::value::SeqDeserializer::new(seq))
        //     }

        //     Object(map) => {
        //         let entries = map
        //             .into_iter()
        //             .map(|(k, v)| (k, DatexDeserializer::from_value(v)));
        //         visitor.visit_map(de::value::MapDeserializer::new(entries))
        //     }
        // }
    }

    // Hand the rest to `deserialize_any`
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }

    fn is_human_readable(&self) -> bool {
        false // binary format
    }
}

pub fn from_bytes<'de, T>(input: &'de [u8]) -> Result<T, SerializationError>
where
    T: serde::Deserialize<'de>,
{
    let deserializer = DatexDeserializer::new(input);
    T::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use crate::values::datex_struct::serializer::to_bytes;

    use super::*;

    #[derive(Deserialize, Serialize)]
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
    }
}
