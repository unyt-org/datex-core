use crate::datex_structs_derive::dif::serialize_to_json_value_default_ctx;
use datex_core::{
    libs::core::{
        core_lib_id::CoreLibIdIndex,
        type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId},
    },
    values::core_values::integer::typed_integer::IntegerTypeVariant,
};
use datex_macros_internal::Datex;

#[derive(Datex, Debug, Clone, PartialEq)]
struct UnnamedExampleStruct(String, u8);

#[test]
fn unnamed_struct_to_dif() {
    let example = UnnamedExampleStruct("Hello, Datex!".to_string(), 42);
    let serialized = serialize_to_json_value_default_ctx(example);

    assert_eq!(
        serialized,
        serde_json::json!([
            CoreLibIdIndex::from(CoreLibBaseTypeId::List),
            [
                "Hello, Datex!",
                [
                    CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(
                        IntegerTypeVariant::U8
                    )),
                    42
                ]
            ]
        ])
    );
}

#[test]
fn unnamed_struct_from_dif() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::List),
        [
            "Hello, Datex!",
            [
                CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(
                    IntegerTypeVariant::U8
                )),
                42
            ]
        ]
    ]);
    let deserialized: UnnamedExampleStruct =
        crate::datex_structs_derive::dif::deserialize_from_dif_default_ctx(
            &json_value,
        );

    assert_eq!(deserialized.0, "Hello, Datex!");
    assert_eq!(deserialized.1, 42);
}
