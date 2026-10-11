use crate::{
    traits::get_core_lib_type_id::GetCoreLibTypeId,
    values::value_container::borrowed_value_container::BorrowedValueContainer,
};

impl<'a> GetCoreLibTypeId for BorrowedValueContainer<'a> {}
