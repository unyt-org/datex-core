use datex_core::{
    compiler::{CompileOptions, compile_template, scope::CompilationScope},
    disassembler::{
        options::DisassemblerOptions, print_disassembled_with_options,
    },
    libs::core::type_id::CoreLibTypeId,
    runtime::{
        Runtime,
        execution::{
            ExecutionInput, ExecutionOptions, execute_dxb_sync,
            execution_input::ExecutionCallerMetadata,
        },
    },
    shared_values::{
        PointerAddress, SharedContainer, SharedContainerMutability,
    },
    types::{
        literal_type_definition::LiteralTypeDefinition,
        shared_container_containing_type::SharedContainerContainingType,
        r#type::Type,
        type_definition::{
            TypeDefinition,
            callable::{CallableKind, CallableTypeDefinition},
            collection::{
                CollectionTypeDefinition,
                type_definition::{
                    list::ListCollectionTypeDefinition,
                    map::MapCollectionTypeDefinition,
                },
            },
            impl_type::ImplMarkers,
            intersection::IntersectionTypeDefinition,
            map::MapTypeDefinition,
            range::RangeTypeDefinition,
            tagged_type::TaggedTypeDefinition,
            union::UnionTypeDefinition,
        },
    },
    values::{
        core_values::{endpoint::Endpoint, integer::Integer},
        value_container::ValueContainer,
    },
};
use test_case::test_case;
use datex_core::compiler::compilation_result::CompilationResult;

#[test_case(LiteralTypeDefinition::Integer(Integer::from(42)))]
#[test_case(LiteralTypeDefinition::TypedInteger(3u8.into()))]
#[test_case(LiteralTypeDefinition::Decimal(1.14.into()))]
#[test_case(LiteralTypeDefinition::TypedDecimal(5.14f64.into()))]
#[test_case(LiteralTypeDefinition::TypedDecimal(6.14f32.into()))]
#[test_case(LiteralTypeDefinition::Text("Hello".to_string().into()))]
#[test_case(LiteralTypeDefinition::Boolean(true.into()))]
#[test_case(LiteralTypeDefinition::Endpoint(Endpoint::new("@+unyt")))]
fn literal(literal: LiteralTypeDefinition) {
    round_trip_type_definition(literal.into(), Runtime::stub());
}

