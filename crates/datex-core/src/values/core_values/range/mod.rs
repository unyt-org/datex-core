//! This module contains the implementation of the [Range] struct, which represents a range of values in the type system.
//! A [Range] consists of a lower bound (inclusive) and an upper bound (exclusive) and can be used to represent ranges of numbers, ... (TBD)

use crate::values::value_container::ValueContainer;
use alloc::boxed::Box;
use core::fmt;
mod child_iterator;
mod classification;
pub mod convert_parts;
mod datex_hash;
mod datex_native;
mod datex_native_structural;
mod get_core_lib_type_id;
mod get_datex_type;
pub mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
mod value_access;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Range {
    // lower bound (inclusive)
    pub start: Box<ValueContainer>,
    // upper bound (exclusive)
    pub end: Box<ValueContainer>,
}

impl Range {
    pub fn new(start: ValueContainer, end: ValueContainer) -> Self {
        Self {
            start: Box::new(start),
            end: Box::new(end),
        }
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{}..{}", self.start, self.end)
    }
}
