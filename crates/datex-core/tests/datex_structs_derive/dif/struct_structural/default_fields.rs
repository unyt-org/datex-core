use datex_core::libs::core::core_lib_id::CoreLibIdIndex;
use datex_core::libs::core::type_id::CoreLibBaseTypeId;
use datex_macros_internal::Datex;
use crate::datex_structs_derive::dif::deserialize_from_dif_default_ctx;

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct ExampleWithDefaultValue {
    a: String,
    #[datex(default)]
    b: String,
}


#[test]
fn struct_with_defaults_from_dif() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": "Hello, Datex!"
        }
    ]);
    let deserialized: ExampleWithDefaultValue =
        deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, "Hello, Datex!");
    assert_eq!(deserialized.b, "");
}