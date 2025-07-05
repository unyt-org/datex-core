pub mod deserializer;
pub mod error;
pub mod serializer;

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::values::serde::{
        deserializer::{from_bytes, from_value_container},
        serializer::{to_bytes, to_value_container},
    };

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_serde_value_container() {
        let original = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let serialized = to_value_container(&original).unwrap();
        let deserialized: TestStruct =
            from_value_container(serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_serde_bytes() {
        let data = to_bytes(&TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        })
        .unwrap();
        let result: TestStruct = from_bytes(&data).unwrap();
        assert_eq!(
            result,
            TestStruct {
                field1: "Hello".to_string(),
                field2: 42
            }
        );
    }
}
