use datex_core::libs::core::core_lib_id::CoreLibIdIndex;
use datex_core::libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId};
use datex_core::values::core_values::integer::typed_integer::IntegerTypeVariant;
use datex_macros_internal::Datex;
use crate::datex_structs_derive;

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct Inner {
    a: String,
    b: u8,
}

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct ExampleUnitStruct(String);


#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct ExampleUnitStructComplex(Inner);

#[test]
fn transparent_struct_to_dif() {
    let example = ExampleUnitStruct("Hello, Datex!".to_string());
    let serialized = datex_structs_derive::dif::serialize_to_json_value_default_ctx(example);

    assert_eq!(
        serialized,
        serde_json::json!("Hello, Datex!")
    );
}

#[test]
fn nested_transparent_struct_to_dif() {
    let example = ExampleUnitStructComplex(Inner {
        a: "Hello, Datex!".to_string(),
        b: 42,
    });
    let serialized = datex_structs_derive::dif::serialize_to_json_value_default_ctx(example);

    assert_eq!(
        serialized,
        serde_json::json!([
            CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
            {
                "a": "Hello, Datex!",
                "b": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42]
            }
        ])
    );
}

#[test]
fn transparent_struct_from_dif() {
    let json_value = serde_json::json!("Hello, Datex!");
    let deserialized: ExampleUnitStruct =
        datex_structs_derive::dif::deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.0, "Hello, Datex!");
}

#[test]
fn nested_transparent_struct_from_dif() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": "Hello, Datex!",
            "b": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42]
        }
    ]);
    let deserialized: ExampleUnitStructComplex =
        datex_structs_derive::dif::deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.0.a, "Hello, Datex!");
    assert_eq!(deserialized.0.b, 42);
}