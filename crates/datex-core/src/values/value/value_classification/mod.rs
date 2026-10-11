mod equality;
pub mod serde_dif;
pub mod unresolved_value_classification;
use crate::{
    prelude::*, shared_values::PointerAddress, types::entity_type::EntityType,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct ValueTag {
    pub tag: String,
    /// if set to true, the inner value is expected to be null but treated as non-existing (#Example instead of #Example(null))
    pub is_empty: bool,
}

impl From<String> for ValueTag {
    fn from(tag: String) -> Self {
        ValueTag {
            tag,
            is_empty: false,
        }
    }
}

impl From<&str> for ValueTag {
    fn from(tag: &str) -> Self {
        ValueTag {
            tag: tag.to_string(),
            is_empty: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct ValueClassification {
    /// a nominal (entity) type associated with the value (e.g. Example {...})
    pub entity_type: Option<EntityType>,
    /// list of impls that are associated with the value (e.g. null + $1234 + $5678)
    pub impls: Vec<PointerAddress>,
    /// a value tag, e.g. #Tagged(42)
    pub tag: Option<ValueTag>,
}

impl ValueClassification {
    /// Creates a new [ValueClassification] with no entity, no impls, and no tag.
    pub fn new_unclassified() -> Self {
        ValueClassification::default()
    }

    /// Creates a new []ValueClassification with the specified entity type.
    pub fn new_with_entity(entity: EntityType) -> Self {
        ValueClassification {
            entity_type: Some(entity),
            ..Default::default()
        }
    }

    /// Creates a new [ValueClassification] with the specified impls.
    pub fn new_with_impls(impls: Vec<PointerAddress>) -> Self {
        ValueClassification {
            impls,
            ..Default::default()
        }
    }

    /// Creates a new [ValueClassification] with the specified tag.
    pub fn new_with_tag(tag: impl Into<ValueTag>) -> Self {
        ValueClassification {
            tag: Some(tag.into()),
            ..Default::default()
        }
    }

    /// Creates a new [ValueClassification] with an optional tag.
    /// If the tag is None, the classification will be unclassified.
    pub fn new_with_maybe_tag(tag: Option<impl Into<ValueTag>>) -> Self {
        ValueClassification {
            tag: tag.map(|t| t.into()),
            ..Default::default()
        }
    }

    /// Merges the current [ValueClassification] with another one, combining their entity, impls, and tag.
    /// Prioritizes the current entity and tag if they exist; otherwise, uses the other classification's values.
    pub fn merge(&self, other: ValueClassification) -> ValueClassification {
        let mut merged_impls = self.impls.clone();
        merged_impls.extend(other.impls);

        ValueClassification {
            entity_type: self.entity_type.clone().or(other.entity_type),
            impls: merged_impls,
            tag: self.tag.clone().or(other.tag),
        }
    }

    /// Returns the tag string if it exists, otherwise returns None.
    pub fn tag_str(&self) -> Option<&str> {
        match &self.tag {
            Some(value_tag) => Some(&value_tag.tag),
            None => None,
        }
    }

    pub fn is_unclassified(&self) -> bool {
        self.entity_type.is_none()
            && self.impls.is_empty()
            && self.tag.is_none()
    }
}

impl From<EntityType> for ValueClassification {
    fn from(entity_type: EntityType) -> Self {
        ValueClassification {
            entity_type: Some(entity_type),
            ..Default::default()
        }
    }
}

impl From<Option<EntityType>> for ValueClassification {
    fn from(entity_type: Option<EntityType>) -> Self {
        ValueClassification {
            entity_type,
            ..Default::default()
        }
    }
}

impl From<Vec<PointerAddress>> for ValueClassification {
    fn from(impls: Vec<PointerAddress>) -> Self {
        ValueClassification::new_with_impls(impls)
    }
}

impl From<String> for ValueClassification {
    fn from(tag: String) -> Self {
        ValueClassification::new_with_tag(tag)
    }
}
