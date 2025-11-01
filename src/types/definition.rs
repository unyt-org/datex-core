use core::prelude::rust_2024::*;
use crate::{
    traits::structural_eq::StructuralEq,
    types::{
        collection_type_definition::CollectionTypeDefinition,
        structural_type_definition::StructuralTypeDefinition,
        type_container::TypeContainer,
    },
};
use datex_core::references::type_reference::TypeReference;
use crate::stdlib::{cell::RefCell, hash::Hash, rc::Rc};
use crate::values::core_values::r#type::Type;
use core::fmt::Display;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDefinition {
    // { x: integer, y: text }
    Structural(StructuralTypeDefinition),

    // TODO #371: Rename to generic?
    // e.g. [integer], [integer; 5], Map<string, integer>
    Collection(CollectionTypeDefinition),

    // type A = B
    Reference(Rc<RefCell<TypeReference>>),
    
    Type(Box<Type>),

    // A & B & C
    Intersection(Vec<TypeContainer>),

    // A | B | C
    Union(Vec<TypeContainer>),

    // ()
    Unit,

    Never,

    Unknown,

    Function {
        // FIXME #372: Include error type definition
        parameters: Vec<(String, TypeContainer)>,
        return_type: Box<TypeContainer>,
    },
}

impl Hash for TypeDefinition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TypeDefinition::Collection(value) => {
                value.hash(state);
            }
            TypeDefinition::Structural(value) => {
                value.hash(state);
            }
            TypeDefinition::Reference(reference) => {
                reference.borrow().hash(state);
            }
            TypeDefinition::Type(value) => {
                value.hash(state);
            }

            TypeDefinition::Unit => 0_u8.hash(state),
            TypeDefinition::Unknown => 1_u8.hash(state),
            TypeDefinition::Never => 2_u8.hash(state),

            TypeDefinition::Union(types) => {
                for ty in types {
                    ty.hash(state);
                }
            }
            TypeDefinition::Intersection(types) => {
                for ty in types {
                    ty.hash(state);
                }
            }
            TypeDefinition::Function {
                parameters,
                return_type,
            } => {
                for (name, ty) in parameters {
                    name.hash(state);
                    ty.hash(state);
                }
                return_type.hash(state);
            }
        }
    }
}

impl Display for TypeDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TypeDefinition::Collection(value) => core::write!(f, "{}", value),
            TypeDefinition::Structural(value) => core::write!(f, "{}", value),
            TypeDefinition::Reference(reference) => {
                core::write!(f, "{}", reference.borrow())
            }
            TypeDefinition::Type(value) => core::write!(f, "{}", value),
            TypeDefinition::Unit => core::write!(f, "()"),
            TypeDefinition::Unknown => core::write!(f, "unknown"),
            TypeDefinition::Never => core::write!(f, "never"),

            TypeDefinition::Union(types) => {
                let is_level_zero = types.iter().all(|t| {
                    core::matches!(
                        t.as_type().type_definition,
                        TypeDefinition::Structural(_)
                            | TypeDefinition::Reference(_)
                    )
                });
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                if is_level_zero {
                    core::write!(f, "{}", types_str.join(" | "))
                } else {
                    core::write!(f, "({})", types_str.join(" | "))
                }
            }
            TypeDefinition::Intersection(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                core::write!(f, "({})", types_str.join(" & "))
            }
            TypeDefinition::Function {
                parameters,
                return_type,
            } => {
                let params_str: Vec<String> = parameters
                    .iter()
                    .map(|(name, ty)| format!("{}: {}", name, ty))
                    .collect();
                core::write!(f, "({}) -> {}", params_str.join(", "), return_type)
            }
        }
    }
}

impl StructuralEq for TypeDefinition {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeDefinition::Structural(a), TypeDefinition::Structural(b)) => {
                a.structural_eq(b)
            }
            (TypeDefinition::Union(a), TypeDefinition::Union(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (item_a, item_b) in a.iter().zip(b.iter()) {
                    if !item_a.structural_eq(item_b) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}
