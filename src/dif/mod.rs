use serde::{Deserialize, Serialize};

use crate::dif::value::DIFValueContainer;
pub mod interface;
pub mod reference;
pub mod representation;
pub mod r#type;
pub mod update;
pub mod value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dif::update::{DIFProperty, DIFUpdate};
    use crate::runtime::memory::Memory;
    use crate::{
        dif::{
            representation::DIFValueRepresentation,
            r#type::{DIFType, DIFTypeContainer},
            value::DIFValue,
        },
        libs::core::CoreLibPointerId,
        types::{
            definition::TypeDefinition,
            structural_type_definition::StructuralTypeDefinition,
        },
        values::{
            core_values::integer::typed_integer::IntegerTypeVariant,
            value_container::ValueContainer,
        },
    };
    use datex_core::dif::r#type::DIFTypeDefinition;
    use datex_core::values::core_values::endpoint::Endpoint;
    use std::cell::RefCell;

    fn dif_value_circle(value_container: ValueContainer) -> DIFValueContainer {
        let memory = RefCell::new(Memory::new(Endpoint::default()));
        let dif_value_container: DIFValueContainer =
            DIFValueContainer::from_value_container(&value_container, &memory);
        let serialized = serde_json::to_string(&dif_value_container).unwrap();
        let deserialized: DIFValueContainer =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_value_container, deserialized);
        dif_value_container
    }

    #[test]
    fn serde() {
        // replace
        let dif_update =
            DIFUpdate::replace(DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::String("Hello".to_string()),
                r#type: None,
            }));
        let serialized = serde_json::to_string(&dif_update).unwrap();
        println!("Serialized DIFUpdate: {}", serialized);
        let deserialized: DIFUpdate =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_update, deserialized);

        // update property
        let dif_update = DIFUpdate::set(
            "name",
            DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::Number(42.0),
                r#type: None,
            }),
        );
        let serialized = serde_json::to_string(&dif_update).unwrap();
        println!("Serialized DIFUpdate: {}", serialized);
        let deserialized: DIFUpdate =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_update, deserialized);
    }

    #[test]
    fn dif_value_serialization() {
        let value = DIFValue {
            value: DIFValueRepresentation::Null,
            r#type: Some(
                DIFType {
                    mutability: None,
                    name: None,
                    type_definition: DIFTypeDefinition::Unit,
                }
                .as_container(),
            ),
        };
        let serialized = serde_json::to_string(&value).unwrap();
        println!("Serialized DIFValue: {}", serialized);
        let deserialized: DIFValue = serde_json::from_str(&serialized).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn from_value_container_i32() {
        let dif_value_container = dif_value_circle(ValueContainer::from(42i32));
        if let DIFValueContainer::Value(dif_value) = &dif_value_container {
            assert_eq!(dif_value.value, DIFValueRepresentation::Number(42f64));
            assert_eq!(
                dif_value.r#type,
                Some(DIFTypeContainer::Reference(
                    CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32))
                        .into()
                ))
            );
        } else {
            panic!("Expected DIFValueContainer::Value variant");
        }
    }

    #[test]
    fn from_value_container_text() {
        let dif_value_container =
            dif_value_circle(ValueContainer::from("Hello, World!"));
        if let DIFValueContainer::Value(dif_value) = &dif_value_container {
            assert_eq!(
                dif_value.value,
                DIFValueRepresentation::String("Hello, World!".to_string())
            );
            assert_eq!(dif_value.r#type, None);
        } else {
            panic!("Expected DIFValueContainer::Value variant");
        }
    }

    //     #[test]
    //     fn dif_property_serialization() {
    //         let property = DIFProperty::Text("example".to_string());
    //         let serialized = serde_json::to_string(&property).unwrap();
    //         let deserialized: DIFProperty =
    //             serde_json::from_str(&serialized).unwrap();
    //         assert_eq!(property, deserialized);
    //     }
}
