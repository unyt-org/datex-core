pub mod dif;
mod impls;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;

use core::{assert_matches, cell::RefCell};
use datex_core::{
    prelude::*,
    traits::get_datex_type::GetDatexType,
    values::{
        core_values::{endpoint::Endpoint, map::Map},
        value_container::ValueContainer,
    },
};
use datex_macros_internal::Datex;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[derive(Datex, Debug)]
#[datex(structural)]
enum ExampleEnum {
    VariantA,
    VariantB(u8, u8),
    VariantC { x: u8, y: String },
    VariantD(u8),
}

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct Example {
    a: u8,
    b: String,
    c: Endpoint,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct SerdeExample {
    inner_a: u8,
    inner_b: String,
}

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
struct SerdeDatexExample {
    a: u8,
    #[datex(serde)]
    serde: SerdeExample,
}

#[derive(Datex, Debug, PartialEq)]
#[datex(structural)]
struct ExampleNewType(Example);

fn assert_round_trip<T>(value: T)
where
    T: ConvertValue
        + ConvertValueContainer
        + PartialEq
        + std::fmt::Debug
        + Clone,
{
    let value_container = ValueContainer::from(value.clone());
    let deserialized_value =
        T::try_from_value_container(value_container).unwrap();
    assert_eq!(value, deserialized_value);
}

use datex_core::{
    self,
    libs::core::type_id::{CoreLibBaseTypeId, CoreLibVariantTypeId},
    runtime::{
        cache::shared_references_cache::SharedReferencesCache,
        pointer_address_provider::SelfOwnedPointerAddressProvider,
    },
    shared_values::{
        OwnedSharedContainer, SharedContainer, SharedContainerMutability,
    },
    traits::{
        classification::Classification, convert_value::ConvertValue,
        convert_value_container::ConvertValueContainer,
        structural_eq::assert_structural_eq,
    },
    types::{
        literal_type_definition::LiteralTypeDefinition,
        r#type::Type,
        type_definition::{
            TypeDefinition, list::ListTypeDefinition, map::MapTypeDefinition,
            tagged_type::TaggedTypeDefinition, union::UnionTypeDefinition,
        },
    },
    values::{
        core_value::CoreValue,
        core_values::{
            integer::typed_integer::{IntegerTypeVariant, TypedInteger},
            list::List,
        },
        value::{
            Value,
            value_classification::{ValueClassification, ValueTag},
        },
    },
};
use test_case::test_case;

#[test_case(
    Example {
        a: 42u8,
        b: "Test".to_string(),
        c: Endpoint::default(),
    } ; "example struct")]
#[test_case(
    SerdeDatexExample {
        a: 42u8,
        serde: SerdeExample {
            inner_a: 1,
            inner_b: "Inner".to_string(),
        },
    } ; "struct with serde field")]
#[test_case(vec![1u8, 2, 3] ; "vector of primitives")]
#[test_case(vec![Endpoint::try_from("@ben").unwrap(), Endpoint::try_from("@jonas").unwrap()] ; "vector of datex direct types")]
#[test_case(Map::from(vec![
    ("key1".to_string(), ValueContainer::from(42u8)),
    ("key2".to_string(), ValueContainer::from("Value".to_string())),
]) ; "map of primitives")]
fn round_trip_struct<T>(structure: T)
where
    T: ConvertValue
        + ConvertValueContainer
        + PartialEq
        + std::fmt::Debug
        + Clone,
{
    assert_round_trip(structure);
}

#[test]
fn struct_to_value_container() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let value_container: ValueContainer = Example {
        a: 42u8,
        b: "Test".to_string(),
        c: Endpoint::default(),
    }
    .into();

    let map: Map = Map::try_cast_from_value_container(value_container).unwrap();
    assert_eq!(map.try_get("a").unwrap(), &ValueContainer::from(42u8));
    assert_eq!(
        map.try_get("b").unwrap(),
        &ValueContainer::from("Test".to_string())
    );
    assert_eq!(
        map.try_get("c").unwrap(),
        &ValueContainer::from(Endpoint::default())
    );
}

