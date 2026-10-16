use crate::datex_structs_derive::dif::{
    deserialize_from_dif, deserialize_from_dif_default_ctx,
    serialize_to_json_value_default_ctx,
};
use core::cell::RefCell;
use datex_core::{
    dif::serde_context::SerdeContext,
    libs::core::{
        core_lib_id::CoreLibIdIndex,
        type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId},
    },
    runtime::{
        Runtime,
        cache::{
            shared_references_cache::SharedReferencesCache,
            shared_values_cache::SharedValuesCache,
        },
    },
    shared_values::SharedContainer,
    traits::{
        convert_value_container::ConvertValueContainer,
        get_datex_type::GetDatexType,
    },
    types::r#type::Type,
    values::{
        core_values::integer::typed_integer::IntegerTypeVariant,
        value_container::ValueContainer,
    },
};
use datex_macros_internal::Datex;

#[derive(Datex, Debug, Clone, PartialEq)]
struct NamedExampleEntityStruct {
    a: u8,
    b: String,
}

#[derive(Datex, Debug, Clone, PartialEq)]
struct NamedExampleEntityStructFake {
    a: u8,
    b: String,
}

#[test]
fn named_entity_struct_to_dif() {
    let cache = &mut SharedReferencesCache::default();

    let example = NamedExampleEntityStruct {
        a: 42,
        b: "Hello, Datex!".to_string(),
    };
    let example_vc = example.to_value_container();
    let serialized = serialize_to_json_value_default_ctx(example_vc);

    let ptr_address = match NamedExampleEntityStruct::datex_type(cache) {
        Type::Entity(entity_type) => {
            entity_type.pointer_address().to_address_string()
        }
        _ => panic!("Expected NamedExampleEntityStruct to be an entity type"),
    };

    assert_eq!(
        serialized,
        serde_json::json!({
            "_": "c",
            "c": [ptr_address],
            "v": [
                 CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
                {
                    "a": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42],
                    "b": "Hello, Datex!",
                }
            ]
        })
    );
}

#[test]
fn named_entity_struct_from_dif() {
    let runtime = Runtime::stub();

    let entity_type_ptr_address = match NamedExampleEntityStruct::datex_type(
        &mut runtime.shared_references_cache_mut(),
    ) {
        Type::Entity(entity_type) => entity_type.pointer_address(),
        _ => panic!("Expected NamedExampleEntityStruct to be an entity type"),
    };

    // type must exist in the runtime cache
    let resolved_type_container = SharedContainer::Referenced(
        runtime
            .shared_references_cache()
            .get_owned_reference(&entity_type_ptr_address)
            .unwrap()
            .clone(),
    );

    // explicitly store the resolved type in the values cache to ensure it can be deserialized correctly
    // TODO: test this by first serializing the type?
    let mut values_cache = SharedValuesCache::default();
    values_cache.store_shared_container(resolved_type_container);
    let values_cache = RefCell::new(values_cache);
    let ctx = SerdeContext::new(&values_cache);

    let json_value = serde_json::json!({
        "_": "c",
        "c": [format!("'${}", entity_type_ptr_address.to_address_string())],
        "v": [
            CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
            {
                "a": [CoreLibIdIndex::from(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)), 42],
                "b": "Hello, Datex!",
            }
        ]
    });

    // TODO also allow deserialized: deserialize_from_dif(&ctx, &json_value) directly by passing deserialize to inner val

    let deserialized_vc: ValueContainer =
        deserialize_from_dif(&ctx, &json_value);
    let deserialized: NamedExampleEntityStruct = deserialized_vc
        .try_into_value()
        .expect("Failed to convert ValueContainer to NamedExampleEntityStruct");

    assert_eq!(deserialized.a, 42);
    assert_eq!(deserialized.b, "Hello, Datex!");

    // cannot deserialize to a different struct with the same fields, as the type pointer address will not match
    let deserialized_vc: ValueContainer =
        deserialize_from_dif(&ctx, &json_value);
    assert!(
        deserialized_vc
            .try_into_value::<NamedExampleEntityStructFake>()
            .is_err()
    );
}
