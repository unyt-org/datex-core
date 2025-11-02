use core::prelude::rust_2024::*;
use core::result::Result;
use crate::crypto::uuid::generate_uuid;
use core::fmt::Display;
use crate::stdlib::string::String;

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
