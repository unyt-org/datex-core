use crate::{
    shared_values::{
        base_shared_value_container::observers::TransceiverId,
        errors::KeyNotFoundError,
    },
    value_updates::{
        update_data::UpdateOperation,
        update_handler::InternalMutabilityUpdateHandler,
    },
    values::{
        core_values::map::{Map, MapAccessError, MapEntries},
        value::Value,
        value_container::{ValueContainer},
    },
    prelude::*,
};
use core::mem;

impl Map {
    /// Removes a key from the map, returning the value if it existed.
    pub fn try_delete_with_source<'a>(
        &mut self,
        key: &ValueContainer,
        source: Option<TransceiverId>,
    ) -> Result<ValueContainer, MapAccessError> {
        let res = match &mut self.entries {
            MapEntries::Dynamic(map) =>
                map.shift_remove(key).ok_or_else(|| {
                    MapAccessError::KeyNotFound(KeyNotFoundError::new(
                        key.clone(),
                    ))
                }),
            MapEntries::Structural(_)
            | MapEntries::StructuralWithStringKeys(_) => {
                Err(MapAccessError::Immutable)
            }
        }?
        .without_local_observers();

        self.maybe_trigger_update_callback(source, || {
            UpdateOperation::delete_entry(key.clone())
        });

        Ok(res)
    }

    /// Removes a key from the map, returning the value if it existed.
    /// Also works for structural maps, but creates a map that no longer matches the assumed type.
    /// # Safety
    /// The map should no longer be used after this operation.
    pub unsafe fn try_delete_unchecked_with_source<'a>(
        &mut self,
        key: &ValueContainer,
        source: Option<TransceiverId>,
    ) -> Result<ValueContainer, KeyNotFoundError> {
        let res = match &mut self.entries {
            MapEntries::Dynamic(map) =>
                map.shift_remove(key)
                    .ok_or_else(|| KeyNotFoundError::new(key.clone())),
            MapEntries::Structural(vec) => {
                for (k, v) in vec.iter_mut() {
                    if k == key {
                        return Ok(core::mem::replace(
                            v,
                            ValueContainer::from(Value::null()),
                        ));
                    }
                }
                Err(KeyNotFoundError::new(key.clone()))
            },
            MapEntries::StructuralWithStringKeys(vec) => {
                if let Some(string) = key.try_as::<String>() {
                    for (k, v) in vec.iter_mut() {
                        if k == string {
                            return Ok(core::mem::replace(
                                v,
                                ValueContainer::from(Value::null()),
                            ));
                        }
                    }
                    Err(KeyNotFoundError::new(key.clone().into()))
                } else {
                    Err(KeyNotFoundError::new(key.clone().into()))
                }
            }
        }?;

        self.maybe_trigger_update_callback(source, || {
            UpdateOperation::delete_entry(key.clone())
        });

        Ok(res)
    }

    /// Clears all entries in the map, returning an error if the map is not dynamic.
    pub fn try_clear_with_source(
        &mut self,
        source: Option<TransceiverId>,
    ) -> Result<ValueContainer, MapAccessError> {
        let res = match &mut self.entries {
            MapEntries::Dynamic(map) => {
                let previous = mem::take(map);
                Ok(ValueContainer::from(Map::from(MapEntries::Dynamic(
                    previous,
                ))))
            }
            MapEntries::Structural(_)
            | MapEntries::StructuralWithStringKeys(_) => {
                Err(MapAccessError::Immutable)
            }
        }?
        .without_local_observers();

        self.maybe_trigger_update_callback(source, UpdateOperation::clear);

        Ok(res)
    }

    /// Sets a value in the map, returning an error if it fails.
    /// This is the preferred way to set values in the map.
    pub(crate) fn try_set_with_source<'a>(
        &mut self,
        key: impl Into<ValueContainer>,
        value: impl Into<ValueContainer>,
        source: Option<TransceiverId>,
    ) -> Result<Option<ValueContainer>, KeyNotFoundError> {
        let key = key.into();
        let mut value = value.into();

        if self.update_callback_data.is_some() {
            self.set_child_update_callback_data_if_local(&key, &mut value);
        }

        // TODO: no clone here
        let value_clone = value.clone();

        let res = match &mut self.entries {
            MapEntries::Dynamic(map) => {
                Ok(map.insert(key.clone(), value))
            }
            MapEntries::Structural(vec) =>
                if let Some((_, v)) = vec.iter_mut().find(|(k, _)| k == &key) {
                    Ok(Some(core::mem::replace(v, value)))
                } else {
                    Err(KeyNotFoundError::new(key.clone()))
                },
            MapEntries::StructuralWithStringKeys(vec) => {
                if let Some(string) = key.try_as::<String>() {
                    if let Some((_, v)) =
                        vec.iter_mut().find(|(k, _)| k == string)
                    {
                        Ok(Some(core::mem::replace(v, value)))
                    } else {
                        Err(KeyNotFoundError::new(key.clone().into()))
                    }
                } else {
                    Err(KeyNotFoundError::new(key.clone().into()))
                }
            }
        }?
        .map(|v| v.without_local_observers());

        self.maybe_trigger_update_callback(source, || {
            UpdateOperation::set_entry(key.into(), value_clone)
        });

        Ok(res)
    }
}
