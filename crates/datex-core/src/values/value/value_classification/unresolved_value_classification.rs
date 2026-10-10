use super::ValueClassification;
use crate::{
    prelude::*, shared_values::PointerAddress,
    values::value::value_classification::ValueTag,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
/// Similar to [ValueClassification], but contains a [PointerAddress] for the entity type instead of [EntityType].
pub struct UnresolvedValueClassification {
    /// A pointer address for a nominal (entity) type associated with the value (e.g. Example {...})
    /// It must be guaranteed that a type with this pointer address is already registered in the runtime
    pub entity_type_address: Option<PointerAddress>,
    /// list of impls that are associated with the value (e.g. null + $1234 + $5678)
    pub impls: Vec<PointerAddress>,
    /// a value tag, e.g. #Tagged(42)
    pub tag: Option<ValueTag>,
}

impl UnresolvedValueClassification {
    pub fn new_unclassified() -> Self {
        UnresolvedValueClassification::default()
    }
    pub fn is_unclassified(&self) -> bool {
        self.entity_type_address.is_none()
            && self.impls.is_empty()
            && self.tag.is_none()
    }


    /// Returns the tag string if it exists, otherwise returns None.
    pub fn tag_str(&self) -> Option<&str> {
        match &self.tag {
            Some(value_tag) => Some(&value_tag.tag),
            None => None,
        }
    }

}

impl From<ValueClassification> for UnresolvedValueClassification {
    fn from(classification: ValueClassification) -> Self {
        let entity_type_address = classification
            .entity_type
            .map(|entity| entity.pointer_address());
        UnresolvedValueClassification {
            entity_type_address,
            impls: classification.impls,
            tag: classification.tag,
        }
    }
}
