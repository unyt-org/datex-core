use core::{cell::RefCell, mem, ops::DerefMut};

use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::errors::{AccessError, IndexOutOfBoundsError},
    types::error::TypeError,
    value_updates::{
        errors::UpdateError,
        update_data::{
            AppendEntryUpdateData, DeleteEntryUpdateData, ListSpliceUpdateData,
            SetEntryUpdateData,
        },
        update_handler::{UpdateCallbackDataAccess, UpdateHandlerImpl},
    },
    values::{
        core_values::native::DatexNativeBase,
        value_container::{ValueContainer},
    },
};
impl<T: DatexNativeBase + 'static> UpdateCallbackDataAccess for Vec<T> {}
impl<T: DatexNativeBase + 'static> UpdateHandlerImpl for Vec<T> {
    fn try_set_entry(
        &mut self,
        data: SetEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Option<ValueContainer>, UpdateError> {
        let key = *data.key.try_as::<u32>().ok_or_else(
            || UpdateError::access_error(AccessError::InvalidIndexKey),
        )? as usize;
        self.get_mut(key)
            .map(|previous| {
                let new = data.value.try_into_value::<T>().map_err(|_| ())?;
                let previous = mem::replace(previous, new);
                Ok(previous.to_value_container())
            })
            .transpose()
            .map_err(|_: ()| UpdateError::type_error(TypeError::Invalid))?
            .ok_or_else(|| {
                UpdateError::access_error(AccessError::IndexOutOfBounds(
                    IndexOutOfBoundsError { index: key as u32 },
                ))
            })
            .map(Some)
    }

    fn try_delete_entry(
        &mut self,
        data: DeleteEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        let key = *data.key.try_as::<u32>().ok_or_else(
            || UpdateError::access_error(AccessError::InvalidIndexKey),
        )? as usize;
        let removed = self.try_remove(key).ok_or_else(|| {
            UpdateError::access_error(AccessError::IndexOutOfBounds(
                IndexOutOfBoundsError { index: key as u32 },
            ))
        })?;
        Ok(removed.to_value_container())
    }

    fn try_append_entry(
        &mut self,
        data: AppendEntryUpdateData,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<(), UpdateError> {
        self.push(
            data.value
                .try_into_value::<T>()
                .map_err(|_| UpdateError::type_error(TypeError::Invalid))?,
        );
        Ok(())
    }

    fn try_clear(
        &mut self,
        _cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        let previous = core::mem::take(self);
        Ok(ValueContainer::Local(previous.into()))
    }

    fn try_list_splice(
        &mut self,
        data: ListSpliceUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Vec<ValueContainer>, UpdateError> {
        let range = (data.start as usize)
            ..(data.start as usize + data.delete_count as usize);
        let res = self
            .splice(
                range,
                data.items
                    .into_iter()
                    .map(|item| {
                        item.try_into_value::<T>().map_err(|_| {
                            UpdateError::type_error(TypeError::Invalid)
                        })
                    })
                    .collect::<Result<Vec<_>, UpdateError>>()?,
            )
            .collect::<Vec<_>>();
        let mut cache = cache.borrow_mut();
        let _cache = cache.deref_mut();
        Ok(res
            .into_iter()
            .map(|item| item.to_value_container())
            .collect())
    }
}
