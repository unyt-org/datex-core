use std::fmt;
use crate::global::binary_codes::BinaryCode;
use super::{Value, ValueResult};
use core::fmt::Write;

pub struct Pointer {
	pub id_formatted: String
}

impl Pointer {

	pub const MAX_POINTER_ID_SIZE:usize = 26;
    pub const STATIC_POINTER_SIZE:usize = 18;

	pub fn from_id(id:Vec<u8>) -> Pointer {
		return Pointer {id_formatted: Pointer::normalize_id(id)}
	}

	fn normalize_id(id:Vec<u8>) -> String {
		let n = id.len();

		let mut s = String::with_capacity(2 * n);
		for byte in id {
			write!(s, "{:02X}", byte).expect("could not parse buffer")
		}
		return s;
	}

}

impl Value for Pointer {
    fn to_string(&self) -> String {
		return format!("${}", self.id_formatted);
    }

    fn binary_operation(&self, code: BinaryCode, other: Box<dyn Value>) -> ValueResult {
        todo!()
    }

    fn cast(&self, dx_type: super::Type) -> ValueResult {
        todo!()
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}