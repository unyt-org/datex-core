use datex_core::libs::core::core_lib_id::CoreLibIdIndex;
use datex_core::libs::core::type_id::CoreLibBaseTypeId;
use datex_macros_internal::Datex;
use crate::datex_structs_derive::dif::deserialize_from_dif_default_ctx;

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct ExampleWithOptionalValue {
    a: String,
    b: Option<String>,
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
    let deserialized: ExampleWithOptionalValue =
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
    let deserialized: ExampleWithOptionalValue =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, "Hello, Datex!");
    assert_eq!(deserialized.b, None);
}