use crate::dif::DIFConvertible;
use crate::dif::representation::DIFTypeRepresentation;
use crate::references::reference::ReferenceMutability;
use crate::references::reference::mutability_option_as_int;
use crate::runtime::memory::Memory;
use crate::stdlib::boxed::Box;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::types::definition::TypeDefinition;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use core::cell::RefCell;
use core::prelude::rust_2024::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "def", rename_all = "kebab-case")]
pub enum DIFTypeDefinition {
    // {x: integer, y: text}
    Structural(Box<DIFStructuralTypeDefinition>),

    Reference(PointerAddress),
    Type(Box<DIFType>),

    // e.g. A & B & C
    Intersection(Vec<DIFType>),

    // e.g. A | B | C
    Union(Vec<DIFType>),

    ImplType(Box<DIFType>, Vec<PointerAddress>),

    // ()
    Unit,

    Never,

    Unknown,

    Function {
        parameters: Vec<(String, DIFType)>,
        return_type: Box<DIFType>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFStructuralTypeDefinition {
    pub value: DIFTypeRepresentation,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub ty: Option<DIFType>,
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
            ty: Some(DIFType {
                type_definition: DIFTypeDefinition::Reference(type_def),
                mutability: None,
                name: None,
            }),
        }
    }
}

impl DIFTypeDefinition {
    pub fn from_type_definition(
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
            TypeDefinition::Type(type_val) => DIFTypeDefinition::Type(
                Box::new(DIFType::from_type(type_val.as_ref(), memory)),
            ),
            TypeDefinition::Intersection(types) => {
                DIFTypeDefinition::Intersection(
                    types
                        .iter()
                        .map(|t| DIFType::from_type(t, memory))
                        .collect(),
                )
            }
            TypeDefinition::Union(types) => DIFTypeDefinition::Union(
                types
                    .iter()
                    .map(|t| DIFType::from_type(t, memory))
                    .collect(),
            ),
            TypeDefinition::ImplType(ty, impls) => DIFTypeDefinition::ImplType(
                Box::new(DIFType::from_type(ty, memory)),
                impls.clone(),
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
                        (name.clone(), DIFType::from_type(ty, memory))
                    })
                    .collect(),
                return_type: Box::new(DIFType::from_type(return_type, memory)),
            },
        }
    }

    fn to_type_definition(&self, memory: &RefCell<Memory>) -> TypeDefinition {
        match self {
            DIFTypeDefinition::Intersection(types) => {
                TypeDefinition::Intersection(
                    types.iter().map(|t| t.to_type(memory)).collect(),
                )
            }
            DIFTypeDefinition::Union(types) => TypeDefinition::Union(
                types.iter().map(|t| t.to_type(memory)).collect(),
            ),
            DIFTypeDefinition::Reference(type_ref_addr) => {
                let type_ref = memory
                    .borrow_mut()
                    .get_type_reference(type_ref_addr)
                    .expect("Reference not found in memory")
                    .clone();
                TypeDefinition::Reference(type_ref)
            }
            DIFTypeDefinition::Type(dif_type) => {
                TypeDefinition::Type(Box::new(dif_type.to_type(memory)))
            }
            DIFTypeDefinition::ImplType(ty, impls) => TypeDefinition::ImplType(
                Box::new(ty.to_type(memory)),
                impls.clone(),
            ),
            DIFTypeDefinition::Unit => TypeDefinition::Unit,
            DIFTypeDefinition::Never => TypeDefinition::Never,
            DIFTypeDefinition::Unknown => TypeDefinition::Unknown,
            _ => {
                core::todo!(
                    "DIFTypeDefinition::to_type_definition for this variant is not implemented yet"
                )
            }
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

    pub(crate) fn from_type(ty: &Type, memory: &RefCell<Memory>) -> Self {
        DIFType {
            name: None,
            mutability: ty.reference_mutability.clone(),
            type_definition: DIFTypeDefinition::from_type_definition(
                &ty.type_definition,
                memory,
            ),
        }
    }

    pub(crate) fn from_type_definition(
        type_def: &TypeDefinition,
        memory: &RefCell<Memory>,
    ) -> Self {
        DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::from_type_definition(
                type_def, memory,
            ),
        }
    }

    pub(crate) fn to_type(&self, memory: &RefCell<Memory>) -> Type {
        Type {
            reference_mutability: self.mutability.clone(),
            type_definition: self.to_type_definition(memory),
            base_type: None,
        }
    }

    pub(crate) fn to_type_definition(
        &self,
        memory: &RefCell<Memory>,
    ) -> TypeDefinition {
        DIFTypeDefinition::to_type_definition(&self.type_definition, memory)
    }
}

impl From<DIFTypeRepresentation> for DIFType {
    fn from(value: DIFTypeRepresentation) -> Self {
        DIFType {
            name: None,
            mutability: None,
            type_definition: DIFTypeDefinition::Structural(Box::new(
                DIFStructuralTypeDefinition { value, ty: None },
            )),
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
                            DIFType::from(DIFTypeRepresentation::Null),
                        ),
                        (
                            "field2".to_string(),
                            DIFType::from(DIFTypeRepresentation::Number(42.0)),
                        ),
                    ]),
                    ty: None,
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
                            )),
                            DIFType::from(DIFTypeRepresentation::Number(42.0)),
                        ),
                        (
                            DIFType::from(DIFTypeRepresentation::Number(1.0)),
                            DIFType::from(DIFTypeRepresentation::Number(3.5)),
                        ),
                    ]),
                    ty: None,
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
                        DIFType::from(DIFTypeRepresentation::Number(1.0)),
                        DIFType::from(DIFTypeRepresentation::Number(2.0)),
                        DIFType::from(DIFTypeRepresentation::Number(3.0)),
                    ]),
                    ty: None,
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
