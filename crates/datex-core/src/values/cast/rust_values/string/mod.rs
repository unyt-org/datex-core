use crate::{
    dif::deserialize_serde_context::impl_serde_with_context,
    prelude::*,
    traits::{datex_hash::impl_datex_hash, value_access::ValueAccess},
};

mod convert_value;
mod to_instructions;
pub mod try_clone;

#[cfg(feature = "ast")]
mod to_datex_expression_data;
pub mod serde_dif;

impl ValueAccess for String {}

impl_datex_hash!(String);