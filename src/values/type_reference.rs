use std::fmt::{Display, Formatter};
use crate::values::core_values::r#type::r#type::Type;
use crate::values::pointer::PointerAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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