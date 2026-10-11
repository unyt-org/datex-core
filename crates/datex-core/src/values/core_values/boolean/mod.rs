use crate::{
    prelude::*,
    values::value_container::{ValueContainer, error::ValueError},
};
mod to_instructions;

use binrw::{BinRead, BinWrite};
use core::{fmt::Display, result::Result};
mod classification;
pub mod convert_parts;
mod datex_hash;
pub mod datex_native;
mod datex_native_structural;
pub mod equality;
pub mod get_core_lib_type_id;
pub mod get_datex_type;
pub mod ops;
mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod try_clone;
mod value_access;

#[derive(Debug, Clone, PartialEq, Eq, Hash, BinRead, BinWrite)]
#[brw(little)]
pub struct Boolean(
    #[br(map = |x: u8| x != 0)]
    #[bw(map = |b: &bool| if *b { 1u8 } else { 0u8 })]
    pub bool,
);

impl Boolean {
    pub fn new(value: bool) -> Self {
        Boolean(value)
    }
}

impl Boolean {
    pub fn as_bool(&self) -> bool {
        self.0
    }
}
impl Boolean {
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }
    pub fn is_true(&self) -> bool {
        self.0
    }
    pub fn is_false(&self) -> bool {
        !self.0
    }
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
    pub fn as_str(&self) -> &str {
        if self.0 { "true" } else { "false" }
    }
}

impl Display for Boolean {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::write!(f, "{}", self.0)
    }
}

impl From<bool> for Boolean {
    fn from(v: bool) -> Self {
        Boolean(v)
    }
}

// new into
impl<T: Into<ValueContainer>> TryFrom<Option<T>> for Boolean {
    type Error = ValueError;
    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        match value {
            Some(v) => {
                let boolean: ValueContainer = v.into();
                boolean
                    .try_into_value()
                    .map_err(|_| ValueError::TypeConversionError)
            }
            None => Err(ValueError::IsVoid),
        }
    }
}
