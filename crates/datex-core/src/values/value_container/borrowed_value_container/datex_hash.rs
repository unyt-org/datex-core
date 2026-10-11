use crate::{
    traits::datex_hash::impl_datex_hash,
    values::value_container::borrowed_value_container::BorrowedValueContainer,
};

impl_datex_hash!(BorrowedValueContainer<'_>);
