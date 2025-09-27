use crate::dif::value::{DIFValue, DIFValueContainer};
pub mod dif_representation;
pub mod interface;
pub mod r#type;
pub mod value;

/// Represents a property in the Datex Interface Format (DIF).
#[derive(Clone, Debug, PartialEq)]
pub enum DIFProperty {
    /// a simple string property
    Text(String),
    /// an integer property (e.g. an array index)
    Integer(i64),
    /// any other property type
    Value(DIFValueContainer),
}

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq)]
pub enum DIFUpdate {
    Replace(DIFValueContainer),
    UpdateProperty {
        property: DIFProperty,
        value: DIFValueContainer,
    },
    Push(DIFValueContainer),
}

#[cfg(test)]
mod tests {
    use crate::{
        dif::{
            dif_representation::DIFRepresentationValue,
            r#type::{DIFType, DIFTypeContainer},
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

    use super::*;

    fn dif_value_circle(value_container: ValueContainer) -> DIFValueContainer {
        let dif_value_container: DIFValueContainer =
            DIFValueContainer::try_from(&value_container).unwrap();
        let serialized = serde_json::to_string(&dif_value_container).unwrap();
        let deserialized: DIFValueContainer =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(dif_value_container, deserialized);
        dif_value_container
    }

    #[test]
    fn dif_value_serialization() {
        let value = DIFValue {
            value: DIFRepresentationValue::Null,
            r#type: Some(
                DIFType {
                    mutability: None,
                    name: None,
                    type_definition: TypeDefinition::Structural(
                        StructuralTypeDefinition::Null,
                    )
                    .into(),
                }
                .as_container(),
            ),
            allowed_type: None,
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
            assert_eq!(dif_value.value, DIFRepresentationValue::Number(42f64));
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
                DIFRepresentationValue::String("Hello, World!".to_string())
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
