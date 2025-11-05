use crate::crypto::uuid::generate_uuid;
use crate::stdlib::string::String;
use core::fmt::Display;
use core::prelude::rust_2024::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UUID(String);

impl UUID {
    pub fn new() -> UUID {
        UUID::default()
    }
    pub fn from_string(uuid: String) -> UUID {
        UUID(uuid)
    }
}

impl Default for UUID {
    fn default() -> Self {
        UUID(generate_uuid())
    }
}

impl Display for UUID {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::write!(f, "{}", self.0)
    }
}
