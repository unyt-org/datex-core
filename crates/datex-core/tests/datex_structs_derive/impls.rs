use core::{cell::Ref, ops::DerefMut};
use datex_core::{
    datex_registry::{
        all_datex_module_registrations, all_datex_type_registrations,
        get_all_modules,
    },
    libs::core::type_id::{
        CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId,
    },
    runtime::{Runtime, cache::shared_references_cache::SharedReferencesCache},
    shared_values::{
        SelfOwnedPointerAddress, SharedContainer, SharedContainerMutability,
    },
    traits::{
        apply::{Apply, ApplyArgument},
        get_datex_type::GetDatexType,
        try_clone::TryClone,
        value_access::ValueAccess,
    },
    types::{
        entities::{
            entity_impls::{EntityImpl, EntityImplMethod},
            entity_type_definition::EntityTypeDefinition,
        },
        entity_type::EntityType,
        literal_type_definition::LiteralTypeDefinition,
        r#type::Type,
        type_definition::{
            TypeDefinition,
            callable::{CallableKind, CallableTypeDefinition},
        },
    },
    values::{
        core_value::CoreValue,
        core_values::{
            callable::Callable,
            integer::typed_integer::{IntegerTypeVariant, TypedInteger},
            native::{DatexNative, NativeCoreValue},
        },
        value::{Value, value_classification::ValueClassification},
        value_container::ValueContainer,
    },
};
use datex_macros_internal::{Datex, datex};

#[derive(Datex, Debug, Clone, PartialEq)]
struct Example {
    a: u8,
    b: u8,
}
#[datex]
impl Example {
    pub fn new(a: u8, b: u8) -> Self {
        Example { a, b }
    }

    pub fn set_a(&mut self, a: u8) -> u8 {
        self.a = a;
        self.a
    }
    pub fn set_b(&mut self, b: u8) {
        self.b = b;
    }

    /// An example of a static method that adds two u8s together.
    pub fn add(a: u8, b: u8) -> u8 {
        a + b
    }

    pub async fn async_test(&self, a: String) -> String {
        format!("a = {}", a)
    }
}

/// Helper function to extract the [EntityTypeDefinition] from a given [Type].
fn entity_type_definition_from_type(
    ty: &Type,
) -> Ref<'_, EntityTypeDefinition> {
    match ty {
        Type::Entity(entity) => entity.entity_definition(),
        _ => panic!("Expected entity type, got {:?}", ty),
    }
}

#[test]
fn take_from_cache() {
    let runtime = Runtime::stub();
    let mut cache = runtime.shared_references_cache_mut();
    let example_type = Example::datex_type(cache.deref_mut());

    // when calling the datex_type function multiple times, it should return the same type definition from cache
    assert_eq!(example_type, Example::datex_type(cache.deref_mut()));
}

#[test]
fn try_clone() {
    // validate that try clone works since Example implements Clone
    let example = Example::new(1, 2);
    example.try_clone().unwrap();

    42u8.try_clone().unwrap();

    // rust core types that implement Clone should also be able to be cloned via try_clone
    let u8_value = Value::native(42u8);
    u8_value.try_clone().unwrap();
}

#[test]
fn call_instance_method_from_runtime() {
    let runtime = Runtime::stub();
    let mut cache = runtime.shared_references_cache_refcell();

    let example = Example::new(1, 2);
    let example_vc = Value::native(example);

    let example_type = Example::datex_type(cache.borrow_mut().deref_mut());
    let set_a = example_type.try_get_property("set_a".into()).unwrap();
    let set_a_callable = set_a.try_as::<Callable>().unwrap();

    let (res, mut borrows) = set_a_callable
        .try_apply_sync_checked(
            &runtime,
            vec![
                ApplyArgument::referenced(example_vc),
                TypedInteger::U8(10).into(),
            ],
        )
        .unwrap();

    let res = res.unwrap();
    let result_u8 = res.try_as::<u8>().unwrap();
    assert_eq!(result_u8, &10u8);
    // borrows should contain the original example value
    assert_eq!(
        borrows.remove(0),
        // a was updated to 10
        ValueContainer::from(Value::native(Example { a: 10, b: 2 },))
    );
}