#[test]
fn skip() {
    let cache = RefCell::new(SharedReferencesCache::default());

    #[derive(Datex, Debug, PartialEq)]
    #[datex(only_structural, no_deserialize)]
    struct SerdeDatexWithSkip {
        a: u8,

        #[datex(skip)]
        b: String,
    }
    let value_container: ValueContainer = SerdeDatexWithSkip {
        a: 42,
        b: "Hello".to_string(),
    }
    .into();

    let map = Map::try_cast_from_value_container(value_container).unwrap();
    assert!(map.has("a"));
    assert!(!map.has("b"));

    let value_container = ValueContainer::from(map);
    // Attempting to deserialize should fail because the `b` field is skipped and cannot be deserialized
    assert!(
        SerdeDatexWithSkip::try_cast_from_value_container(value_container)
            .is_err()
    )
}

#[test]
fn skip2() {
    #[derive(Default, Debug, PartialEq)]
    struct NoDerive {
        a: u8,
        b: String,
    }
    #[derive(Datex, Debug, PartialEq)]
    #[datex(only_structural, no_deserialize)]
    struct SerdeDatexWithSkip2 {
        a: u8,
        #[datex(skip)]
        b: NoDerive,
    }
    let value_container: ValueContainer = SerdeDatexWithSkip2 {
        a: 42,
        b: NoDerive {
            a: 1,
            b: "Hello".to_string(),
        },
    }
    .into();
    let deserialized = value_container
        .try_into_value::<SerdeDatexWithSkip2>()
        .unwrap();
    assert_eq!(deserialized.a, 42);
    assert_eq!(
        deserialized.b,
        NoDerive {
            a: 1,
            b: "Hello".to_string(),
        }
    );
}

#[test]
fn default() {
    let cache = RefCell::new(SharedReferencesCache::default());

    #[derive(Datex, Debug, PartialEq)]
    #[datex(only_structural)]
    struct SerdeDatexWithDefault {
        a: u8,
        #[datex(default)]
        b: String,
    }

    let map: Map =
        Map::from(vec![("a".to_string(), ValueContainer::from(42u8))]);
    let value_container = ValueContainer::from(map);
    let deserialized =
        SerdeDatexWithDefault::try_cast_from_value_container(value_container)
            .unwrap();
    assert_eq!(deserialized.a, 42);
    assert_eq!(deserialized.b, "".to_string());
}

#[test]
fn enum_to_value() {
    let cache = &mut SharedReferencesCache::default();
    let variant_a: Value = ExampleEnum::VariantA.into();

    // TODO:
    // assert_structural_eq!(variant_a, Value::null());
    assert_eq!(
        variant_a.classification(cache),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantA".to_string(),
            is_empty: true
        })
    );

    let variant_b: Value = ExampleEnum::VariantB(1, 2).into();
    // TODO:
    // assert_structural_eq!(
    //     variant_b,
    //     Value::from(vec![Value::from(1u8), Value::from(2u8)])
    // );
    assert_eq!(
        variant_b.classification(cache),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantB".to_string(),
            is_empty: false
        })
    );

    let variant_c: Value = ExampleEnum::VariantC {
        x: 3,
        y: "Hello".to_string(),
    }
    .into();
    // TODO:
    // assert_structural_eq!(
    //     variant_c,
    //     Value::from(Map::from(vec![
    //         ("x".to_string(), Value::from(3).into()),
    //         ("y".to_string(), Value::from("Hello".to_string()).into()),
    //     ]))
    // );
    assert_eq!(
        variant_c.classification(cache),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantC".to_string(),
            is_empty: false
        })
    );

    let variant_d: Value = ExampleEnum::VariantD(1).into();
    // TODO:
    // assert_structural_eq!(variant_d, Value::from(1u8));
    assert_eq!(
        variant_d.classification(cache),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantD".to_string(),
            is_empty: false
        })
    );
}

