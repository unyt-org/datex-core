use crate::datex_values::traits::soft_eq::SoftEq;
use core::fmt;
use std::fmt::Display;
use std::hash::Hash;

use super::super::core_value_trait::CoreValueTrait;

#[derive(Debug, Clone, Eq)]
pub struct Null;

impl CoreValueTrait for Null {}
impl SoftEq for Null {
    fn soft_eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "null")
    }
}
impl PartialEq for Null {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Hash for Null {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Null has no state, so we can use a constant value
        0.hash(state);
    }
}