#[tokio::test]
async fn call_async_instance_method_from_runtime() {
    let runtime = Runtime::stub();

    let example = Example::new(1, 2);
    let example_vc = Value::native(example);

    let example_type =
        Example::datex_type(runtime.shared_references_cache_mut().deref_mut());
    let async_test =
        example_type.try_get_property("async_test".into()).unwrap();
    let async_test_callable = async_test.try_as::<Callable>().unwrap();

    let (res, mut borrows) = async_test_callable
        .try_apply_async_checked(
            &runtime,
            vec![
                ApplyArgument::referenced(example_vc),
                "test".to_string().into(),
            ],
        )
        .await
        .unwrap();

    let res = res.unwrap();
    let result_string = res.try_as::<String>().unwrap();
    assert_eq!(result_string, "a = test");
    // borrows should contain the original example value
    assert_eq!(
        borrows.remove(0),
        ValueContainer::Local(Value::native(Example::new(1, 2)))
    );
}

#[test]
fn signatures() {
    let runtime = Runtime::stub();
    let mut cache = runtime.shared_references_cache_mut();
    let example_type = Example::datex_type(cache.deref_mut());
    let type_definition = entity_type_definition_from_type(&example_type);

    // call static method by manually extracting it from the type definition
    match &example_type {
        Type::Entity(entity) => {
            let definition = entity.entity_definition();
            let static_add_method = &definition
                .try_get_property("add")
                .expect("Static add method not found");

            // call the static add method
            let result = static_add_method
                .try_apply_sync(
                    &Runtime::stub(),
                    vec![
                        TypedInteger::U8(1).into(),
                        TypedInteger::U8(2).into(),
                    ],
                )
                .unwrap()
                .0
                .unwrap();
            assert_eq!(result, ValueContainer::Local(Value::native(3u8)));
        }
        _ => {
            panic!("Expected entity type, got {:?}", example_type);
        }
    }

    // call method directly on an Example instance via the ValueContainer
    let example_instance = Box::new(Example { a: 1, b: 2 });
    let example_instance_vc =
        ValueContainer::from(Value::native(example_instance));
    // TODO: also store type in value container (this will require passing cache to the ValueContainer::from function somehow)
    // Then we can access methods on the type definition here

    {
        // set_a
        let set_a_sig = &type_definition
            .try_get_property("set_a")
            .expect("set_a method not found")
            .signature;
        assert_eq!(
            set_a_sig,
            &CallableTypeDefinition {
                kind: CallableKind::Procedure,
                requires_async: false,
                parameters: vec![
                    (Some("self".to_string()), example_type.clone(),),
                    (
                        Some("a".to_string()),
                        Type::core(CoreLibTypeId::Variant(
                            CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )
                        ))
                    )
                ],
                rest_parameter: None,
                return_type: Some(Box::new(Type::core(
                    CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(
                        IntegerTypeVariant::U8
                    ))
                ))),
                yeet_type: None,
            }
        );
    }
    {
        // set_b
        let set_b_sig = &type_definition
            .try_get_property("set_b")
            .expect("set_b method not found")
            .signature;
        assert_eq!(
            set_b_sig,
            &CallableTypeDefinition {
                kind: CallableKind::Procedure,
                requires_async: false,
                parameters: vec![
                    (Some("self".to_string()), example_type.clone(),),
                    (
                        Some("b".to_string()),
                        Type::core(CoreLibTypeId::Variant(
                            CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )
                        ))
                    )
                ],
                rest_parameter: None,
                return_type: None,
                yeet_type: None,
            }
        );
    }
    {
        // new
        let new_sig = &type_definition
            .try_get_property("new")
            .expect("new method not found")
            .signature;
        assert_eq!(
            new_sig,
            &CallableTypeDefinition {
                kind: CallableKind::Function,
                requires_async: false,
                parameters: vec![
                    (
                        Some("a".to_string()),
                        Type::core(CoreLibTypeId::Variant(
                            CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )
                        ))
                    ),
                    (
                        Some("b".to_string()),
                        Type::core(CoreLibTypeId::Variant(
                            CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )
                        ))
                    )
                ],
                rest_parameter: None,
                return_type: Some(Box::new(example_type.clone())),
                yeet_type: None,
            }
        );
    }
}