#[test]
fn struct_to_value() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let value: Value = Example {
        a: 42u8,
        b: "Test".to_string(),
        c: Endpoint::default(),
    }
    .into();

    let map = Map::try_cast_from_value_container(ValueContainer::from(value))
        .unwrap();
    assert_eq!(map.try_get("a").unwrap(), &ValueContainer::from(42u8));
    assert_eq!(
        map.try_get("b").unwrap(),
        &ValueContainer::from("Test".to_string())
    );
    assert_eq!(
        map.try_get("c").unwrap(),
        &ValueContainer::from(Endpoint::default())
    );
}

#[test]
fn new_type_struct_to_value() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let value: Value = ExampleNewType(Example {
        a: 42u8,
        b: "Test".to_string(),
        c: Endpoint::default(),
    })
    .into();

    let map = Map::try_cast_from_value_container(ValueContainer::from(value))
        .unwrap();
    assert_eq!(map.try_get("a").unwrap(), &ValueContainer::from(42u8));
    assert_eq!(
        map.try_get("b").unwrap(),
        &ValueContainer::from("Test".to_string())
    );
    assert_eq!(
        map.try_get("c").unwrap(),
        &ValueContainer::from(Endpoint::default())
    );
}

#[test]
fn value_container_to_struct() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let value_container: ValueContainer =
        ValueContainer::from(Map::from(vec![
            ("a".to_string(), ValueContainer::from(42u8)),
            ("b".to_string(), ValueContainer::from("Test".to_string())),
            ("c".to_string(), ValueContainer::from(Endpoint::default())),
        ]));

    let example =
        Example::try_cast_from_value_container(value_container).unwrap();

    assert_eq!(example.a, 42u8);
    assert_eq!(example.b, "Test".to_string());
    assert_eq!(example.c, Endpoint::default());
}

#[test]
fn value_to_struct() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let value: Value = Value::from(Map::from(vec![
        ("a".to_string(), ValueContainer::from(42u8)),
        ("b".to_string(), ValueContainer::from("Test".to_string())),
        ("c".to_string(), ValueContainer::from(Endpoint::default())),
    ]));

    let example =
        Example::try_cast_from_value_container(ValueContainer::from(value))
            .unwrap();

    assert_eq!(example.a, 42u8);
    assert_eq!(example.b, "Test".to_string());
    assert_eq!(example.c, Endpoint::default());
}

#[test]
fn value_to_new_typestruct() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let value: Value = Value::from(Map::from(vec![
        ("a".to_string(), ValueContainer::from(42u8)),
        ("b".to_string(), ValueContainer::from("Test".to_string())),
        ("c".to_string(), ValueContainer::from(Endpoint::default())),
    ]));

    let example = ExampleNewType::try_cast_from_value_container(
        ValueContainer::from(value),
    )
    .unwrap();

    assert_eq!(example.0.a, 42u8);
    assert_eq!(example.0.b, "Test".to_string());
    assert_eq!(example.0.c, Endpoint::default());
}

#[test]
fn value_to_empty_enum_variant() {
    let cache = RefCell::new(SharedReferencesCache::default());

    let variant_a = Value::core_with_classification(
        CoreValue::Null,
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantA".to_string(),
            is_empty: true,
        }),
    );

    let example = ExampleEnum::try_cast_from_value_container(
        ValueContainer::from(variant_a),
    )
    .unwrap();
    assert_matches!(example, ExampleEnum::VariantA);
}

