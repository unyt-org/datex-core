use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::errors::AccessError,
    traits::{
        convert_value::ConvertValue,
        local_child_path_resolver::LocalChildPathResolver,
    },
    value_updates::{
        errors::UpdateError::{self},
        update_data::*,
        update_handler::{UpdateCallbackDataAccess, UpdateHandlerImpl},
    },
    values::{
        core_values::native::{DatexNative, DatexNativeOps},
        value_container::ValueContainer,
    },
};
use core::{any::Any, cell::RefCell};
use crate::traits::get_datex_type::GetDatexType;

impl<T: DatexNative + GetDatexType + ConvertValue + 'static> DatexNative for Option<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl<T: DatexNative + ConvertValue + 'static> DatexNativeOps for Option<T> {}
impl<T: DatexNative + ConvertValue + 'static> LocalChildPathResolver
    for Option<T>
{
}

impl<T: DatexNative + ConvertValue + 'static> UpdateCallbackDataAccess
    for Option<T>
{
}
impl<T: DatexNative + ConvertValue + 'static> UpdateHandlerImpl for Option<T> {
    fn try_set_entry(
        &mut self,
        data: SetEntryUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Option<ValueContainer>, UpdateError> {
        if let Some(inner) = self {
            inner.try_set_entry(data, cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot set entry on null value".to_string(),
            )))
        }
    }
    fn try_delete_entry(
        &mut self,
        data: DeleteEntryUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        if let Some(inner) = self {
            inner.try_delete_entry(data, cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot delete entry from null value".to_string(),
            )))
        }
    }
    fn try_append_entry(
        &mut self,
        data: AppendEntryUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<(), UpdateError> {
        if let Some(inner) = self {
            inner.try_append_entry(data, cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot append entry to null value".to_string(),
            )))
        }
    }
    fn try_clear(
        &mut self,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<ValueContainer, UpdateError> {
        if let Some(inner) = self {
            inner.try_clear(cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot clear null value".to_string(),
            )))
        }
    }
    fn try_list_splice(
        &mut self,
        data: ListSpliceUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<Vec<ValueContainer>, UpdateError> {
        if let Some(inner) = self {
            inner.try_list_splice(data, cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot list splice null value".to_string(),
            )))
        }
    }
    fn try_increment(
        &mut self,
        data: IncrementUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<(), UpdateError> {
        if let Some(inner) = self {
            inner.try_increment(data, cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot increment null value".to_string(),
            )))
        }
    }
    fn try_decrement(
        &mut self,
        data: DecrementUpdateData,
        cache: &RefCell<SharedReferencesCache>,
    ) -> Result<(), UpdateError> {
        if let Some(inner) = self {
            inner.try_decrement(data, cache)
        } else {
            Err(UpdateError::access_error(AccessError::InvalidOperation(
                "Cannot decrement null value".to_string(),
            )))
        }
    }
}
