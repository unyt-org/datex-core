use std::{fmt::Display, ops::Not};

use super::{
    super::core_value_trait::CoreValueTrait,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bool(pub bool);

impl Bool {
    pub fn as_bool(&self) -> bool {
        self.0
    }
}
impl Bool {
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }
}

impl Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CoreValueTrait for Bool {
}


impl From<bool> for Bool {
    fn from(v: bool) -> Self {
        Bool(v)
    }
}

impl Not for Bool {
    type Output = Bool;

    fn not(self) -> Self::Output {
        Bool(!self.0)
    }
}