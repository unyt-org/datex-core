use crate::{
    prelude::*,
    shared_values::errors::{AccessError, KeyNotFoundError},
    traits::value_access::ValueAccess,
    types::r#type::Type,
    utils::goat::Goat,
    values::{
        core_value::{CoreValue, borrowed_core_value::BorrowedCoreValue},
        core_value_with_classification::{
            CoreValueWithClassification,
            borrowed_core_value_with_classification::BorrowedCoreValueWithClassification,
        },
        value::{
            borrowed_value::BorrowedValue,
            value_classification::ValueClassification,
        },
        value_container::{
            borrowed_value_container::{
                BorrowedValueContainer, BorrowedValueContainerMut,
            },
            value_key::BorrowedValueKey,
        },
    },
};
use core::cell::Ref;

impl ValueAccess for CoreValueWithClassification {
    fn try_get_property(
        &self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        match &self.inner {
            CoreValue::Map(map) => map.try_get_property(key),
            CoreValue::List(list) => list.try_get_property(key),
            CoreValue::Type(Type::Entity(container)) => {
                if let Some(key) = key.try_as_text() {
                    let reference = Ref::filter_map(
                        container.entity_definition(),
                        |entity_definition| {
                            entity_definition.try_get_property(key)
                        },
                    )
                    .map_err(|_| {
                        AccessError::KeyNotFound(KeyNotFoundError::new(
                            key.to_string().into(),
                        ))
                    })?;
                    Ok(BorrowedValueContainer::Local(BorrowedValue::Core(
                        BorrowedCoreValueWithClassification {
                            inner: BorrowedCoreValue::Callable(Goat::Ref(
                                reference,
                            )),
                            classification:
                                ValueClassification::new_unclassified(),
                        },
                    )))
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            _ => {
                // If the value is not a map, we cannot get a property
                Err(AccessError::InvalidOperation(
                    "Cannot get property".to_string(),
                ))
            }
        }
    }

    fn try_get_property_mut(
        &mut self,
        key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainerMut<'_>, AccessError> {
        match &mut self.inner {
            CoreValue::Map(map) => map.try_get_property_mut(key),
            CoreValue::List(list) => list.try_get_property_mut(key),
            _ => {
                // If the value is not a map, we cannot get a property
                Err(AccessError::InvalidOperation(
                    "Cannot get property".to_string(),
                ))
            }
        }
    }
}
