use std::fmt::Display;

use crate::types::type_container::TypeContainer;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum CollectionTypeDefinition {
    // e.g. [integer]
    List(Box<TypeContainer>),

    // e.g. [integer; 5]
    ArraySlice(Box<TypeContainer>, usize),

    // e.g. {string: integer}
    Map {
        key: Box<TypeContainer>,
        value: Box<TypeContainer>,
    },
}

impl Display for CollectionTypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectionTypeDefinition::List(ty) => write!(f, "[{}]", ty),
            CollectionTypeDefinition::ArraySlice(ty, size) => {
                write!(f, "[{}; {}]", ty, size)
            }
            CollectionTypeDefinition::Map { key, value } => {
                write!(f, "Map<{}, {}>", key, value)
            }
        }
    }
}