#[test]
fn value_to_list_enum_variant() {
    let cache = RefCell::new(SharedReferencesCache::default());

    let variant_b = Value::core_with_classification(
        List::new(vec![ValueContainer::from(1u8), ValueContainer::from(2u8)]),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantB".to_string(),
            is_empty: false,
        }),
    );
    let example = ExampleEnum::try_cast_from_value_container(
        ValueContainer::from(variant_b),
    )
    .unwrap();
    assert_matches!(example, ExampleEnum::VariantB(1, 2));
}

#[test]
fn value_to_map_enum_variant() {
    let cache = RefCell::new(SharedReferencesCache::default());

    let variant_c = Value::core_with_classification(
        Map::from(vec![
            ("x".to_string(), ValueContainer::from(3u8)),
            ("y".to_string(), ValueContainer::from("Hello".to_string())),
        ]),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantC".to_string(),
            is_empty: false,
        }),
    );
    let example = ExampleEnum::try_cast_from_value_container(
        ValueContainer::from(variant_c),
    )
    .unwrap();
    assert_matches!(example, ExampleEnum::VariantC { x: 3, y } if &y == "Hello" );
}

#[test]
fn value_to_transparent_enum_variant() {
    let cache = RefCell::new(SharedReferencesCache::default());

    let variant_d = Value::core_with_classification(
        TypedInteger::U8(42),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantD".to_string(),
            is_empty: false,
        }),
    );

    let example = ExampleEnum::try_cast_from_value_container(
        ValueContainer::from(variant_d),
    )
    .unwrap();
    assert_matches!(example, ExampleEnum::VariantD(42));
}

#[test]
fn value_to_enum_failure() {
    let invalid_variant = Value::core_with_classification(
        List::new(vec![ValueContainer::from(1u8), ValueContainer::from(2u8)]),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantX".to_string(),
            is_empty: false,
        }),
    );
    assert!(invalid_variant.try_into_value::<ExampleEnum>().is_err());

    let invalid_variant = Value::core_with_classification(
        TypedInteger::U8(42),
        ValueClassification::new_with_tag(ValueTag {
            tag: "VariantA".to_string(),
            is_empty: false,
        }),
    );
    assert!(invalid_variant.try_into_value::<ExampleEnum>().is_err());
}

#[test]
fn struct_with_serde_to_value_container() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let serde_example = SerdeDatexExample {
        a: 42u8,
        serde: SerdeExample {
            inner_a: 1,
            inner_b: "Inner".to_string(),
        },
    };

    // Note: uses try_into because of datex(serde)
    let value_container: ValueContainer = serde_example.try_into().unwrap();

    let map: Map = Map::try_cast_from_value_container(value_container).unwrap();
    assert_eq!(map.try_get("a").unwrap(), &ValueContainer::from(42u8));
    let serde_map: Map = map
        .try_get("serde")
        .unwrap()
        .clone()
        .try_into_value()
        .unwrap();
    assert_eq!(
        serde_map.try_get("inner_a").unwrap(),
        &ValueContainer::from(1u8)
    );
    assert_eq!(
        serde_map.try_get("inner_b").unwrap(),
        &ValueContainer::from("Inner".to_string())
    );
}

#[test]
fn struct_with_serde_infallible_to_value_container() {
    let cache = RefCell::new(SharedReferencesCache::default());

    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct SerdeDatexExampleInfallible {
        a: u8,
        #[datex(serde)]
        serde: SerdeExample,
    }

    let serde_example = SerdeDatexExampleInfallible {
        a: 42u8,
        serde: SerdeExample {
            inner_a: 1,
            inner_b: "Inner".to_string(),
        },
    };

    // Note: uses into instead of try_into because of datex(serde_infallible)
    let value_container: ValueContainer = serde_example.into();

    let map: Map = Map::try_cast_from_value_container(value_container).unwrap();
    assert_eq!(map.try_get("a").unwrap(), &ValueContainer::from(42u8));
    let serde_map: Map = map
        .try_get("serde")
        .unwrap()
        .clone()
        .try_into_value()
        .unwrap();
    assert_eq!(
        serde_map.try_get("inner_a").unwrap(),
        &ValueContainer::from(1u8)
    );
    assert_eq!(
        serde_map.try_get("inner_b").unwrap(),
        &ValueContainer::from("Inner".to_string())
    );
}

