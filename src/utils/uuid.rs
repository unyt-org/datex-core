use crate::crypto::uuid::generate_uuid;
use crate::stdlib::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UUID(String);

impl UUID {
    pub fn new() -> UUID {
        UUID::default()
    }
    pub fn to_string(&self) -> String {
        self.0.clone()
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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
