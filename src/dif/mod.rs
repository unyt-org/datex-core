use crate::dif::value::DIFValue;
pub mod core_value;
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
    Value(DIFValue),
}

/// Represents an update operation for a DIF value.
#[derive(Clone, Debug, PartialEq)]
pub enum DIFUpdate {
    Replace(DIFValue),
    UpdateProperty {
        property: DIFProperty,
        value: DIFValue,
    },
    Push(DIFValue),
}

#[cfg(test)]
mod tests {
    use crate::{
        dif::{core_value::DIFCoreValue, r#type::DIFType},
        types::{
            definition::TypeDefinition,
            structural_type_definition::StructuralTypeDefinition,
        },
    };

    use super::*;

    #[test]
    fn dif_value_serialization() {
        let value = DIFValue {
            value: DIFCoreValue::Null,
            r#type: DIFType {
                mutability: None,
                name: None,
                type_definition: TypeDefinition::Structural(
                    StructuralTypeDefinition::Null,
                ),
            },
        };
        let serialized = serde_json::to_string(&value).unwrap();
        println!("Serialized DIFValue: {}", serialized);
        let deserialized: DIFValue = serde_json::from_str(&serialized).unwrap();
        assert_eq!(value, deserialized);
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

//     #[test]
//     fn from_value_container_i32() {
//         let value_container = ValueContainer::from(42i32);
//         let dif_value: DIFValue = DIFValue::from(&value_container);
//         assert_eq!(dif_value.value, Some(DIFCoreValue::Number(42f64)));
//         // assert_eq!(dif_value.r#type, "i32");
//         assert!(dif_value.ptr_id.is_none());
//         let serialized = serde_json::to_string(&dif_value).unwrap();
//         println!("Serialized DIFValue from int: {}", serialized);
//     }

//     #[test]
//     fn from_value_container_text() {
//         let value_container = ValueContainer::from("Hello, World!");
//         let dif_value: DIFValue = DIFValue::from(&value_container);
//         assert_eq!(
//             dif_value.value,
//             Some(DIFCoreValue::String("Hello, World!".to_string()))
//         );
//         // assert_eq!(dif_value.core_type, CoreValueType::Text);
//         // assert_eq!(dif_value.r#type, "text");
//         assert!(dif_value.ptr_id.is_none());
//     }

//     #[test]
//     fn to_value_container_i32() {
//         let dif_value = DIFValue {
//             value: Some(DIFCoreValue::Number(42f64)),
//             r#type: TypeContainer::Type(Type::structural(
//                 StructuralTypeDefinition::Null,
//             )), // TODO
//             ptr_id: None,
//         };
//         let value_container: ValueContainer = ValueContainer::from(&dif_value);
//         if let ValueContainer::Value(val) = value_container {
//             assert_eq!(
//                 val.inner,
//                 CoreValue::TypedInteger(TypedInteger::I32(42))
//             );
//             // assert_eq!(val.get_type(), CoreValueType::I32);
//         } else {
//             panic!("Expected ValueContainer::Value");
//         }
//     }

//     #[test]
//     fn to_value_container_text() {
//         let dif_value = DIFValue {
//             value: Some(DIFCoreValue::String("Hello, World!".to_string())),
//             r#type: TypeContainer::Type(Type::structural(
//                 StructuralTypeDefinition::Null,
//             )), // TODO
//             ptr_id: None,
//         };
//         let value_container: ValueContainer = ValueContainer::from(&dif_value);
//         if let ValueContainer::Value(val) = value_container {
//             assert_eq!(
//                 val.inner,
//                 CoreValue::Text(Text("Hello, World!".to_string()))
//             );
//             // assert_eq!(val.get_type(), CoreValueType::Text);
//         } else {
//             panic!("Expected ValueContainer::Value");
//         }
//     }
// }
