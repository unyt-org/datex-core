use core::fmt::Write;
use std::fmt;


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

pub enum PrimitiveValue {
	INT_8(i8),
	INT_16(i16),
	INT_32(i32),
	INT_64(i64),
	FLOAT_64(f64),
	TEXT(String),
	BUFFER(Vec<u8>),
	BOOLEAN(bool),
	NULL,
	VOID
}



impl PrimitiveValue {
	pub fn to_string(&self) -> String {
		match &self {
			PrimitiveValue::INT_8(value) => value.to_string(),
			PrimitiveValue::INT_16(value) => value.to_string(),
			PrimitiveValue::INT_32(value) => value.to_string(),
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
				let mut string = str::replace(
					&str::replace(value, "\\", "\\\\"),
				 "\"", "\\\"");
				// TODO: only if formatted
				string = str::replace(value, "\n", "\\n");

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