use crate::{
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    types::entities::entity_type_definition::EntityTypeDefinition,
    values::value_container::{
        borrowed_value_container::BorrowedValueContainer,
    },
};
use crate::values::value_container::ValueContainer;

impl ValueAccess for EntityTypeDefinition {
    fn try_get_property(
        &self,
        _key: &ValueContainer,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        todo!()
    }
}
