use core::fmt::Write;
use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct SlotIdentifier {
	pub index: u16
}

pub mod internal_slot {
    use super::SlotIdentifier;

	pub const THIS:SlotIdentifier     = SlotIdentifier {index: 0xf000};
	pub const IT:SlotIdentifier       = SlotIdentifier {index: 0xf001};
	pub const PUBLIC:SlotIdentifier   = SlotIdentifier {index: 0xf002};

}

impl Default for SlotIdentifier {
    fn default() -> Self { SlotIdentifier {index:0} }
}

impl SlotIdentifier {
	pub fn to_string(&self) -> String {
		match *self {
			internal_slot::THIS => {"#this".to_string()},
			internal_slot::IT => {"#it".to_string()},
			internal_slot::PUBLIC => {"#public".to_string()},

			_ => format!("#{:X}", self.index)
		}
    }
}

impl fmt::Display for SlotIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
