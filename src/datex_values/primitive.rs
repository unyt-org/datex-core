use core::fmt::Write;
use std::fmt;

use regex::Regex;
use lazy_static::lazy_static;

use crate::global::binary_codes::BinaryCode;

use super::{Value, Error, ValueResult};

#[derive(Clone)]
pub enum PrimitiveValue {
	INT_8(i8),
	INT_16(i16),
	INT_32(i32),
	UINT_32(u32),
	INT_64(i64),
	FLOAT_64(f64),
	TEXT(String),
	BUFFER(Vec<u8>),
	BOOLEAN(bool),
	NULL,
	VOID
}

impl Default for PrimitiveValue {
    fn default() -> Self { PrimitiveValue::VOID }
}

impl fmt::Display for PrimitiveValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}

fn escape_string(value:&String) -> String {
	let mut string = str::replace(
		&str::replace(value, "\\", "\\\\"),
	 "\"", "\\\"");
	// TODO: only if formatted
	return str::replace(&string, "\n", "\\n");
}

impl Value for PrimitiveValue {
	fn to_string(&self) -> String {
		match &self {
			PrimitiveValue::INT_8(value) => value.to_string(),
			PrimitiveValue::INT_16(value) => value.to_string(),
			PrimitiveValue::INT_32(value) => value.to_string(),
			PrimitiveValue::UINT_32(value) => value.to_string(),
			PrimitiveValue::INT_64(value) => value.to_string(),
			PrimitiveValue::FLOAT_64(value) => {
				if value.is_infinite() {
					if value.is_sign_negative() {return "-infinity".to_string()}
					else {return "infinity".to_string()}
				}
				else if value.is_nan() {return "nan".to_string()}
				else {return value.to_string()}
			},
			PrimitiveValue::TEXT(value) => {
				let string = escape_string(value);
				return format!("\"{string}\"");
			}
			PrimitiveValue::BUFFER(value) => {
				let n = value.len();

				let mut s = String::with_capacity(2 * n);
				for byte in value {
					write!(s, "{:02X}", byte).expect("could not parse buffer")
				}
				return format!("`{s}`");
			},
			PrimitiveValue::BOOLEAN(value) => value.to_string(),
			PrimitiveValue::VOID => "void".to_string(),
			PrimitiveValue::NULL => "null".to_string()
		}
    }


	fn binary_operation(&self, code: BinaryCode, other: Box<dyn Value>) -> ValueResult {
		if other.is::<PrimitiveValue>() {
			let other_prim = other.downcast_ref::<PrimitiveValue>().expect("error casting stack value to primitive");

			return match 
				match code {
					BinaryCode::ADD => self.sum(other_prim),
					BinaryCode::SUBTRACT => self.difference(other_prim),

					_ => Err(Error {message:"invalid binary operation".to_string()})
				} 
			{
				Ok(result) => Ok(Box::new(result)),
				Err(err) => Err(err)
			}
	
		}

		return Err(Error {message:"invalid binary operation".to_string()})
    }

}

impl PrimitiveValue {

	fn sum(&self, other: &PrimitiveValue) -> Result<PrimitiveValue,Error> {
		if self.is_number() && other.is_number() {
			match self {
				PrimitiveValue::INT_8(val) 	=> Ok(PrimitiveValue::INT_8   (val + other.get_as_integer() as i8)),
				PrimitiveValue::INT_16(val) 	=> Ok(PrimitiveValue::INT_16  (val + other.get_as_integer() as i16)),
				PrimitiveValue::INT_32(val) 	=> Ok(PrimitiveValue::INT_32  (val + other.get_as_integer() as i32)),
				PrimitiveValue::INT_64(val) 	=> Ok(PrimitiveValue::INT_64  (val + other.get_as_integer() as i64)),
				PrimitiveValue::FLOAT_64(val) => Ok(PrimitiveValue::FLOAT_64(val + other.get_as_float())),
				_ => Err(Error {message:"cannot perform an add operation".to_string()})
			}
		}
		else {return Err(Error {message:"cannot perform an add operation".to_string()})}
	}

	fn difference(&self, other: &PrimitiveValue) -> Result<PrimitiveValue,Error> {
		if self.is_number() && other.is_number() {
			match self {
				PrimitiveValue::INT_8(val) 	=> Ok(PrimitiveValue::INT_8   (val - other.get_as_integer() as i8)),
				PrimitiveValue::INT_16(val) 	=> Ok(PrimitiveValue::INT_16  (val - other.get_as_integer() as i16)),
				PrimitiveValue::INT_32(val) 	=> Ok(PrimitiveValue::INT_32  (val - other.get_as_integer() as i32)),
				PrimitiveValue::INT_64(val) 	=> Ok(PrimitiveValue::INT_64  (val - other.get_as_integer() as i64)),
				PrimitiveValue::FLOAT_64(val) => Ok(PrimitiveValue::FLOAT_64(val - other.get_as_float())),
				_ => Err(Error {message:"cannot perform a subtract operation".to_string()})
			}
		}
		else {return Err(Error {message:"cannot perform a subtract operation".to_string()})}
	}

	fn is_number(&self) -> bool {
		match &self {
			PrimitiveValue::INT_8(_) => true,
			PrimitiveValue::INT_16(_) => true,
			PrimitiveValue::INT_32(_) => true,
			PrimitiveValue::INT_64(_) => true,
			PrimitiveValue::FLOAT_64(_) => true,
			_ => false
		}
	}

	
	fn get_as_integer(&self) -> isize {
		match &self {
			PrimitiveValue::INT_8(value) => *value as isize,
			PrimitiveValue::INT_16(value) => *value as isize,
			PrimitiveValue::INT_32(value) => *value as isize,
			PrimitiveValue::INT_64(value) => *value as isize,
			PrimitiveValue::FLOAT_64(value) => *value as isize,
			_ => 0
		}
	}

	fn get_as_float(&self) -> f64 {
		match &self {
			PrimitiveValue::INT_8(value) => *value as f64,
			PrimitiveValue::INT_16(value) => *value as f64,
			PrimitiveValue::INT_32(value) => *value as f64,
			PrimitiveValue::INT_64(value) => *value as f64,
			PrimitiveValue::FLOAT_64(value) => *value as f64,
			_ => 0.0
		}
	}

	// returns a string, omits quotes if possible (for keys)
	pub fn to_key_string(&self) -> String  {

		lazy_static! {
			static ref KEY_CAN_OMIT_QUOTES:Regex = Regex::new(r"^[A-Za-z_][A-Za-z_0-9]?$").unwrap();
		}

		match &self {
			PrimitiveValue::TEXT(value) => {
				let string = escape_string(value);
				// key:
				if KEY_CAN_OMIT_QUOTES.is_match(&string) {
					return string;
				}
				// "key":
				else {return format!("\"{string}\"");}
			}
			_ => Value::to_string(self)
		}
		// [A-Za-z_][A-Za-z_0-9]?
	}
}