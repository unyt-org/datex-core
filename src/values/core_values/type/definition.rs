use std::fmt::Display;
use serde::{Deserialize, Serialize};
use datex_core::values::type_reference::TypeReference;
use crate::values::{
    core_values::r#type::structural_type_definition::StructuralTypeDefinition,
    traits::structural_eq::StructuralEq,
    type_container::TypeContainer,
};
#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum TypeDefinition {
    // {x: integer, y: text}
    Structural(StructuralTypeDefinition),
    Reference(Box<TypeReference>),

    // e.g. A & B & C
    Intersection(Vec<TypeContainer>),

    // e.g. A | B | C
    Union(Vec<TypeContainer>),
    // ()
    Unit,

    Function {
        parameters: Vec<(String, TypeContainer)>,
        return_type: Box<TypeContainer>,
    },
}
impl Display for TypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeDefinition::Structural(value) => write!(f, "{}", value),
            TypeDefinition::Reference(reference) => {
                write!(f, "{}", reference)
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
