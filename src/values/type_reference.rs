use serde::{Deserialize, Serialize};

use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
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
