use crate::dif::DIFConvertible;
use crate::dif::representation::DIFTypeRepresentation;
use crate::references::reference::Reference;
use crate::references::reference::ReferenceMutability;
use crate::references::reference::mutability_option_as_int;
use crate::runtime::memory::Memory;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::pointer::PointerAddress;
use serde::{Deserialize, Serialize};
use core::cell::RefCell;
use crate::values::core_values::r#type::Type;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "def", rename_all = "lowercase")]
pub enum DIFTypeDefinition {
    // {x: integer, y: text}
    Structural(Box<DIFStructuralTypeDefinition>),

    Reference(PointerAddress),
    Type(Box<DIFType>),

    // e.g. A & B & C
    Intersection(Vec<DIFTypeContainer>),

    // e.g. A | B | C
    Union(Vec<DIFTypeContainer>),

    // ()
    Unit,

    Never,

    Unknown,

    Function {
        parameters: Vec<(String, DIFTypeContainer)>,
        return_type: Box<DIFTypeContainer>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFStructuralTypeDefinition {
    pub value: DIFTypeRepresentation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<DIFTypeContainer>,
}

impl DIFStructuralTypeDefinition {
    fn from_structural_definition(
        struct_def: &StructuralTypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        let value = DIFTypeRepresentation::from_structural_type_definition(
            struct_def, memory,
        );
        let type_def =
            PointerAddress::from(struct_def.get_core_lib_type_pointer_id());
        DIFStructuralTypeDefinition {
            value,
            r#type: Some(DIFTypeContainer::Reference(type_def)),
        }
    }
}

impl DIFTypeDefinition {
    fn from_type_definition(
        type_def: &TypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        match type_def {
            TypeDefinition::Collection(collection_def) => {
                core::todo!("#387 handle collection type conversion");
            }
            TypeDefinition::Structural(struct_def) => {
                DIFTypeDefinition::Structural(Box::new(
                    DIFStructuralTypeDefinition::from_structural_definition(
                        struct_def, memory,
                    ),
                ))
            }
            TypeDefinition::Reference(type_ref) => {
                DIFTypeDefinition::Reference(
                    type_ref.borrow().pointer_address.clone().unwrap(),
                )
            }
            TypeDefinition::Type(type_val) => {
                DIFTypeDefinition::Type(Box::new(DIFType::from_type(type_val.as_ref(), memory)))
            }
            TypeDefinition::Intersection(types) => {
                DIFTypeDefinition::Intersection(
                    types
                        .iter()
                        .map(|t| {
                            DIFTypeContainer::from_type_container(t, memory)
                        })
                        .collect(),
                )
            }
            TypeDefinition::Union(types) => DIFTypeDefinition::Union(
                types
                    .iter()
                    .map(|t| DIFTypeContainer::from_type_container(t, memory))
                    .collect(),
            ),
            TypeDefinition::Unit => DIFTypeDefinition::Unit,
            TypeDefinition::Never => DIFTypeDefinition::Never,
            TypeDefinition::Unknown => DIFTypeDefinition::Unknown,
            TypeDefinition::Function {
                parameters,
                return_type,
            } => DIFTypeDefinition::Function {
                parameters: parameters
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            DIFTypeContainer::from_type_container(ty, memory),
                        )
                    })
                    .collect(),
                return_type: Box::new(DIFTypeContainer::from_type_container(
                    return_type,
                    memory,
                )),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIFTypeContainer {
    Type(DIFType),
    Reference(PointerAddress),
}

impl DIFConvertible for DIFTypeContainer {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mut")]
    #[serde(default)]
    #[serde(with = "mutability_option_as_int")]
    pub mutability: Option<ReferenceMutability>,

    #[serde(flatten)]
    pub type_definition: DIFTypeDefinition,
}
impl DIFConvertible for DIFType {}

impl DIFType {
    pub fn as_container(self) -> DIFTypeContainer {
        DIFTypeContainer::Type(self)
    }

    fn from_type(ty: &Type, memory: &RefCell<Memory>) -> Self {
        DIFType {
            name: None,
            mutability: ty.reference_mutability.clone(),
            type_definition: DIFTypeDefinition::from_type_definition(
                &ty.type_definition,
                memory,
            ),
        }
    }
}

impl From<DIFTypeRepresentation> for DIFType {
    fn from(value: DIFTypeRepresentation) -> Self {
        DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value,
                    r#type: None,
                },
            )),
        }
    }
}

impl DIFTypeContainer {
    pub fn from_type_container(
        type_container: &TypeContainer,
        memory: &RefCell<Memory>,
    ) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFTypeContainer::Type(DIFType::from_type(ty, memory)),
            TypeContainer::TypeReference(type_ref) => {
                let type_ref_borrow = type_ref.borrow();
                let address = if let Some(ref address) =
                    type_ref_borrow.pointer_address
                {
                    address
                } else {
                    &memory.borrow_mut().register_reference(
                        &Reference::TypeReference(type_ref.clone()),
                    )
                };
                DIFTypeContainer::Reference(address.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dif_type_serialization() {
        let dif_type = DIFType {
            name: Some("ExampleType".to_string()),
            mutability: Some(ReferenceMutability::Mutable),
            type_definition: DIFTypeDefinition::Unit,
        };
        let serialized = dif_type.as_json();
        println!("Serialized DIFType: {}", serialized);
        let deserialized = DIFType::from_json(&serialized);
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn object() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Object(vec![
                        (
                            "field1".to_string(),
                            DIFType::from(DIFTypeRepresentation::Null)
                                .as_container(),
                        ),
                        (
                            "field2".to_string(),
                            DIFType::from(DIFTypeRepresentation::Number(42.0))
                                .as_container(),
                        ),
                    ]),
                    r#type: None,
                },
            )),
        };
        let serialized = dif_type.as_json();
        let deserialized: DIFType = DIFType::from_json(&serialized);
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn map() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Map(vec![
                        (
                            DIFType::from(DIFTypeRepresentation::String(
                                "key1".to_string(),
                            ))
                            .as_container(),
                            DIFType::from(DIFTypeRepresentation::Number(42.0))
                                .as_container(),
                        ),
                        (
                            DIFType::from(DIFTypeRepresentation::Number(1.0))
                                .as_container(),
                            DIFType::from(DIFTypeRepresentation::Number(3.5))
                                .as_container(),
                        ),
                    ]),
                    r#type: None,
                },
            )),
        };
        let serialized = dif_type.as_json();
        let deserialized: DIFType = DIFType::from_json(&serialized);
        assert_eq!(dif_type, deserialized);
    }

    #[test]
    fn array() {
        let dif_type = DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition {
                    value: DIFTypeRepresentation::Array(vec![
                        DIFType::from(DIFTypeRepresentation::Number(1.0))
                            .as_container(),
                        DIFType::from(DIFTypeRepresentation::Number(2.0))
                            .as_container(),
                        DIFType::from(DIFTypeRepresentation::Number(3.0))
                            .as_container(),
                    ]),
                    r#type: None,
                },
            )),
        };
        let serialized = dif_type.as_json();
        println!("Serialized DIFType: {}", serialized);
        let deserialized: DIFType = DIFType::from_json(&serialized);
        println!("Deserialized DIFType: {:#?}", deserialized);
        assert_eq!(dif_type, deserialized);
    }
}
