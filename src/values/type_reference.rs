use serde::{Deserialize, Serialize};

use crate::values::pointer::PointerAddress;
use crate::values::{core_values::r#type::Type, type_container::TypeContainer};
use std::{
    cell::RefCell,
    fmt::{Display, Formatter},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NominalTypeDeclaration {
    pub name: String,
    pub variant: Option<String>,
}
impl From<String> for NominalTypeDeclaration {
    fn from(name_and_variant: String) -> Self {
        NominalTypeDeclaration::from(name_and_variant.as_str())
    }
}
impl From<&str> for NominalTypeDeclaration {
    fn from(name_and_variant: &str) -> Self {
        let parts: Vec<&str> = name_and_variant.split('/').collect();
        NominalTypeDeclaration {
            name: parts[0].to_string(),
            variant: parts.get(1).map(|&s| s.to_string()),
        }
    }
}

impl Display for NominalTypeDeclaration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(variant) = &self.variant {
            write!(f, "{}/{}", self.name, variant)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeReference {
    /// the value that contains the type declaration
    pub type_value: Type,
    /// optional nominal type declaration
    pub nominal_type_declaration: Option<NominalTypeDeclaration>,
    /// pointer id, can be initialized as None for local pointers
    pub pointer_address: Option<PointerAddress>,
}

impl TypeReference {
    pub fn nominal<T>(
        type_value: Type,
        nominal_type_declaration: T,
        pointer_address: Option<PointerAddress>,
    ) -> Self
    where
        T: Into<NominalTypeDeclaration>,
    {
        TypeReference {
            type_value,
            nominal_type_declaration: Some(nominal_type_declaration.into()),
            pointer_address,
        }
    }
    pub fn anonymous(
        type_value: Type,
        pointer_address: Option<PointerAddress>,
    ) -> Self {
        TypeReference {
            type_value,
            nominal_type_declaration: None,
            pointer_address,
        }
    }
    pub fn as_ref_cell(self) -> Rc<RefCell<TypeReference>> {
        Rc::new(RefCell::new(self))
    }
    pub fn as_type_container(self) -> TypeContainer {
        TypeContainer::TypeReference(self.as_ref_cell())
    }
}

impl TypeReference {
    pub fn as_type(&self) -> &Type {
        &self.type_value
    }

    pub fn base_type(&self) -> Option<Rc<RefCell<TypeReference>>> {
        self.type_value.base_type()
    }
}
impl Display for TypeReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(nominal) = &self.nominal_type_declaration {
            write!(f, "{}", nominal)
        } else {
            write!(f, "{}", self.type_value)
        }
    }
}
