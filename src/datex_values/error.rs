use std::fmt;

use crate::global::binary_codes::BinaryCode;

use super::{Value, ValueResult};

pub struct Error {
	pub message: String
}

impl Value for Error {
    fn to_string(&self) -> String {
		return format!("!{}", self.message);
    }

    fn binary_operation(&self, code: BinaryCode, other: Box<dyn Value>) -> ValueResult {
        todo!()
    }

    fn cast(&self, dx_type: super::Type) -> ValueResult {
        todo!()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}