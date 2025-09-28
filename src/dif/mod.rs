use serde::{Deserialize, Serialize};

use crate::dif::value::DIFValueContainer;
pub mod dif_representation;
pub mod interface;
pub mod r#type;
pub mod value;
pub mod reference;

/// Represents a property in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DIFProperty {
    /// a simple string property
    Text(String),
    /// an integer property (e.g. an array index)
    Integer(i64),
    /// any other property type
    Value(DIFValueContainer),
}

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DIFUpdate {
    Replace {
        value: DIFValueContainer,
    },
    UpdateProperty {
        property: DIFProperty,
        value: DIFValueContainer,
    },
    Push {
        value: DIFValueContainer,
    },
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use datex_core::dif::r#type::DIFTypeDefinition;
    use datex_core::values::core_values::endpoint::Endpoint;
    use crate::{
        dif::{
            dif_representation::DIFValueRepresentation,
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
    use crate::runtime::memory::Memory;
    use super::*;

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
        let dif_update = DIFUpdate::Replace {
            value: DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::String("Hello".to_string()),
                r#type: None,
            }),
        };
        let serialized = serde_json::to_string(&dif_update).unwrap();
        println!("Serialized DIFUpdate: {}", serialized);
        let deserialized: DIFUpdate =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_update, deserialized);

        // update property
        let dif_update = DIFUpdate::UpdateProperty {
            property: DIFProperty::Text("name".to_string()),
            value: DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::Number(42.0),
                r#type: None,
            }),
        };
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
