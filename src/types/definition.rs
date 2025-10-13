use crate::{
    traits::structural_eq::StructuralEq,
    types::{
        collection_type_definition::CollectionTypeDefinition,
        structural_type_definition::StructuralTypeDefinition,
        type_container::TypeContainer,
    },
};
use datex_core::references::type_reference::TypeReference;
use std::{cell::RefCell, fmt::Display, hash::Hash, rc::Rc};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDefinition {
    // { x: integer, y: text }
    Structural(StructuralTypeDefinition),

    // TODO #371: Rename to generic?
    // e.g. [integer], [integer; 5], Map<string, integer>
    Collection(CollectionTypeDefinition),

    // type A = B
    Reference(Rc<RefCell<TypeReference>>),

    // A & B & C
    Intersection(Vec<TypeContainer>),

    // A | B | C
    Union(Vec<TypeContainer>),

    // ()
    Unit,

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
            TypeDefinition::Unit => 0_u8.hash(state),
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeDefinition::Collection(value) => write!(f, "{}", value),
            TypeDefinition::Structural(value) => write!(f, "{}", value),
            TypeDefinition::Reference(reference) => {
                write!(f, "{}", reference.borrow())
            }
            TypeDefinition::Unit => write!(f, "()"),
            TypeDefinition::Union(types) => {
                let is_level_zero = types.iter().all(|t| {
                    matches!(
                        t.as_type().type_definition,
                        TypeDefinition::Structural(_)
                            | TypeDefinition::Reference(_)
                    )
                });
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                if is_level_zero {
                    write!(f, "{}", types_str.join(" | "))
                } else {
                    write!(f, "({})", types_str.join(" | "))
                }
            }
            TypeDefinition::Intersection(types) => {
                let types_str: Vec<String> =
                    types.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", types_str.join(" & "))
            }
            TypeDefinition::Function {
                parameters,
                return_type,
            } => {
                let params_str: Vec<String> = parameters
                    .iter()
                    .map(|(name, ty)| format!("{}: {}", name, ty))
                    .collect();
                write!(f, "({}) -> {}", params_str.join(", "), return_type)
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