#[test]
fn struct_with_value_container() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let address_provider = &mut SelfOwnedPointerAddressProvider::default();

    #[derive(Datex, Debug, PartialEq, Clone)]
    #[datex(structural)]
    struct ExampleWithValueContainer {
        a: u8,
        val: ValueContainer,
    }

    // local inner value container
    let example_local = ExampleWithValueContainer {
        a: 42u8,
        val: ValueContainer::from("Test".to_string()),
    };

    let value: Value = example_local.into();
    let map = Map::try_cast_from_value_container(ValueContainer::from(value))
        .unwrap();
    assert_eq!(map.try_get("a").unwrap(), &ValueContainer::from(42u8));
    assert_eq!(
        map.try_get("val").unwrap(),
        &ValueContainer::from("Test".to_string())
    );

    // shared inner value container
    let shared_container = ValueContainer::Shared(
        SharedContainer::new_owned_with_inferred_allowed_type(
            42,
            SharedContainerMutability::Mutable,
            address_provider,
        ),
    );
    let example_shared = ExampleWithValueContainer {
        a: 42u8,
        val: shared_container.clone(),
    };

    let value_container: ValueContainer = example_shared.clone().into();
    let value_container_2: ValueContainer = example_shared.into();

    let map_2 = Map::try_cast_from_value_container(value_container).unwrap();
    assert_eq!(map_2.try_get("a").unwrap(), &ValueContainer::from(42u8));
    assert_eq!(map_2.try_get("val").unwrap(), &shared_container);

    let deserialized_example_shared =
        ExampleWithValueContainer::try_cast_from_value_container(
            value_container_2,
        )
        .unwrap();
    assert_eq!(deserialized_example_shared.a, 42u8);
    assert_eq!(deserialized_example_shared.val, shared_container);
}

#[test]
fn struct_with_owned_shared_value_container() {
    let cache = RefCell::new(SharedReferencesCache::default());
    let address_provider = &mut SelfOwnedPointerAddressProvider::default();

    #[derive(Datex, Debug, PartialEq)]
    #[datex(structural)]
    struct ExampleWithOwnedContainer {
        owned: OwnedSharedContainer,
    }

    let owned_container = OwnedSharedContainer::new_with_inferred_allowed_type(
        42,
        SharedContainerMutability::Mutable,
        address_provider,
    );
    let address = owned_container.pointer_address().clone();
    let example = ExampleWithOwnedContainer {
        owned: owned_container,
    };

    let value_container: ValueContainer = example.into();

    let map = Map::try_cast_from_value_container(value_container).unwrap();

    if let ValueContainer::Shared(SharedContainer::Owned(shared_container)) =
        map.try_get("owned").unwrap()
    {
        assert_eq!(*shared_container.pointer_address(), address);
    } else {
        panic!("Expected a Shared Owned variant in the ValueContainer");
    }

    // TODO: function mapping, SharedRef<x>, Shared<x>
}

#[test]
fn get_datex_type_from_struct() {
    let dx_type = Example::datex_type(&mut SharedReferencesCache::default());

    assert_eq!(
        dx_type,
        Type::Definition(
            TypeDefinition::Map(MapTypeDefinition(vec![
                (
                    Type::Definition(
                        TypeDefinition::Literal(LiteralTypeDefinition::Text(
                            "a".into()
                        ))
                        .into()
                    ),
                    Type::Definition(
                        TypeDefinition::CoreType(
                            CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )
                            .into()
                        )
                        .into()
                    )
                ),
                (
                    Type::Definition(
                        TypeDefinition::Literal(LiteralTypeDefinition::Text(
                            "b".into()
                        ))
                        .into()
                    ),
                    Type::Definition(
                        TypeDefinition::CoreType(
                            CoreLibBaseTypeId::Text.into()
                        )
                        .into()
                    )
                ),
                (
                    Type::Definition(
                        TypeDefinition::Literal(LiteralTypeDefinition::Text(
                            "c".into()
                        ))
                        .into()
                    ),
                    Type::Definition(
                        TypeDefinition::CoreType(
                            CoreLibBaseTypeId::Endpoint.into()
                        )
                        .into()
                    )
                )
            ]))
            .into()
        )
    )
}

