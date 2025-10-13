use std::fmt::Display;

use crate::types::type_container::TypeContainer;

// TODO #377: Rename to Generic type definition?
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum CollectionTypeDefinition {
    // e.g. [integer]
    List(Box<TypeContainer>),

    // e.g. [integer; 5]
    ListSlice(Box<TypeContainer>, usize),

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
            CollectionTypeDefinition::ListSlice(ty, size) => {
                write!(f, "[{}; {}]", ty, size)
            }
            CollectionTypeDefinition::Map { key, value } => {
                write!(f, "Map<{}, {}>", key, value)
            }
        }
    }
}
