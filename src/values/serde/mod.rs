pub mod deserializer;
pub mod error;
pub mod serializer;

#[cfg(test)]
mod tests {
    use crate::assert_structural_eq;
    use crate::decompiler::DecompileOptions;
    use crate::logger::init_logger_debug;
    use crate::values::serde::{
        deserializer::{from_bytes, from_value_container},
        serializer::{to_bytes, to_value_container},
    };
    use crate::values::traits::structural_eq::StructuralEq;
    use crate::values::value_container::ValueContainer;
    use datex_core::decompiler::decompile_body;
    use log::info;
    use serde::{Deserialize, Serialize};

    // Tuple Struct
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct TestTupleStruct(String, i32, bool);

    #[test]
    fn tuplestruct_serde_value_container() {
        let original = TestTupleStruct("Hello".to_string(), 42, true);
        let serialized = to_value_container(&original).unwrap();
        let deserialized: TestTupleStruct =
            from_value_container(serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn tuplestruct_serde_bytes() {
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
    fn struct_serde_value_container() {
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
    fn struct_serde_bytes() {
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

    #[test]
    fn test_nested_struct_serde_value_container() {
        let original = NestedStruct {
            nested_field: "Nested".to_string(),
            test_struct: TestStruct {
                field1: "Hello".to_string(),
                field2: 42,
            },
        };
        let serialized = to_value_container(&original).unwrap();
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
        pub value_container: ValueContainer,
    }

    #[test]
    fn struct_with_option_serde_bytes() {
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
    fn core_value_serde_bytes() {
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
    fn struct_with_value_container_serde_bytes() {
        init_logger_debug();
        // struct with value container
        let val = StructWithValueContainer {
            name: "test".to_string(),
            value_container: ValueContainer::from(vec![1, 2, 3]),
        };
        let result = to_bytes(&val).unwrap();
        info!(
            "{}",
            decompile_body(&result, DecompileOptions::colorized()).unwrap()
        );
        let deserialized: StructWithValueContainer =
            from_bytes(&result).unwrap();
        assert_structural_eq!(
            val.value_container,
            deserialized.value_container
        );
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct EmptyStruct;

    #[test]
    fn unit_struct_serde() {
        let original = EmptyStruct;
        let serialized = to_bytes(&original).unwrap();
        let deserialized: EmptyStruct = from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct EmptyTuple();

    #[test]
    fn empty_tuple_struct_serde() {
        let original = EmptyTuple();
        let serialized = to_bytes(&original).unwrap();
        let deserialized: EmptyTuple = from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    // Enum Variants
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    enum TestEnum {
        Unit,
        Tuple(i32, String),
        Struct { x: bool, y: f64 },
    }

    #[test]
    #[ignore = "WIP"]
    fn enum_variants_serde() {
        let tuple = TestEnum::Tuple(42, "hello".to_string());
        let serialized = to_bytes(&tuple).unwrap();
        let deserialized: TestEnum = from_bytes(&serialized).unwrap();
        assert_eq!(tuple, deserialized);

        // let unit = TestEnum::Unit;
        // let strukt = TestEnum::Struct { x: true, y: 3.5 };

        // for original in [unit, tuple, strukt] {
        //     let serialized = to_bytes(&original).unwrap();
        //     let deserialized: TestEnum = from_bytes(&serialized).unwrap();
        //     assert_eq!(original, deserialized);
        // }
    }

    // Collections
    use std::collections::{HashMap, HashSet};

    #[test]
    fn empty_and_nested_collections_serde() {
        // empty vec
        let v: Vec<i32> = vec![];
        let s = to_bytes(&v).unwrap();
        let d: Vec<i32> = from_bytes(&s).unwrap();
        assert_eq!(v, d);

        // nested vec
        let v = vec![vec![1, 2], vec![3, 4]];
        let s = to_bytes(&v).unwrap();
        let d: Vec<Vec<i32>> = from_bytes(&s).unwrap();
        assert_eq!(v, d);

        // hashmap
        let mut map = HashMap::new();
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);
        let s = to_bytes(&map).unwrap();
        let d: HashMap<String, i32> = from_bytes(&s).unwrap();
        assert_eq!(map, d);

        // hashset
        let mut set = HashSet::new();
        set.insert(1);
        set.insert(2);
        let s = to_bytes(&set).unwrap();
        let d: HashSet<i32> = from_bytes(&s).unwrap();
        assert_eq!(set, d);
    }

    #[test]
    fn special_types_serde() {
        // floats
        let val = std::f64::consts::PI;
        let s = to_bytes(&val).unwrap();
        let d: f64 = from_bytes(&s).unwrap();
        assert_eq!(val, d);

        // char
        let val = 'a';
        let s = to_bytes(&val).unwrap();
        let d: char = from_bytes(&s).unwrap();
        assert_eq!(val, d);

        // string with Unicode
        let val = "こんにちは".to_string();
        let s = to_bytes(&val).unwrap();
        let d: String = from_bytes(&s).unwrap();
        assert_eq!(val, d);
    }

    #[test]
    fn error_handling_deserialize() {
        // corrupted bytes
        let bad_data = vec![0xFF, 0x00, 0xAB];
        let result: Result<TestStruct, _> = from_bytes(&bad_data);
        assert!(result.is_err());

        // mismatched type
        let original = TestStruct {
            field1: "wrong".to_string(),
            field2: 123,
        };
        let serialized = to_bytes(&original).unwrap();
        let result: Result<NestedStruct, _> = from_bytes(&serialized);
        assert!(result.is_err());
    }
}
