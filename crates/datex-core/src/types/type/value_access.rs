use crate::{traits::value_access::ValueAccess, types::r#type::Type};
use crate::shared_values::errors::AccessError;
use crate::values::value_container::borrowed_value_container::BorrowedValueContainer;
use crate::prelude::*;
use crate::values::value_container::ValueContainer;

impl ValueAccess for Type {
    fn try_get_property(
        &self,
        key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        match self {
            Type::Entity(entity) => entity.try_get_property(key),
            _ => Err(AccessError::InvalidOperation(
                "Cannot get property".to_string(),
            ))
        }
    }
}
