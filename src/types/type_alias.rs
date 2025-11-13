use core::fmt::Display;

use crate::{
    references::type_reference::NominalTypeDeclaration,
    types::type_container::TypeContainer,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    pub nominal_type_declaration: NominalTypeDeclaration,
    pub type_container: TypeContainer,
}

impl TypeAlias {
    pub fn new<T: Into<NominalTypeDeclaration>, U: Into<TypeContainer>>(
        nominal_type_declaration: T,
        type_container: U,
    ) -> Self {
        Self {
            nominal_type_declaration: nominal_type_declaration.into(),
            type_container: type_container.into(),
        }
    }
}

impl Display for TypeAlias {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "{}", self.nominal_type_declaration,)
    }
}
