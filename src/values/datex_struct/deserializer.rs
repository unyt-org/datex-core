use serde::Deserialize;

use crate::values::{value::Value, value_container::ValueContainer};

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

pub fn from_bytes<'a, T>(s: Vec<u8>) -> Result<T, ()>
where
    T: Deserialize<'a>,
{
    let mut deserializer = DatexDeserializer::new(&s);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let data = b"test data";
        let result: Result<String, ()> = from_bytes(data);
        assert!(result.is_ok());
    }
}
