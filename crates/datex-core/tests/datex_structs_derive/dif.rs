use core::cell::RefCell;
use datex_core::{
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::{core_lib_id::CoreLibIdIndex, type_id::CoreLibBaseTypeId},
    prelude::*,
    runtime::cache::shared_values_cache::SharedValuesCache,
    values::core_values::endpoint::Endpoint,
};
use datex_macros_internal::Datex;
use serde::de::IntoDeserializer;

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct Example {
    a: u8,
    b: String,
    c: Endpoint,
}

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct ExampleWithOptionalValues {
    a: String,
    b: Option<String>,
}

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
enum ExampleEnum {
    Variant1(Endpoint),
    Variant2 { x: String, y: u16 },
    Variant3,
}

/// TODO: general helper function in datex_core
fn serialize_to_json_value(
    context: &SerdeContext,
    value: &impl SerializeWithSerdeContext,
) -> serde_json::Value {
    let serializer = serde_json::value::Serializer;
    value.serialize_with_ctx(context, serializer).unwrap()
}

/// Generates a serialized DIF JSON string from a value using a default SerdeContext and SharedValuesCache.
fn serialize_to_json_value_default_ctx(
    value: &impl SerializeWithSerdeContext,
) -> serde_json::Value {
    let cache = RefCell::new(SharedValuesCache::default());
    let context = SerdeContext::new(&cache);
    serialize_to_json_value(&context, value)
}

fn deserialize_from_dif<'de, T: DeserializeWithSerdeContext<'de>>(
    context: &SerdeContext<'_>,
    json_value: &'de serde_json::Value,
) -> T {
    let deserializer = json_value.into_deserializer();
    T::deserialize_with_ctx(context, deserializer).unwrap()
}

fn deserialize_from_dif_default_ctx<
    'de,
    T: DeserializeWithSerdeContext<'de>,
>(
    json_value: &'de serde_json::Value,
) -> T {
    let cache = RefCell::new(SharedValuesCache::default());
    let context = SerdeContext::new(&cache);
    deserialize_from_dif(&context, json_value)
}

#[test]
fn struct_to_dif() {
    let example = Example {
        a: 42,
        b: "Hello, Datex!".to_string(),
        c: Endpoint::new("@jonas"),
    };
    let serialized = serialize_to_json_value_default_ctx(&example);

    assert_eq!(
        serialized,
        serde_json::json!({
            "a": 42,
            "b": "Hello, Datex!",
            "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
        })
    );
}

#[test]
fn struct_from_dif() {
    let json_value = serde_json::json!({
        "a": 42,
        "b": "Hello, Datex!",
        "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
    });
    let deserialized: Example =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, 42);
    assert_eq!(deserialized.b, "Hello, Datex!");
    assert_eq!(deserialized.c, Endpoint::new("@jonas"));
}

#[test]
fn struct_with_optional_values_from_dif_null() {
    let json_value = serde_json::json!({
        "a": "Hello, Datex!",
        "b": null
    });
    let deserialized: ExampleWithOptionalValues =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, "Hello, Datex!");
    assert_eq!(deserialized.b, None);
}

#[test]
fn struct_with_optional_values_from_dif_omitted() {
    let json_value = serde_json::json!({
        "a": "Hello, Datex!"
    });
    let deserialized: ExampleWithOptionalValues =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, "Hello, Datex!");
    assert_eq!(deserialized.b, None);
}

#[test]
fn enum_variant_1_from_dif() {
    let json_value = serde_json::json!({
        "t": "Variant1",
        "v": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
    });
    let deserialized: ExampleEnum =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized, ExampleEnum::Variant1(Endpoint::new("@jonas")));
}
