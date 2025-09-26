use crate::references::reference::ReferenceMutability;
use crate::types::definition::TypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::pointer::PointerAddress;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DIFTypeContainer {
    Type(DIFType),
    Reference(PointerAddress),
}

// impl Serialize for DIFTypeContainer {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         match self {
//             DIFTypeContainer::Type(ty) => ty.serialize(serializer),
//             DIFTypeContainer::Reference(ptr) => ptr.serialize(serializer),
//         }
//     }
// }
// impl<'de> Deserialize<'de> for DIFTypeContainer {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         if let Ok(s) = String::deserialize(&deserializer) {
//             return Ok(DIFTypeContainer::Reference(PointerAddress::from(s)));
//         }
//         let ty = DIFType::deserialize(deserializer)?;
//         Ok(DIFTypeContainer::Type(ty))
//     }
// }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DIFType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability: Option<ReferenceMutability>,
    pub type_definition: TypeDefinition,
}

impl DIFType {
    pub fn as_container(self) -> DIFTypeContainer {
        DIFTypeContainer::Type(self)
    }
}

impl From<TypeContainer> for DIFTypeContainer {
    fn from(type_container: TypeContainer) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFTypeContainer::Type(DIFType {
                name: None,
                mutability: ty.reference_mutability,
                type_definition: ty.type_definition,
            }),
            TypeContainer::TypeReference(type_ref) => {
                DIFTypeContainer::Reference(
                    type_ref.borrow().pointer_address.clone().unwrap(),
                )
            }
        }
    }
}
impl From<TypeContainer> for DIFType {
    fn from(type_container: TypeContainer) -> Self {
        match type_container {
            TypeContainer::Type(ty) => DIFType {
                name: None,
                mutability: ty.reference_mutability,
                type_definition: ty.type_definition,
            },
            TypeContainer::TypeReference(type_ref) => {
                let type_ref = type_ref.borrow().collapse_reference_chain();
                let actual_type = type_ref.type_value;
                let name = type_ref
                    .nominal_type_declaration
                    .as_ref()
                    .map(|decl| decl.name.clone());
                DIFType {
                    name,
                    mutability: actual_type.reference_mutability,
                    type_definition: actual_type.type_definition,
                }
            }
        }
    }
}
