use core::cell::RefCell;
use datex_core::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    libs::core::{core_lib_id::CoreLibIdIndex, type_id::CoreLibBaseTypeId},
    prelude::*,
    runtime::cache::{
        shared_references_cache::SharedReferencesCache,
        shared_values_cache::SharedValuesCache,
    },
    traits::convert_value_container::ConvertValueContainer,
    values::{
        core_value::CoreValue,
        core_values::{
            boolean::Boolean, endpoint::Endpoint,
            integer::typed_integer::TypedInteger, native::DatexNative,
        },
        value::Value,
        value_container::ValueContainer,
    },
};
use datex_macros_internal::Datex;
use serde::de::IntoDeserializer;

use crate::datex_structs_derive::dif::{
    deserialize_from_dif_default_ctx, serialize_to_json_value_default_ctx,
};

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
enum ExampleEnum {
    Unit,
    TupleSingle(Endpoint),
    TupleMultiple(Endpoint, u8),
    Struct { x: String, y: u16 },
}

#[test]
fn enum_struct_roundtrip() {
    let variant = ExampleEnum::Struct {
        x: "test".to_string(),
        y: 24,
    };
    let serialized = serialize_to_json_value_default_ctx(&variant);
    let json_value = serde_json::json!({
        "t": "Struct",
        "v": {
            "x": "test",
            "y": 24
        }
    });
    println!("json_value: {json_value:#}");
    assert_eq!(serialized, json_value);

    let deserialized: ExampleEnum =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized, variant);
}

#[test]
fn enum_tuple_single_roundtrip() {
    let variant = ExampleEnum::TupleSingle(Endpoint::new("@jonas"));
    let serialized = serialize_to_json_value_default_ctx(&variant);
    let json_value = serde_json::json!({
        "t": "TupleSingle",
        "v": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
    });
    println!("json_value: {json_value:#}");
    assert_eq!(serialized, json_value);

    // let deserialized: ExampleEnum =
    //     deserialize_from_dif_default_ctx(&json_value);

    // assert_eq!(deserialized, variant);
}

#[test]
fn enum_tuple_multiple_roundtrip() {
    let variant = ExampleEnum::TupleMultiple(Endpoint::new("@jonas"), 42);
    let serialized = serialize_to_json_value_default_ctx(&variant);
    let json_value = serde_json::json!({
        "t": "TupleMultiple",
        "v": [[CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"], 42]
    });
    println!("json_value: {json_value:#}");
    assert_eq!(serialized, json_value);

    // let deserialized: ExampleEnum =
    //     deserialize_from_dif_default_ctx(&json_value);

    // assert_eq!(deserialized, variant);
}

#[test]
fn enum_unit_roundtrip() {
    let variant = ExampleEnum::Unit;
    let serialized = serialize_to_json_value_default_ctx(&variant);
    let json_value = serde_json::json!({
        "t": "Unit",
        "v": null
    });
    println!("json_value: {json_value:#}");
    assert_eq!(serialized, json_value);

    // let deserialized: ExampleEnum =
    //     deserialize_from_dif_default_ctx(&json_value);

    // assert_eq!(deserialized, variant);
}
