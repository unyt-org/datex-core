use crate::runtime::cache::shared_references_cache::SharedReferencesCache;
use crate::shared_values::PointerAddress;
use crate::traits::classification::Classification;
use crate::types::entity_type::EntityType;
use crate::values::core_values::native::NativeCoreValue;
use crate::values::value::value_classification::{ValueTag};
use crate::prelude::*;

impl Classification for NativeCoreValue {
    fn entity_type(&self, cache: &mut SharedReferencesCache) -> Option<EntityType> {
        self.value.entity_type(cache)
    }

    fn entity_type_address(&self) -> Option<PointerAddress> {
        self.value.entity_type_address()
    }

    fn tag(&self) -> Option<ValueTag> {
        self.value.tag()
    }

    fn impls(&self) -> Vec<PointerAddress> {
        self.value.impls()
    }
}