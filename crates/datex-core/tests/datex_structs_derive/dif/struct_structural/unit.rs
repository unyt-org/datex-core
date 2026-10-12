use datex_core::libs::core::type_id::{CoreLibTypeId, CoreLibVariantTypeId};
use datex_macros_internal::Datex;
use crate::datex_structs_derive;


#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct UnitStruct;

#[test]
fn unit_struct_to_dif() {
    let example = UnitStruct;
    let serialized = datex_structs_derive::dif::serialize_to_json_value_default_ctx(example);

    assert_eq!(
        serialized,
        serde_json::json!(null)
    );
}

#[test]
fn unit_struct_from_dif() {
    let json_value = serde_json::json!(null);
    let deserialized: UnitStruct =
        datex_structs_derive::dif::deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized, UnitStruct);
}