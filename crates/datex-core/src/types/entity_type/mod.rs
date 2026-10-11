pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_type_expression_data;
mod value_access;

use crate::{
    prelude::*,
    runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
    shared_values::{
        PointerAddress, SharedContainer, SharedContainerMutability,
        traits::{_ExposeRcInternal, SharedContainerCommon},
    },
    types::{
        entities::entity_type_definition::EntityTypeDefinition,
        shared_container_containing_type::SharedContainerContainingType,
        traits::type_match::{TypeSatisfiesValueContainer, TypeSuperset},
    },
    values::{
        core_value::CoreValue,
        core_value_with_classification::CoreValueWithClassification,
        value::{
            Value,
            value_classification::unresolved_value_classification::UnresolvedValueClassification,
        },
        value_container::ValueContainer,
    },
};
use core::{cell::Ref, ops::Deref};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct EntityType(SharedContainer);

impl Deref for EntityType {
    type Target = SharedContainer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<EntityType> for SharedContainer {
    fn from(value: EntityType) -> Self {
        value.0
    }
}

impl From<EntityType> for SharedContainerContainingType {
    fn from(value: EntityType) -> Self {
        unsafe { SharedContainerContainingType::new_unchecked(value.0) }
    }
}

impl EntityType {
    pub fn new_from_definition(
        definition: EntityTypeDefinition,
        address_provider: &mut SelfOwnedPointerAddressProvider,
    ) -> EntityType {
        EntityType(SharedContainer::new_owned_with_inferred_allowed_type(
            CoreValue::EntityTypeDefinition(definition),
            SharedContainerMutability::Immutable,
            address_provider,
        ))
    }

    /// Converts the [EntityType] into a [SharedContainer], consuming the wrapper.
    pub fn to_shared_container(self) -> SharedContainer {
        self.0
    }

    /// Returns a reference to the inner [SharedContainer] of the [EntityType].
    pub fn shared_container(&self) -> &SharedContainer {
        &self.0
    }

    /// Creates a new [EntityType] from a [SharedContainer] without checking the constraint.
    /// # Safety
    /// The caller must ensure that the constraint for [EntityType] is satisfied
    /// (i.e. the allowed type of the container is a [Type::Nominal])
    pub unsafe fn new_unchecked(container: SharedContainer) -> Self {
        EntityType(container)
    }

    /// Returns a reference to the inner [EntityTypeDefinition] contained in the [SharedContainer].
    /// The [EntityType] guarantees that the inner value is always a [CoreValue::EntityTypeDefinition], so this method can never panic.
    pub fn entity_definition(&self) -> Ref<'_, EntityTypeDefinition> {
        let val = self.0.value_container();
        Ref::map(val, |v| match v {
            ValueContainer::Local(Value::Core(
                CoreValueWithClassification {
                    inner: CoreValue::EntityTypeDefinition(ty),
                    ..
                },
            )) => ty,
            _ => unreachable!(
                "The constraint for SharedContainerContainingEntityType guarantees that the inner value is always a CoreValue::EntityTypeDefinition"
            ),
        })
    }

    /// Replaces the inner [EntityTypeDefinition] contained in the [SharedContainer].
    pub(crate) fn replace_definition(&self, definition: EntityTypeDefinition) {
        let reference = self.0.clone().derive_reference_with_max_mutability();
        let mut inner = reference.get_rc_internal().borrow_mut();
        *inner.base_shared_container_mut().value_container_mut() =
            ValueContainer::Local(Value::from(
                CoreValue::EntityTypeDefinition(definition),
            ));
    }
}

impl TryFrom<SharedContainer> for EntityType {
    type Error = ();
    fn try_from(value: SharedContainer) -> Result<Self, Self::Error> {
        // container must be immutable and contain nominal type
        if value.container_mutability() == SharedContainerMutability::Immutable
        {
            let is_nominal = {
                let val = value.collapsed_value();
                let val_sheep = val.borrow();
                matches!(
                    *val_sheep,
                    Value::Core(CoreValueWithClassification {
                        inner: CoreValue::EntityTypeDefinition(_),
                        ..
                    })
                )
            };

            if is_nominal {
                Ok(EntityType(value))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}

impl TypeSuperset<EntityType> for EntityType {
    fn is_superset_of(&self, other: &EntityType) -> bool {
        // if it is directly the same nominal type definition
        self.pointer_address() == other.pointer_address()
    }
}

impl TypeSuperset<PointerAddress> for EntityType {
    fn is_superset_of(&self, other: &PointerAddress) -> bool {
        &self.pointer_address() == other
    }
}

impl TypeSatisfiesValueContainer for EntityType {
    fn satisfies_value_container(&self, value: &ValueContainer) -> bool {
        match &value.collapsed_value().borrow().unresolved_classification() {
            UnresolvedValueClassification {
                entity_type_address: Some(entity_type_address),
                ..
            } => self.is_superset_of(entity_type_address),
            _ => false,
        }
    }
}
