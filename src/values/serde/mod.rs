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

    #[test]
    fn test_core_value() {
        // string
        let val = "test string";
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: String = from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);

        // integer
        let val = 42;
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: i32 = from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);

        // boolean
        let val = true;
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: bool = from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);

        // null
        let val: Option<()> = None;
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: Option<()> = from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);

        // array
        let val = vec![1, 2, 3];
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: Vec<i32> = from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);

        // tuple
        let val = (1, "test".to_string(), true);
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: (i32, String, bool) =
            from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);
    }
}
