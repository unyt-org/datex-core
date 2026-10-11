use crate::{
    shared_values::errors::AccessError,
    traits::value_access::ValueAccess,
    types::entities::entity_type_definition::EntityTypeDefinition,
    values::value_container::{
        borrowed_value_container::BorrowedValueContainer,
        value_key::BorrowedValueKey,
    },
};

impl ValueAccess for EntityTypeDefinition {
    fn try_get_property(
        &self,
        _key: BorrowedValueKey,
    ) -> Result<BorrowedValueContainer<'_>, AccessError> {
        todo!()
    }
}