#[test]
fn get_datex_type_from_enum() {
    let dx_type =
        ExampleEnum::datex_type(&mut SharedReferencesCache::default());

    assert_eq!(
        dx_type,
        Type::Definition(
            TypeDefinition::Union(UnionTypeDefinition(vec![
                TypeDefinition::TaggedType(TaggedTypeDefinition {
                    tag: "VariantA".to_string(),
                    ty: None,
                })
                .into(),
                TypeDefinition::TaggedType(TaggedTypeDefinition {
                    tag: "VariantB".to_string(),
                    ty: Some(Box::new(
                        TypeDefinition::List(ListTypeDefinition(vec![
                            Type::Definition(
                                TypeDefinition::CoreType(
                                    CoreLibVariantTypeId::Integer(
                                        IntegerTypeVariant::U8
                                    )
                                    .into()
                                )
                                .into()
                            ),
                            Type::Definition(
                                TypeDefinition::CoreType(
                                    CoreLibVariantTypeId::Integer(
                                        IntegerTypeVariant::U8
                                    )
                                    .into()
                                )
                                .into()
                            ),
                        ]))
                        .into()
                    ))
                })
                .into(),
                TypeDefinition::TaggedType(TaggedTypeDefinition {
                    tag: "VariantC".to_string(),
                    ty: Some(Box::new(
                        TypeDefinition::Map(MapTypeDefinition(vec![
                            (
                                Type::Definition(
                                    TypeDefinition::Literal(
                                        LiteralTypeDefinition::Text("x".into())
                                    )
                                    .into()
                                ),
                                Type::Definition(
                                    TypeDefinition::CoreType(
                                        CoreLibVariantTypeId::Integer(
                                            IntegerTypeVariant::U8
                                        )
                                        .into()
                                    )
                                    .into()
                                )
                            ),
                            (
                                Type::Definition(
                                    TypeDefinition::Literal(
                                        LiteralTypeDefinition::Text("y".into())
                                    )
                                    .into()
                                ),
                                Type::Definition(
                                    TypeDefinition::CoreType(
                                        CoreLibBaseTypeId::Text.into()
                                    )
                                    .into()
                                )
                            )
                        ]))
                        .into()
                    ))
                })
                .into(),
                TypeDefinition::TaggedType(TaggedTypeDefinition {
                    tag: "VariantD".to_string(),
                    ty: Some(Box::new(
                        TypeDefinition::CoreType(
                            CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )
                            .into()
                        )
                        .into()
                    ))
                })
                .into(),
            ]))
            .into()
        )
    );
}

#[test]
fn recursive_struct() {
    #[derive(Datex)]
    struct Node {
        next: Option<Box<Node>>,
    }
    let cache = &mut SharedReferencesCache::default();
    let ty = Node::datex_type(cache);
    ty.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            let next_type = map
                .first()
                .expect("Expected 'next' field in map type definition")
                .1
                .clone();
            next_type.with_collapsed_type_definition(|next_ty_def| {
                next_ty_def
                    .try_unbox()
                    .expect("Expected Option type for 'next' field")
                    .with_collapsed_type_definition(|ty_def| match &ty_def {
                        TypeDefinition::Union(union_ty_def) => {
                            union_ty_def.0 == vec![Type::NULL, ty.clone()]
                        }
                        _ => panic!(
                            "Expected Union type for Option, got {:?}",
                            next_ty_def
                        ),
                    });
            })
        }
        _ => panic!("Expected map type definition"),
    });
}

