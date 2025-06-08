use std::{fmt::Display, ops::Not};

use crate::datex_values::traits::soft_eq::SoftEq;

use super::super::core_value_trait::CoreValueTrait;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boolean(pub bool);

impl Boolean {
    pub fn as_bool(&self) -> bool {
        self.0
    }
}
impl Boolean {
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }
}

impl Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CoreValueTrait for Boolean {}

impl SoftEq for Boolean {
    fn soft_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl From<bool> for Boolean {
    fn from(v: bool) -> Self {
        Boolean(v)
    }
}

impl Not for Boolean {
    type Output = Boolean;

    fn not(self) -> Self::Output {
        Boolean(!self.0)
    }
}
