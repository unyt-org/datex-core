use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::classification::Classification,
    values::{
        value_container::ValueContainer,
    },
};
use crate::shared_values::PointerAddress;
use crate::types::entity_type::EntityType;
use crate::values::value::value_classification::ValueTag;
use crate::prelude::*;

impl Classification for ValueContainer {
    fn entity_type(&self, cache: &mut SharedReferencesCache) -> Option<EntityType> {
        match self {
            ValueContainer::Local(value) => Classification::entity_type(value, cache),
            ValueContainer::Shared(_shared) => None,
        }
    }

    fn entity_type_address(&self) -> Option<PointerAddress> {
        match self {
            ValueContainer::Local(value) => Classification::entity_type_address(value),
            ValueContainer::Shared(_shared) => None,
        }
    }

    fn tag(&self) -> Option<ValueTag> {
        match self {
            ValueContainer::Local(value) => Classification::tag(value),
            ValueContainer::Shared(_shared) => None,
        }
    }

    fn impls(&self) -> Vec<PointerAddress> {
        match self {
            ValueContainer::Local(value) => Classification::impls(value),
            ValueContainer::Shared(_shared) => Vec::new(),
        }
    }
}
