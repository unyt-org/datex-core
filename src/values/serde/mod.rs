pub mod deserializer;
pub mod error;
pub mod serializer;

#[cfg(test)]
mod tests {
    use log::info;
    use serde::{Deserialize, Serialize};
    use datex_core::decompiler::decompile_body;
    use crate::{assert_structural_eq, assert_value_eq};
    use crate::decompiler::DecompileOptions;
    use crate::logger::init_logger;
    use crate::values::serde::{
        deserializer::{from_bytes, from_value_container},
        serializer::{to_bytes, to_value_container},
    };
    use crate::values::value_container::ValueContainer;
    use crate::values::traits::structural_eq::StructuralEq;

    // Tuple Struct
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct TestTupleStruct(String, i32, bool);

    #[test]
    fn test_tuplestruct_serde_value_container() {
        let original = TestTupleStruct("Hello".to_string(), 42, true);
        let serialized = to_value_container(&original).unwrap();
        let deserialized: TestTupleStruct =
            from_value_container(serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_tuplestruct_serde_bytes() {
        let original = TestTupleStruct("Hello".to_string(), 42, true);
        let serialized = to_bytes(&original).unwrap();
        let deserialized: TestTupleStruct = from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    // Struct
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_struct_serde_value_container() {
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
    fn test_struct_serde_bytes() {
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

    // Nested Struct
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct NestedStruct {
        nested_field: String,
        test_struct: TestStruct,
    }

    // FIXME
    #[test]
    // #[ignore = "This test is currently failing"]
    fn test_nested_struct_serde_value_container() {
        let original = NestedStruct {
            nested_field: "Nested".to_string(),
            test_struct: TestStruct {
                field1: "Hello".to_string(),
                field2: 42,
            },
        };
        let serialized = to_value_container(&original).unwrap();
        println!("Serialized: {serialized}");

        let deserialized: NestedStruct =
            from_value_container(serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq)]
    pub struct StructWithUSize {
        pub usize: Option<usize>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct StructWithValueContainer {
        pub name: String,
        pub value_container: ValueContainer
    }

    #[test]
    fn test_struct_with_option_serde_bytes() {
        // struct with option
        let val = StructWithUSize { usize: Some(42) };
        let result = to_bytes(&val);
        assert!(result.is_ok());
        let deserialized: StructWithUSize =
            from_bytes(&result.unwrap()).unwrap();
        assert_eq!(val, deserialized);
    }

    // Core Value
    #[test]
    fn test_core_value_serde_bytes() {
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

    #[test]
    fn test_struct_with_value_container_serde_bytes() {
        init_logger();
        // struct with value container
        let val = StructWithValueContainer {
            name: "test".to_string(),
            value_container: ValueContainer::from(vec![1, 2, 3]),
        };
        let result = to_bytes(&val).unwrap();
        info!("{}", decompile_body(&result, DecompileOptions::colorized()).unwrap());
        let deserialized: StructWithValueContainer =
            from_bytes(&result).unwrap();
        assert_structural_eq!(val.value_container, deserialized.value_container);
    }
}
