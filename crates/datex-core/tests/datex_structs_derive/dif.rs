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
        core_values::{
            endpoint::Endpoint, integer::typed_integer::TypedInteger,
            native::DatexNative,
        },
        value::Value,
        value_container::ValueContainer,
    },
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
    value: impl ConvertValueContainer,
) -> serde_json::Value {
    let value_container = value.to_value_container();
    let serializer = serde_json::value::Serializer;
    value_container
        .serialize_with_ctx(context, serializer)
        .unwrap()
}

/// Generates a serialized DIF JSON string from a value using a default SerdeContext and SharedValuesCache.
fn serialize_to_json_value_default_ctx(
    value: impl ConvertValueContainer,
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
    let serialized = serialize_to_json_value_default_ctx(example);

    assert_eq!(
        serialized,
        serde_json::json!([
            CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
            {
                "a": 42,
                "b": "Hello, Datex!",
                "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
            }
        ])
    );
}

#[test]
fn struct_from_dif() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": 42,
            "b": "Hello, Datex!",
            "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
        }
    ]);
    let deserialized: Example = deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, 42);
    assert_eq!(deserialized.b, "Hello, Datex!");
    assert_eq!(deserialized.c, Endpoint::new("@jonas"));
}

#[test]
fn struct_with_optional_values_from_dif_null() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": "Hello, Datex!",
            "b": null
        }
    ]);
    let deserialized: ExampleWithOptionalValues =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, "Hello, Datex!");
    assert_eq!(deserialized.b, None);
}

#[test]
fn struct_with_optional_values_from_dif_omitted() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": "Hello, Datex!"
        }
    ]);
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

#[derive(Datex, Debug, Clone, PartialEq)]
struct TestConfig {
    name: String,
    i8: i8,
    val: Value,
    vc: ValueContainer,
    ti: TypedInteger,
}

#[test]
fn serde_context_round_trip() {
    let original = TestConfig {
        name: "hello".to_string(),
        i8: 42,
        val: Value::from(42i16),
        vc: ValueContainer::from(TypedInteger::I128(123)),
        ti: TypedInteger::I16(-456),
    };
    let cache = RefCell::new(SharedValuesCache::default());
    let ctx = SerdeContext::new(&cache);

    let value =
        serde_json::to_value(ValueWithSerdeContext::new(&original, &ctx))
            .expect("serialization should succeed");

    println!("ser {value:#}");

    let deserialized =
        TestConfig::deserialize_with_ctx(&ctx, value.into_deserializer())
            .expect("deserialization should succeed");

    assert_eq!(original, deserialized);
}
