use std::fmt;

use crate::global::binary_codes::BinaryCode;

use super::{Error, Type};

pub trait Value: mopa::Any {
	fn to_string(&self) -> String;

	fn cast(&self, dx_type: Type) -> ValueResult;
	fn binary_operation(&self, code: BinaryCode, other: Box<dyn Value>) -> ValueResult;
}

impl fmt::Display for dyn Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}

mopafy!(Value);

pub type ValueResult = Result<Box<dyn Value>, Error>;