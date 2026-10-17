use crate::{
    prelude::*,
    shared_values::errors::{AccessError, KeyNotFoundError},
    traits::value_access::ValueAccess,
    types::entity_type::EntityType,
    values::{
        value::{
            borrowed_value::{
                BorrowedValue,
            },
            value_classification::ValueClassification,
        },
    },
};
use core::cell::Ref;
use crate::values::core_value::borrowed_core_value::BorrowedCoreValue;
use crate::values::core_value_with_classification::borrowed_core_value_with_classification::BorrowedCoreValueWithClassification;
use crate::values::value_container::borrowed_value_container::BorrowedValueContainer;
use crate::values::value_container::ValueContainer;

impl ValueAccess for EntityType {
    fn try_get_property(
        &self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        if let Some(key) = key.try_as_str() {
            let callable_ref = Ref::filter_map(
                self.entity_definition(),
                |entity_definition| entity_definition.try_get_property(key),
            )
            .map_err(|_| {
                AccessError::KeyNotFound(KeyNotFoundError::new(
                    key.to_string().into(),
                ))
            })?;
            Ok(BorrowedValueContainer::Local(BorrowedValue::Core(
                BorrowedCoreValueWithClassification {
                    inner: BorrowedCoreValue::Callable(callable_ref.into()),
                    classification: ValueClassification::new_unclassified(),
                },
            )))
        } else {
            Err(AccessError::InvalidIndexKey)
        }
    }
}
