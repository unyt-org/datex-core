use crate::stdlib::string::String;
use serde::{Deserialize, Serialize};

pub mod interface;
pub mod reference;
pub mod representation;
pub mod r#type;
pub mod update;
pub mod value;

pub trait DIFConvertible: Serialize + for<'de> Deserialize<'de> {
    fn to_json(self) -> String {
        self.as_json()
    }
    fn to_json_pretty(self) -> String {
        self.as_json_pretty()
    }
    fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap()
    }
    fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn as_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::dif::DIFConvertible;
    use crate::dif::update::DIFUpdateData;
    use crate::dif::value::DIFValueContainer;
    use crate::runtime::memory::Memory;
    use crate::{
        dif::{representation::DIFValueRepresentation, value::DIFValue},
        libs::core::CoreLibPointerId,
        values::{
            core_values::integer::typed_integer::IntegerTypeVariant,
            value_container::ValueContainer,
        },
    };
    use core::cell::RefCell;
    use datex_core::dif::r#type::DIFTypeDefinition;
    use datex_core::values::core_values::endpoint::Endpoint;

    fn dif_value_circle(value_container: ValueContainer) -> DIFValueContainer {
        let memory = RefCell::new(Memory::new(Endpoint::default()));
        let dif_value_container: DIFValueContainer =
            DIFValueContainer::from_value_container(&value_container, &memory);
        let serialized = dif_value_container.as_json();
        let deserialized: DIFValueContainer =
            DIFValueContainer::from_json(&serialized);
        assert_eq!(dif_value_container, deserialized);
        dif_value_container
    }

    #[test]
    fn serde() {
        // replace
        let dif_update =
            DIFUpdateData::replace(DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::String("Hello".to_string()),
                ty: None,
            }));
        let serialized = dif_update.as_json();
        println!("Serialized DIFUpdate: {}", serialized);
        let deserialized: DIFUpdateData = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);

        // update property
        let dif_update = DIFUpdateData::set(
            "name",
            DIFValueContainer::Value(DIFValue {
                value: DIFValueRepresentation::Number(42.0),
                ty: None,
            }),
        );
        let serialized = dif_update.as_json();
        println!("Serialized DIFUpdate: {}", serialized);
        let deserialized: DIFUpdateData = DIFUpdateData::from_json(&serialized);
        assert_eq!(dif_update, deserialized);
    }

    #[test]
    fn dif_value_serialization() {
        let value = DIFValue {
            value: DIFValueRepresentation::Null,
            ty: Some(DIFTypeDefinition::Unit),
        };
        let serialized = value.as_json();
        println!("Serialized DIFValue: {}", serialized);
        let deserialized = DIFValue::from_json(&serialized);
        assert_eq!(value, deserialized);
    }

    #[test]
    fn from_value_container_i32() {
        let dif_value_container = dif_value_circle(ValueContainer::from(42i32));
        if let DIFValueContainer::Value(dif_value) = &dif_value_container {
            assert_eq!(dif_value.value, DIFValueRepresentation::Number(42f64));
            assert_eq!(
                dif_value.ty,
                Some(DIFTypeDefinition::Reference(
                    CoreLibPointerId::Integer(Some(IntegerTypeVariant::I32))
                        .into()
                ))
            );
        } else {
            core::panic!("Expected DIFValueContainer::Value variant");
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
            assert_eq!(dif_value.ty, None);
        } else {
            core::panic!("Expected DIFValueContainer::Value variant");
        }
    }
}