#[test]
fn mutual_recursion_structural_containing_entity() {
    #[derive(Datex)]
    #[datex(structural)]
    struct A {
        b: Box<B>,
    }

    #[derive(Datex)]
    struct B {
        a: Box<A>,
    }
    let cache = &mut SharedReferencesCache::default();

    let ty_a = A::datex_type(cache);
    let ty_b = B::datex_type(cache);

    ty_a.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            assert_eq!(map.first().expect("wtf").1, ty_b);
        }
        _ => panic!("Expected map type definition for A"),
    });
    ty_b.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            assert_eq!(map.first().expect("wtf").1, ty_a);
        }
        _ => panic!("Expected map type definition for B"),
    });
}

#[test]
fn mutual_recursion_entity_containing_structural() {
    #[derive(Datex)]
    struct A {
        b: Box<B>,
    }

    #[derive(Datex)]
    #[datex(structural)]
    struct B {
        a: Box<A>,
    }
    let cache = &mut SharedReferencesCache::default();

    let ty_a = A::datex_type(cache);
    let ty_b = B::datex_type(cache);

    ty_a.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            assert_eq!(map.first().expect("wtf").1, ty_b);
        }
        _ => panic!("Expected map type definition for A"),
    });
    ty_b.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            assert_eq!(map.first().expect("wtf").1, ty_a);
        }
        _ => panic!("Expected map type definition for B"),
    });
}

#[test]
fn mutual_recursion_entity() {
    #[derive(Datex)]
    struct A {
        b: Box<B>,
    }

    #[derive(Datex)]
    struct B {
        a: Box<A>,
    }
    let cache = &mut SharedReferencesCache::default();

    let ty_a = A::datex_type(cache);
    let ty_b = B::datex_type(cache);

    ty_a.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            assert_eq!(map.first().expect("wtf").1, ty_b);
        }
        _ => panic!("Expected map type definition for A"),
    });
    ty_b.with_collapsed_type_definition(|ty_def| match ty_def {
        TypeDefinition::Map(map) => {
            assert_eq!(map.first().expect("wtf").1, ty_a);
        }
        _ => panic!("Expected map type definition for B"),
    });
}

#[test]
#[should_panic(expected = "Can not use recursive structural")]
fn mutual_recursion_panic_with_structural() {
    #[derive(Datex)]
    #[datex(structural)]
    struct A {
        b: Box<B>,
    }
    #[derive(Datex)]
    #[datex(structural)]
    struct B {
        a: Box<A>,
    }
    let cache = &mut SharedReferencesCache::default();
    let _ = A::datex_type(cache);
    let _ = B::datex_type(cache);
}

#[test_case(None ; "none")]
#[test_case(Some(0u8) ; "some zero")]
#[test_case(Some(u8::MAX) ; "some max")]
fn round_trip_option(value: Option<u8>) {
    assert_round_trip(value);
}

#[test_case(None ; "outer none")]
#[test_case(Some(None) ; "inner none")]
#[test_case(Some(Some(42u8)) ; "some value")]
fn round_trip_nested_option(value: Option<Option<u8>>) {
    assert_round_trip(value);
}

#[test_case(Box::new(0u8) ; "boxed zero")]
#[test_case(Box::new(42u8) ; "boxed primitive")]
#[test_case(Box::new(u8::MAX) ; "boxed max")]
fn round_trip_box(value: Box<u8>) {
    assert_round_trip(value);
}

#[test_case(None ; "none")]
#[test_case(Some(Box::new(0u8)) ; "some boxed zero")]
#[test_case(Some(Box::new(42u8)) ; "some boxed value")]
fn round_trip_option_box(value: Option<Box<u8>>) {
    assert_round_trip(value);
}

