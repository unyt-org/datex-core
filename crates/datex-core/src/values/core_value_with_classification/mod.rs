use crate::{
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::errors::AccessError,
    types::{
        r#type::Type,
        type_definition::{
            TypeDefinition, impl_type::ImplMarkers,
            intersection::IntersectionTypeDefinition,
            tagged_type::TaggedTypeDefinition,
        },
    },
    values::{
        core_value::CoreValue,
        value::Value,
        value_container::{ValueContainer},
    },
};

mod iter_parts;
use core::{cell::RefCell, fmt::Display};
mod apply;
mod classification;
mod convert_parts;
mod equality;
pub mod serde_dif;
mod to_instructions;
mod update_handler;
mod value_access;
use crate::{
    prelude::*,
    values::value::value_classification::{ValueClassification, ValueTag},
};
use crate::values::core_values::text::SetCharAtError;
use crate::values::value_container::borrowed_value_container::BorrowedValueContainer;

pub mod borrowed_core_value_with_classification;
mod hash;
mod ops;
mod try_clone;

#[derive(Debug, Clone)]
pub struct CoreValueWithClassification {
    /// The inner representation of the value, which is a [CoreValue].
    pub inner: CoreValue,
    /// additional extensions for the value, including its entity type, impls, and tag
    pub classification: ValueClassification,
}

impl From<CoreValue> for CoreValueWithClassification {
    fn from(value: CoreValue) -> Self {
        CoreValueWithClassification::new(value)
    }
}

impl Display for CoreValueWithClassification {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if !self.classification.is_unclassified() {
            write!(f, "{:?}", self.classification)?; // TODO: better display for classification?
        }
        write!(f, "{}", self.inner)
    }
}

impl CoreValueWithClassification {
    pub fn new(inner: CoreValue) -> Self {
        CoreValueWithClassification {
            inner,
            classification: ValueClassification::default(),
        }
    }
    pub fn new_with_classification(
        inner: CoreValue,
        classification: ValueClassification,
    ) -> Self {
        CoreValueWithClassification {
            inner,
            classification,
        }
    }

    pub fn is_uninitialized(&self) -> bool {
        matches!(&self.inner, CoreValue::Uninitialized)
    }

    pub fn is_null(&self) -> bool {
        matches!(self.inner, CoreValue::Null)
    }

    /// Returns the actual current [TypeDefinition] of the value
    pub fn actual_type(&self) -> TypeDefinition {
        let mut types = Vec::<Type>::new();

        if let Some(entity_type) = &self.classification.entity_type {
            types.push(Type::Entity(entity_type.clone()));
        }

        if let Some(ValueTag { tag, is_empty }) = &self.classification.tag {
            types.push(
                TypeDefinition::TaggedType(TaggedTypeDefinition {
                    tag: tag.clone(),
                    ty: if *is_empty {
                        None
                    } else {
                        Some(Box::new(Type::core(
                            self.inner.core_lib_type_id(),
                        )))
                    },
                })
                .into(),
            );
        }

        if !self.classification.impls.is_empty() {
            types.push(
                TypeDefinition::ImplMarkers(ImplMarkers::new(
                    self.classification.impls.clone(),
                ))
                .into(),
            );
        }

        if types.is_empty() {
            TypeDefinition::CoreType(self.inner.core_lib_type_id())
        } else if types.len() == 1 {
            types.into_iter().next().unwrap().convert_to_definition()
        } else {
            TypeDefinition::Intersection(IntersectionTypeDefinition::new(types))
        }
    }

    /// Takes (removes) a property from the value if applicable (e.g. for map and structs)
    pub fn try_take_property<'a>(
        &mut self,
        key: &ValueContainer,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, AccessError> {
        // TODO
        match self.inner {
            CoreValue::Map(ref mut map) => {
                // If the value is a map, get the property
                Ok(map.try_delete(key)?)
            }
            CoreValue::List(ref mut list) => {
                if let Some(index) = key.try_as::<u32>() {
                    Ok(list.try_delete(*index)?)
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            CoreValue::Text(ref text) => {
                if let Some(index) = key.try_as::<u32>() {
                    let char = text.char_at(*index)?;                
                    Ok(ValueContainer::from(char.to_string()))
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            _ => {
                // If the value is not an map, we cannot get a property
                Err(AccessError::InvalidOperation(
                    "Cannot get property".to_string(),
                ))
            }
        }
    }

    pub fn try_delete_property<'a>(
        &mut self,
        key: &ValueContainer,
    ) -> Result<(), AccessError> {
        match self.inner {
            CoreValue::Map(ref mut map) => {
                // If the value is a map, delete the property
                map.try_delete(key)?;
                Ok(())
            }
            CoreValue::List(ref mut list) => {
                if let Some(index) = key.try_as::<u32>() {
                    list.try_delete(*index)?;
                    Ok(())
                } else {
                    Err(AccessError::InvalidIndexKey)
                }
            }
            CoreValue::Text(_) => Err(AccessError::InvalidOperation(
                "Cannot delete property on text".to_string(),
            )),
            _ => {
                // If the value is not a map, we cannot delete a property
                Err(AccessError::InvalidOperation(
                    "Cannot delete property".to_string(),
                ))
            }
        }
    }

    /// Sets a property on the value if applicable (e.g. for maps)
    pub fn try_set_property<'a>(
        &mut self,
        key: impl Into<ValueContainer>,
        val: ValueContainer,
    ) -> Result<(), AccessError> {
        let key = key.into();
        match self.inner {
            CoreValue::Map(ref mut map) => {
                // If the value is an map, set the property
                map.try_set(key, val)?;
            }
            CoreValue::List(ref mut list) => {
                if let Some(index) = key.try_as::<u32>() {
                    list.try_set(*index, val)
                        .map_err(AccessError::IndexOutOfBounds)?;
                } else {
                    return Err(AccessError::InvalidIndexKey);
                }
            }
            CoreValue::Text(ref mut text) => {
                if let Some(index) = key.try_as::<u32>() {
                    if let ValueContainer::Local(Value::Core(v)) = &val
                        && let CoreValue::Text(new_char) = &v.inner
                        && new_char.len() == 1
                    {
                        let char = new_char.as_str().chars().next().unwrap_or('\0');
                        text.set_char_at(*index, char).map_err(|err| {
                            match err {
                                SetCharAtError::IndexOutOfBoundsError(err) => AccessError::IndexOutOfBounds(err),
                                SetCharAtError::NotMutableError => AccessError::InvalidOperation(
                                    "Cannot set char in non-mutable text".to_string(),
                                ),
                            }
                        })?;
                    } else {
                        return Err(AccessError::InvalidOperation(
                            "Can only set char character in text".to_string(),
                        ));
                    }
                } else {
                    return Err(AccessError::InvalidIndexKey);
                }
            }
            _ => {
                // If the value is not a map, we cannot set a property
                return Err(AccessError::InvalidOperation(format!(
                    "Cannot set property '{}' on non-map value: {:?}",
                    key, self
                )));
            }
        }

        Ok(())
    }
}
