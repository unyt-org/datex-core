use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    traits::classification::Classification,
    values::value::{
        Value,
    },
};
use crate::shared_values::PointerAddress;
use crate::types::entity_type::EntityType;
use crate::values::value::value_classification::ValueTag;
use crate::prelude::*;

impl Classification for Value {
    fn entity_type(&self, cache: &mut SharedReferencesCache) -> Option<EntityType> {
        match self {
            Value::Core(core_value) => core_value.entity_type(cache),
            Value::Native(custom_value) => custom_value.entity_type(cache),
        }
    }

    fn entity_type_address(&self) -> Option<PointerAddress> {
        match self {
            Value::Core(core_value) => core_value.entity_type_address(),
            Value::Native(custom_value) => custom_value.entity_type_address(),
        }
    }

    fn tag(&self) -> Option<ValueTag> {
        match self {
            Value::Core(core_value) => core_value.tag(),
            Value::Native(custom_value) => custom_value.tag(),
        }
    }

    fn impls(&self) -> Vec<PointerAddress> {
        match self {
            Value::Core(core_value) => core_value.impls(),
            Value::Native(custom_value) => custom_value.impls(),
        }
    }
}
