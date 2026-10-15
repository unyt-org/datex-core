use crate::{
    random::RandomState,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::errors::{AccessError, KeyNotFoundError},
    traits::{
        convert_value::ConvertValue,
        convert_value_container::ConvertValueContainer,
    },
    values::{
        core_values::map::MapAccessError, value_container::ValueContainer,
    },
};
use core::{cell::RefCell, hash::Hash};
use indexmap::IndexMap;

use crate::{
    types::error::TypeError,
    value_updates::{
        errors::UpdateError,
        update_data::{DeleteEntryUpdateData, SetEntryUpdateData},
        update_handler::{UpdateCallbackDataAccess, UpdateHandlerImpl},
    },
    values::core_values::native::DatexNativeBase,
};
impl<K, V> UpdateCallbackDataAccess for IndexMap<K, V, RandomState>
where
    K: ConvertValueContainer + Eq + Hash,
    V: ConvertValueContainer,
{
}

impl<K, V> UpdateHandlerImpl for IndexMap<K, V, RandomState>
where
    K: Eq + Hash + DatexNativeBase + 'static,
    V: DatexNativeBase + 'static,
{
    fn try_set_entry(
        &mut self,
        data: SetEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Option<ValueContainer>, UpdateError> {
        let key = data
            .key
            .try_into_value()
            .map_err(|_| UpdateError::type_error(TypeError::Invalid))?;
        let value = data
            .value
            .try_into_value()
            .map_err(|_| UpdateError::type_error(TypeError::Invalid))?;
        Ok(self
            .insert(key, value)
            .map(|previous| previous.to_value_container()))
    }

    fn try_delete_entry(
        &mut self,
        data: DeleteEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        let key = data
            .key
            .try_into_value::<K>()
            .map_err(|_| UpdateError::type_error(TypeError::Invalid))?;

        self.shift_remove(&key)
            .map(|previous| previous.to_value_container())
            .ok_or_else(|| {
                AccessError::MapAccessError(MapAccessError::KeyNotFound(
                    KeyNotFoundError::new(key.to_value_container()),
                ))
                .into()
            })
    }

    fn try_clear(
        &mut self,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        let previous = core::mem::take(self);
        Ok(ValueContainer::Local(previous.to_value()))
    }
}
