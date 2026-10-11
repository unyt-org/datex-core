use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::PointerAddress,
    types::entity_type::EntityType,
    values::value::value_classification::{
        ValueClassification, ValueTag,
        unresolved_value_classification::UnresolvedValueClassification,
    },
};

pub trait Classification {
    /// Returns the DATEX [EntityType] of the native value if it has an entity type.
    /// For structural types, this will return None.
    /// The default implementation returns None, indicating that the value does not have an entity type.
    fn entity_type(
        &self,
        _cache: &mut SharedReferencesCache,
    ) -> Option<EntityType> {
        None
    }

    /// Returns the [PointerAddress] of the entity type if it has one.
    /// The default implementation returns None, indicating that the value does not have an entity type address.
    /// It must be ensured that the entity type for the given address is already registered in the runtime.
    fn entity_type_address(&self) -> Option<PointerAddress> {
        None
    }

    /// Returns a [ValueTag] if the value has a tag.
    /// The default implementation returns None, indicating that the value does not have a tag.
    fn tag(&self) -> Option<ValueTag> {
        None
    }

    /// Returns a list of [PointerAddress]es of the impls associated with the value.
    /// The default implementation returns an empty list, indicating that the value does not have any impls.
    fn impls(&self) -> Vec<PointerAddress> {
        Vec::new()
    }

    /// Returns the DATEX [ValueClassification] of the native value.
    /// This tries to resolve the entity type and tag, assuming at most one of them is present.
    fn classification(
        &self,
        cache: &mut SharedReferencesCache,
    ) -> ValueClassification {
        let impls = self.impls();
        let entity_type = self.entity_type(cache);
        let tag = self.tag();

        ValueClassification {
            entity_type,
            impls,
            tag,
        }
    }

    /// Returns the unresolved DATEX [UnresolvedValueClassification] of the native value.
    /// This does not try to resolve the entity, but instead returns the address of the entity type if it has one.
    fn unresolved_classification(&self) -> UnresolvedValueClassification {
        let impls = self.impls();
        let entity_type_address = self.entity_type_address();
        let tag = self.tag();

        UnresolvedValueClassification {
            entity_type_address,
            impls,
            tag,
        }
    }
}
