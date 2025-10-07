use serde::{Deserialize, Serialize};

use crate::types::type_container::TypeContainer;
use crate::values::pointer::PointerAddress;
use crate::{
    types::definition::TypeDefinition, values::core_values::r#type::Type,
};
use std::{
    cell::RefCell,
    fmt::{Display, Formatter},
    rc::Rc,
};
use log::info;
use crate::libs::core::CoreLibPointerId;
use crate::runtime::execution::ExecutionError;
use crate::traits::apply::Apply;
use crate::values::value_container::ValueContainer;

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

    pub fn collapse_reference_chain(&self) -> TypeReference {
        match &self.type_value.type_definition {
            TypeDefinition::Reference(reference) => {
                // If this is a reference type, resolve it to its current reference
                reference.borrow().collapse_reference_chain()
            }
            _ => {
                // If this is not a reference type, return it directly
                self.clone()
            }
        }
    }
}

impl TypeReference {
    pub fn as_type(&self) -> &Type {
        &self.type_value
    }

    pub fn base_type(&self) -> Option<Rc<RefCell<TypeReference>>> {
        self.type_value.base_type()
    }

    pub fn matches_reference(&self, other: Rc<RefCell<TypeReference>>) -> bool {
        todo!("implement type matching");
    }

    pub fn matches_type(&self, other: &Type) -> bool {
        println!("Other {:?}", other.base_type());
        println!("Matching type {:?} against type {}", self, other);

        if let Some(base) = other.base_type() {
            return *self == *base.borrow();
        }

        todo!("implement type matching");
    }
}

impl Apply for TypeReference {
    fn apply(&self, args: &[ValueContainer]) -> Result<Option<ValueContainer>, ExecutionError> {
        todo!()
    }

    fn apply_single(&self, arg: &ValueContainer) -> Result<Option<ValueContainer>, ExecutionError> {
        // TODO: ensure that we can guarantee that pointer_address is always Some here
        let core_lib_id = CoreLibPointerId::try_from(self.pointer_address.as_ref().unwrap());
        if let Ok(core_lib_id) = core_lib_id {
            match core_lib_id {
                CoreLibPointerId::Integer(None) => {
                    arg.to_value().borrow().cast_to_integer()
                        .map(|i| Some(ValueContainer::from(i)))
                        .ok_or_else(|| ExecutionError::InvalidTypeCast)
                }
                CoreLibPointerId::Integer(Some(variant)) => {
                    arg.to_value().borrow().cast_to_typed_integer(variant)
                        .map(|i| Some(ValueContainer::from(i)))
                        .ok_or_else(|| ExecutionError::InvalidTypeCast)
                }
                CoreLibPointerId::Decimal(None) => {
                    arg.to_value().borrow().cast_to_decimal()
                        .map(|d| Some(ValueContainer::from(d)))
                        .ok_or_else(|| ExecutionError::InvalidTypeCast)
                }
                CoreLibPointerId::Decimal(Some(variant)) => {
                    arg.to_value().borrow().cast_to_typed_decimal(variant)
                        .map(|d| Some(ValueContainer::from(d)))
                        .ok_or_else(|| ExecutionError::InvalidTypeCast)
                }
                _ => todo!()
            }
        }
        else {
            todo!()
        }
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