#[test]
fn list() {
    let ty = Type::Definition(
        TypeDefinition::Collection(CollectionTypeDefinition::List(
            ListCollectionTypeDefinition::new(Type::Definition(
                LiteralTypeDefinition::Integer(42.into()).into(),
            )),
        ))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
fn core_type() {
    for id in CoreLibTypeId::iter() {
        round_trip_type_definition(
            TypeDefinition::CoreType(id),
            Runtime::stub(),
        );
    }
}

#[test]
fn tagged() {
    let ty = Type::Definition(
        TypeDefinition::TaggedType(TaggedTypeDefinition {
            tag: "Example".to_string(),
            ty: None,
        })
        .into(),
    );
    round_trip_type(ty, Runtime::stub());

    let ty = Type::Definition(
        TypeDefinition::TaggedType(TaggedTypeDefinition {
            tag: "Example".to_string(),
            ty: Some(Box::new(Type::Definition(
                LiteralTypeDefinition::Integer(42.into()).into(),
            ))),
        })
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
fn union() {
    let ty = Type::Definition(
        TypeDefinition::Union(UnionTypeDefinition::new(vec![
            Type::Definition(LiteralTypeDefinition::Integer(42.into()).into()),
            Type::Definition(
                LiteralTypeDefinition::Text("Hello".to_string().into()).into(),
            ),
        ]))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
fn impl_type() {
    let ty = Type::Definition(
        TypeDefinition::ImplMarkers(ImplMarkers::new(vec![
            PointerAddress::self_owned([1u8, 2u8, 3u8, 4u8, 5u8]),
        ]))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
fn callable() {
    let ty = Type::Definition(
        TypeDefinition::Callable(CallableTypeDefinition {
            kind: CallableKind::Function,
            parameters: vec![(
                Some("test".to_owned()),
                Type::Definition(
                    LiteralTypeDefinition::Integer(Integer::from(42)).into(),
                ),
            )],
            return_type: Some(Box::new(Type::Definition(
                LiteralTypeDefinition::Text("Hello".to_string().into()).into(),
            ))),
            requires_async: false,
            rest_parameter: None,
            yeet_type: None,
        })
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
#[ignore = "TBD if the boxed type actually is preserved in compilation"]
fn boxed() {
    let ty = Type::Definition(
        TypeDefinition::Box(Box::new(Type::Definition(
            LiteralTypeDefinition::Integer(42.into()).into(),
        )))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
#[ignore = "FIXME"] // TODO
fn shared_reference() {
    let runtime = Runtime::stub();
    let ty = Type::Definition(
        TypeDefinition::Shared(unsafe {
            SharedContainerContainingType::new_unchecked(
                SharedContainer::Referenced(
                    SharedContainer::new_owned_with_inferred_allowed_type(
                        ValueContainer::Local(
                            Type::Definition(
                                LiteralTypeDefinition::Integer(42.into())
                                    .into(),
                            )
                            .into(),
                        ),
                        SharedContainerMutability::Immutable,
                        &mut runtime.pointer_address_provider_mut(),
                    )
                    .derive_immutable_reference(),
                ),
            )
        })
        .into(),
    );
    round_trip_type(ty, runtime);
}

#[test_case(CollectionTypeDefinition::List(ListCollectionTypeDefinition::new(
	Type::Definition(LiteralTypeDefinition::Integer(42.into()).into()),
)))]
#[test_case(CollectionTypeDefinition::Map(MapCollectionTypeDefinition::new(
	Type::Definition(LiteralTypeDefinition::Integer(42.into()).into()),
	Type::Definition(LiteralTypeDefinition::Text("Hello".to_string().into()).into()),
)))]
fn collection(collection: CollectionTypeDefinition) {
    round_trip_type_definition(
        TypeDefinition::Collection(collection),
        Runtime::stub(),
    );
}

#[test]
fn intersection() {
    let ty = Type::Definition(
        TypeDefinition::Intersection(IntersectionTypeDefinition::new(vec![
            Type::Definition(LiteralTypeDefinition::Integer(42.into()).into()),
            Type::Definition(
                LiteralTypeDefinition::Text("Hello".to_string().into()).into(),
            ),
        ]))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
fn range() {
    let ty = Type::Definition(
        TypeDefinition::Range(RangeTypeDefinition::new(
            Type::Definition(LiteralTypeDefinition::Integer(42.into()).into()),
            Type::Definition(LiteralTypeDefinition::Integer(100.into()).into()),
        ))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

#[test]
fn map() {
    let ty = Type::Definition(
        TypeDefinition::Map(MapTypeDefinition::new(vec![
            (
                Type::Definition(
                    LiteralTypeDefinition::Integer(42.into()).into(),
                ),
                Type::Definition(
                    LiteralTypeDefinition::Text("Hello".to_string().into())
                        .into(),
                ),
            ),
            (
                Type::Definition(
                    LiteralTypeDefinition::Boolean(true.into()).into(),
                ),
                Type::Definition(
                    LiteralTypeDefinition::Decimal(1.14.into()).into(),
                ),
            ),
        ]))
        .into(),
    );
    round_trip_type(ty, Runtime::stub());
}

fn round_trip_type_definition(ty: TypeDefinition, runtime: Runtime) {
    round_trip_type(Type::Definition(ty.into()), runtime);
}

fn round_trip_type(ty: Type, runtime: Runtime) {
    let CompilationResult {
        dxb_with_shared_values,
        ..
    } = compile_template(
        "?",
        vec![Some(ValueContainer::local(ty.clone()))],
        CompileOptions::new(CompilationScope::default(), vec![Endpoint::LOCAL]),
        runtime.clone(),
    )
    .unwrap();
    print_disassembled_with_options(&dxb_with_shared_values.dxb, DisassemblerOptions::default());
    let result = execute_dxb_sync(ExecutionInput::new(
        dxb_with_shared_values,
        ExecutionCallerMetadata::local_default(),
        ExecutionOptions { verbose: true },
        runtime,
    ))
    .unwrap()
    .unwrap();
    let out: &Type = result.try_as().expect("Result should be a Type");
    assert_eq!(&ty, out);
}
