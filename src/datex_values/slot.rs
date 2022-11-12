use core::fmt::Write;
use std::fmt;

#[derive(Clone)]
pub enum SlotIdentifier {
	ID(u16),
	NAME(String)
}

impl Default for SlotIdentifier {
    fn default() -> Self { SlotIdentifier::ID(0) }
}

impl SlotIdentifier {
	pub fn to_string(&self) -> String {
		match &self {
			SlotIdentifier::ID(value) => {
				return format!("#{value}");
			},
			SlotIdentifier::NAME(value) => {
				return format!("#{value}");
			}
		}
    }
}

impl fmt::Display for SlotIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
