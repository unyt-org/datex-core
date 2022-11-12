use core::fmt::Write;
use std::fmt;

use regex::Regex;
use lazy_static::lazy_static;

use super::Value;

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

}

impl PrimitiveValue {
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