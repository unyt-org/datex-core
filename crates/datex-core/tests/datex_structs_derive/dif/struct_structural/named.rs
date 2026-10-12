use datex_core::libs::core::core_lib_id::CoreLibIdIndex;
use datex_core::libs::core::type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId};
use datex_core::values::core_values::boolean::Boolean;
use datex_core::values::core_values::endpoint::Endpoint;
use datex_core::values::core_values::integer::typed_integer::{IntegerTypeVariant, TypedInteger};
use datex_core::values::value::Value;
use datex_core::values::value_container::ValueContainer;
use datex_macros_internal::Datex;
use crate::datex_structs_derive::dif::{deserialize_from_dif_default_ctx, serialize_to_json_value_default_ctx};

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct NamedExampleStruct {
    a: u8,
    b: String,
    c: Endpoint,
}

#[test]
fn named_struct_to_dif() {
    let example = NamedExampleStruct {
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
                "a": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42],
                "b": "Hello, Datex!",
                "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
            }
        ])
    );
}


#[test]
fn named_struct_from_dif() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42],
            "b": "Hello, Datex!",
            "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"]
        }
    ]);
    let deserialized: NamedExampleStruct = deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, 42);
    assert_eq!(deserialized.b, "Hello, Datex!");
    assert_eq!(deserialized.c, Endpoint::new("@jonas"));
}

#[test]
fn named_struct_from_dif_with_additional_fields() {
    let json_value = serde_json::json!([
        CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
        {
            "a": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42],
            "b": "Hello, Datex!",
            "c": [CoreLibIdIndex::from(CoreLibBaseTypeId::Endpoint), "@jonas"],
            "d": "additional field"
        }
    ]);
    let deserialized: NamedExampleStruct = deserialize_from_dif_default_ctx(&json_value);

    assert_eq!(deserialized.a, 42);
    assert_eq!(deserialized.b, "Hello, Datex!");
    assert_eq!(deserialized.c, Endpoint::new("@jonas"));
}



#[test]
fn named_struct_roundtrip() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    struct Inner {
        field: String,
        value: Value,
    }



    #[derive(Datex, Debug, Clone, PartialEq)]
    struct ExampleComplexNested {
        name: String,
        i8: i8,
        val: Value,
        vc: ValueContainer,
        ti: TypedInteger,
        cv: Boolean,
        inner: Inner,
    }


    let original = ExampleComplexNested {
        name: "hello".to_string(),
        i8: 42,
        val: Value::from(42i16),
        vc: ValueContainer::from(TypedInteger::I128(123)),
        ti: TypedInteger::I16(-456),
        cv: Boolean::from(true),
        inner: Inner {
            field: "inner field".to_string(),
            value: Value::from(42i16),
        },
    };
    let serialized = serialize_to_json_value_default_ctx(&original);
    let deserialized: ExampleComplexNested =
        deserialize_from_dif_default_ctx(&serialized);
    assert_eq!(original, deserialized);
}
