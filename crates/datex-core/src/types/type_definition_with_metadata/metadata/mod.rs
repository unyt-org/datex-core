use crate::shared_values::ReferenceMutability;
use binrw::{BinRead, BinWrite};
use core::fmt::Display;
use serde_repr::*;

use crate::shared_values::{
    SharedContainerMutability, SharedContainerOwnership,
};
use serde::{Deserialize, Serialize};
pub mod type_match;
pub mod to_instructions;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize_repr,
    Deserialize_repr,
    Copy,
    BinRead,
    BinWrite,
)]
#[brw(little, repr = u8)]
#[repr(u8)]
pub enum LocalReferenceMutability {
    Immutable = 0,
    Mutable = 1,
}
impl Display for LocalReferenceMutability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LocalReferenceMutability::Immutable => write!(f, "&"),
            LocalReferenceMutability::Mutable => write!(f, "&mut"),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize_repr,
    Deserialize_repr,
    Copy,
    BinRead,
    BinWrite,
)]
#[brw(little, repr = u8)]
#[repr(u8)]
pub enum LocalMutability {
    Immutable = 0,
    Mutable = 1,
}

impl Display for LocalMutability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LocalMutability::Immutable => write!(f, ""),
            LocalMutability::Mutable => write!(f, "mut"),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Copy,
    BinRead,
    BinWrite,
)]
#[serde(untagged)]
#[brw(little)]
pub enum LocalOwnership {
    #[brw(magic = 0x0u8)]
    Owned,
    #[brw(magic = 0x1u8)]
    Referenced(LocalReferenceMutability),
}
impl Display for LocalOwnership {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LocalOwnership::Owned => write!(f, ""),
            LocalOwnership::Referenced(reference_mutability) => {
                write!(f, "{}", reference_mutability)
            }
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Copy,
    BinRead,
    BinWrite,
)]
/// Combination of &/&mut, '/'mut shared and mut prefixes
#[serde(tag = "kind")]
#[brw(little)]
pub enum TypeMetadata {
    /// Local types can be mut or not, and can optionally be a reference type with an additional reference mutability (e.g. &mut User)
    #[brw(magic = 0x0u8)]
    Local {
        mutability: LocalMutability,
        ownership: LocalOwnership,
    },
    /// Shared types are always (shared or shared mut) and can optionally be a non-owned, reference type
    /// with an additional reference mutability (e.g. 'mut shared mut User)
    #[brw(magic = 0x1u8)]
    Shared {
        mutability: SharedContainerMutability,
        ownership: SharedContainerOwnership,
    },
}

impl Display for TypeMetadata {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TypeMetadata::Local {
                mutability,
                ownership,
            } => {
                write!(f, "{}", ownership)?;
                if let LocalOwnership::Referenced(
                    LocalReferenceMutability::Mutable,
                ) = ownership
                {
                    write!(f, " ")?
                };
                write!(f, "{}", mutability)
            }
            TypeMetadata::Shared {
                mutability,
                ownership,
            } => {
                write!(f, "{}", ownership)?;
                if let SharedContainerOwnership::Referenced(
                    ReferenceMutability::Mutable,
                ) = ownership
                {
                    write!(f, " ")?
                };
                write!(f, "{}", mutability)
            }
        }
    }
}

impl TypeMetadata {
    /// Ownership type for a shared container
    pub fn shared_container_ownership(
        &self,
    ) -> Option<&SharedContainerOwnership> {
        match self {
            TypeMetadata::Local { .. } => None,
            TypeMetadata::Shared { ownership, .. } => Some(ownership),
        }
    }

    /// Mutability for a shared type (e.g. shared mut X / shared X), if applicable
    pub fn shared_mutability(&self) -> Option<SharedContainerMutability> {
        match self {
            TypeMetadata::Local { .. } => None,
            TypeMetadata::Shared { mutability, .. } => Some(*mutability),
        }
    }

    /// Whether this type is a shared type (e.g. shared X, shared mut X, &shared X, &mut shared X)
    pub fn is_shared_type(&self) -> bool {
        match self {
            TypeMetadata::Shared { .. } => true,
            TypeMetadata::Local { .. } => false,
        }
    }

    /// Whether this is the default metadata without any prefix
    pub fn is_default_metadata(&self) -> bool {
        *self == TypeMetadata::default()
    }
}

impl TypeMetadata {
    // FIXME: Move to const default once supported is stable
    pub const fn default() -> Self {
        TypeMetadata::Local {
            mutability: LocalMutability::Immutable,
            ownership: LocalOwnership::Owned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;
    #[test_case(LocalMutability::Mutable; "mutable")]
    #[test_case(LocalMutability::Immutable; "immutable")]
    fn mutability(muty: LocalMutability) {
        let serialized = serde_json::to_value(&muty).unwrap();
        let deserialized: LocalMutability =
            serde_json::from_value(serialized).unwrap();
        assert_eq!(muty, deserialized);
    }
}
