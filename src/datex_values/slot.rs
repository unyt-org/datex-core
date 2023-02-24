use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct SlotIdentifier {
	pub index: u16,
}



pub mod internal_slot {
    use super::SlotIdentifier;

	pub const THIS:SlotIdentifier       = SlotIdentifier {index: 0xff00};
	pub const IT:SlotIdentifier         = SlotIdentifier {index: 0xff01};
	pub const PUBLIC:SlotIdentifier     = SlotIdentifier {index: 0xff02};
	pub const FROM:SlotIdentifier       = SlotIdentifier {index: 0xff03};
	pub const ENDPOINT:SlotIdentifier   = SlotIdentifier {index: 0xff04};
	pub const LOCATION:SlotIdentifier   = SlotIdentifier {index: 0xff05};
	pub const META:SlotIdentifier       = SlotIdentifier {index: 0xff06};
	pub const ENV:SlotIdentifier        = SlotIdentifier {index: 0xff07};
	pub const RESULT:SlotIdentifier     = SlotIdentifier {index: 0xff08};
	pub const SUB_RESULT:SlotIdentifier = SlotIdentifier {index: 0xff09};
	pub const ENTRYPOINT:SlotIdentifier = SlotIdentifier {index: 0xff0a};
	pub const STD:SlotIdentifier        = SlotIdentifier {index: 0xff0b};

	pub const OBJ_WRITE:SlotIdentifier  = SlotIdentifier {index: 0xfef0};
	pub const OBJ_READ:SlotIdentifier   = SlotIdentifier {index: 0xfef1};
	pub const OBJ_EXEC:SlotIdentifier   = SlotIdentifier {index: 0xfef2};
}


/**
 * global slot address space:
 * 0xff00 - 0xffff (255) reserved internal slots (#result, #location, #it, ...)
 * 0xf000 - 0xf9ff (2559) use for scope value transfers
 * 
 * object slot address space
 * 0xfef0 - 0xfeff (15) reserved internal object slots (#read, #write)
 * 0xfa00 - 0xfeef (1263) use for object slots
 * 
 * free slots (for variables):
 * 0x0000 - 0xefff (61439)
 */

pub mod internal_slot_address_space {
	pub const RESERVED:(u16,u16) 				= (0xff00, 0xffff);
	pub const RESERVED_OBJECT_SLOTS:(u16,u16) 	= (0xfef0, 0xfeff);

	pub const SCOPE_VAL_TRANSFER:(u16,u16) 		= (0xf000, 0xf9ff);
	pub const OBJECT_SLOTS:(u16,u16) 			= (0xfa00, 0xfeef);
	pub const UNASSIGNED:(u16,u16) 				= (0x0000, 0xefff);
}

impl Default for SlotIdentifier {
    fn default() -> Self { SlotIdentifier {index:0} }
}

impl SlotIdentifier {

	pub fn new(index:u16) -> SlotIdentifier {
		SlotIdentifier {index}
	}

	pub fn to_string(&self) -> String {
		match *self {
			internal_slot::THIS => {"#this".to_string()},
			internal_slot::IT => {"#it".to_string()},
			internal_slot::PUBLIC => {"#public".to_string()},
			internal_slot::FROM => {"#from".to_string()},
			internal_slot::ENDPOINT => {"#endpoint".to_string()},
			internal_slot::LOCATION => {"#location".to_string()},
			internal_slot::META => {"#meta".to_string()},
			internal_slot::ENV => {"#env".to_string()},
			internal_slot::RESULT => {"#result".to_string()},
			internal_slot::SUB_RESULT => {"#sub_result".to_string()},
			internal_slot::ENTRYPOINT => {"#entrypoint".to_string()},
			internal_slot::STD => {"#std".to_string()},

			internal_slot::OBJ_READ => {"#read".to_string()},
			internal_slot::OBJ_WRITE => {"#write".to_string()},
			internal_slot::OBJ_EXEC => {"#exec".to_string()},

			_ => format!("#{:X}", self.index)
		}
    }

	pub fn is_reserved(&self) -> bool {
		self.index >= internal_slot_address_space::RESERVED_OBJECT_SLOTS.0 && self.index <= internal_slot_address_space::RESERVED.1
	}

	pub fn is_object_slot(&self) -> bool {
		(self.index >= internal_slot_address_space::OBJECT_SLOTS.0 && self.index <= internal_slot_address_space::OBJECT_SLOTS.1) ||
		(self.index >= internal_slot_address_space::RESERVED_OBJECT_SLOTS.0 && self.index <= internal_slot_address_space::RESERVED_OBJECT_SLOTS.1)
	}
}

impl fmt::Display for SlotIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
