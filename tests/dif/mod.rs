use std::cell::RefCell;

use datex_core::{
    dif::{
        representation::DIFValueRepresentation,
        r#type::{DIFType, DIFTypeDefinition},
        value::{DIFValue, DIFValueContainer},
    },
    libs::core::CoreLibPointerId,
    runtime::memory::Memory,
    types::definition::TypeDefinition,
    values::{
        core_value::CoreValue,
        core_values::{endpoint::Endpoint, r#type::Type},
        pointer::PointerAddress,
        value::Value,
        value_container::ValueContainer,
    },
};

#[test]
fn dif_value_no_type() {
    let val = ValueContainer::Value(Value::null());
    let memory = RefCell::new(Memory::new(Endpoint::default()));
    let dif_val = DIFValueContainer::from_value_container(&val, &memory);
    assert_eq!(
        dif_val,
        DIFValueContainer::Value(DIFValue::new(
            DIFValueRepresentation::Null,
            Option::<DIFTypeDefinition>::None,
        ),)
    );
}

#[test]
fn dif_value_with_type() {
    let val = ValueContainer::Value(Value {
        inner: CoreValue::Null,
        actual_type: Box::new(TypeDefinition::ImplType(
            Box::new(Type::integer()),
            vec![PointerAddress::Local([0, 0, 0, 0, 0])],
        )),
    });

    let memory = RefCell::new(Memory::new(Endpoint::default()));
    let dif_val = DIFValueContainer::from_value_container(&val, &memory);
    assert_eq!(
        dif_val,
        DIFValueContainer::Value(DIFValue {
            value: DIFValueRepresentation::Null,
            ty: Some(DIFTypeDefinition::ImplType(
                Box::new(DIFType {
                    name: None,
                    mutability: None,
                    type_definition: DIFTypeDefinition::Reference(
                        PointerAddress::from(CoreLibPointerId::Integer(None))
                    )
                }),
                vec![PointerAddress::Local([0, 0, 0, 0, 0])]
            ))
        })
    );
}