#[test_case(Box::new(None) ; "boxed none")]
#[test_case(Box::new(Some(0u8)) ; "boxed some zero")]
#[test_case(Box::new(Some(42u8)) ; "boxed some value")]
fn round_trip_box_option(value: Box<Option<u8>>) {
    assert_round_trip(value);
}

#[test_case(None ; "outer none")]
#[test_case(Some(Box::new(None)) ; "boxed none")]
#[test_case(Some(Box::new(Some(42u8))) ; "boxed some value")]
fn round_trip_option_box_option(value: Option<Box<Option<u8>>>) {
    assert_round_trip(value);
}

#[test]
fn round_trip_boxed_struct() {
    let example = Box::new(Example {
        a: 42u8,
        b: "Test".to_string(),
        c: Endpoint::default(),
    });
    assert_round_trip(example);
}

#[test]
fn struct_with_option() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct ExampleWithOption {
        value: Option<u8>,
    }
    assert_round_trip(ExampleWithOption { value: None });
    assert_round_trip(ExampleWithOption { value: Some(42) });
}

#[test]
fn struct_with_box() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct ExampleWithBox {
        value: Box<u8>,
    }
    assert_round_trip(ExampleWithBox {
        value: Box::new(42),
    });
}

#[test]
fn struct_with_option_box() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct ExampleWithOptionBox {
        value: Option<Box<u8>>,
    }
    assert_round_trip(ExampleWithOptionBox { value: None });
    assert_round_trip(ExampleWithOptionBox {
        value: Some(Box::new(42)),
    });
}

#[test]
fn struct_with_box_option() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct ExampleWithBoxOption {
        value: Box<Option<u8>>,
    }
    assert_round_trip(ExampleWithBoxOption {
        value: Box::new(None),
    });
    assert_round_trip(ExampleWithBoxOption {
        value: Box::new(Some(42)),
    });
}

#[test]
fn struct_with_nested_option() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct ExampleWithNestedOption {
        value: Option<Option<u8>>,
    }
    assert_round_trip(ExampleWithNestedOption { value: None });
    assert_round_trip(ExampleWithNestedOption { value: Some(None) });
    assert_round_trip(ExampleWithNestedOption {
        value: Some(Some(42)),
    });
}

#[test_case(
    ExampleEnumWithOptionAndBox::Optional(None)
    ; "option none"
)]
#[test_case(
    ExampleEnumWithOptionAndBox::Optional(Some(42))
    ; "option some"
)]
#[test_case(
    ExampleEnumWithOptionAndBox::Boxed(Box::new(42))
    ; "boxed"
)]
#[test_case(
    ExampleEnumWithOptionAndBox::OptionalBoxed(None)
    ; "option boxed none"
)]
#[test_case(
    ExampleEnumWithOptionAndBox::OptionalBoxed(Some(Box::new(42)))
    ; "option boxed some"
)]
fn round_trip_enum_with_option_and_box(value: ExampleEnumWithOptionAndBox) {
    assert_round_trip(value);
}

#[derive(Datex, Debug, Clone, PartialEq)]
#[datex(structural)]
enum ExampleEnumWithOptionAndBox {
    Optional(Option<u8>),
    Boxed(Box<u8>),
    OptionalBoxed(Option<Box<u8>>),
}

#[test]
fn recursive_struct_round_trip() {
    #[derive(Datex, Debug, Clone, PartialEq)]
    #[datex(structural)]
    struct Node {
        value: u8,
        next: Option<Box<Node>>,
    }

    let node = Node {
        value: 42,
        next: Some(Box::new(Node {
            value: 69,
            next: Some(Box::new(Node {
                value: 10,
                next: None,
            })),
        })),
    };
    assert_round_trip(node);
}

#[test]
fn vec_with_option_box() {
    let value = vec![None, Some(Box::new(0u8)), Some(Box::new(42u8)), None];
    assert_round_trip(value);
}
