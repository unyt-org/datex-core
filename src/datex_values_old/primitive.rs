use crate::{datex_values::core_values::endpoint::Endpoint, stdlib::fmt};
use core::fmt::Write;

use lazy_static::lazy_static;
use num_bigint::BigInt;
use regex::Regex;

use crate::global::binary_codes::BinaryCode;

use super::{primitives::time::Time, Error, Quantity, Url, Value, ValueResult};

#[derive(Clone)]
// Native values (array, object)
#[derive(Default)]
pub enum PrimitiveValue {
    Int8(i8),
    Uint8(u8),
    Int16(i16),
    Int32(i32),
    UInt16(u16),
    UInt32(u32),
    Int64(i64),
    Float64(f64),
    BigInt(BigInt),
    Text(String),
    Buffer(Vec<u8>),
    Boolean(bool),
    Quantity(Quantity),
    Time(Time),
    Endpoint(Endpoint),
    Url(Url),
    Null,
    #[default]
    Void,
}

impl fmt::Display for PrimitiveValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}

fn escape_string(value: &String) -> String {
    // TODO: \n only if formatted?

    // TODO:
    // name = Regex::new(r"[\u0000-\u0008\u000B-\u001F\u007F-\u009F\u2000-\u200F\u2028-\u202F\u205F-\u206F\u3000\uFEFF\u{E0100}-\u{E01EF}]").
    // 	unwrap().
    // 	replace_all(&name, "").to_string();

    value
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\u{0008}", "\\b")
        .replace("\u{000c}", "\\f")
        .replace("\u{001b}", "\\u001b")
}

impl Value for PrimitiveValue {
    fn to_string(&self) -> String {
        match &self {
            PrimitiveValue::Int8(value) => value.to_string(),
            PrimitiveValue::Uint8(value) => value.to_string(),
            PrimitiveValue::Int16(value) => value.to_string(),
            PrimitiveValue::UInt16(value) => value.to_string(),
            PrimitiveValue::Int32(value) => value.to_string(),
            PrimitiveValue::UInt32(value) => value.to_string(),
            PrimitiveValue::Int64(value) => value.to_string(),
            PrimitiveValue::BigInt(value) => value.to_string(),
            PrimitiveValue::Float64(value) => {
                if value.is_infinite() {
                    if value.is_sign_negative() {
                        "-infinity".to_string()
                    } else {
                        "infinity".to_string()
                    }
                } else if value.is_nan() {
                    return "nan".to_string();
                } else {
                    let mut string = value.to_string();
                    if !string.contains('.') {
                        string += ".0";
                    }
                    return string;
                }
            }
            PrimitiveValue::Text(value) => {
                let string = escape_string(value);
                format!("\"{string}\"")
            }
            PrimitiveValue::Buffer(value) => {
                let n = value.len();

                let mut s = String::with_capacity(2 * n);
                for byte in value {
                    write!(s, "{byte:02X}").expect("could not parse buffer")
                }
                format!("`{s}`")
            }
            PrimitiveValue::Boolean(value) => value.to_string(),
            PrimitiveValue::Void => "void".to_string(),
            PrimitiveValue::Null => "null".to_string(),
            PrimitiveValue::Quantity(value) => value.to_string(false),
            PrimitiveValue::Endpoint(value) => value.to_string(),
            PrimitiveValue::Time(value) => value.to_string(),
            PrimitiveValue::Url(value) => value.to_string(),
        }
    }

    fn binary_operation(
        &self,
        code: BinaryCode,
        other: Box<dyn Value>,
    ) -> ValueResult {
        if other.is::<PrimitiveValue>() {
            let other_prim = other
                .downcast_ref::<PrimitiveValue>()
                .expect("error casting stack value to primitive");

            return match match code {
                BinaryCode::ADD => self.sum(other_prim),
                BinaryCode::SUBTRACT => self.difference(other_prim),
                BinaryCode::MULTIPLY => self.product(other_prim),
                BinaryCode::DIVIDE => self.quotient(other_prim),
                BinaryCode::MODULO => self.modulo(other_prim),
                BinaryCode::POWER => self.power(other_prim),

                _ => Err(Error {
                    message: "invalid binary operation".to_string(),
                }),
            } {
                Ok(result) => Ok(Box::new(result)),
                Err(err) => Err(err),
            };
        }

        Err(Error {
            message: "invalid binary operation".to_string(),
        })
    }

    fn cast(&self, dx_type: super::Type) -> ValueResult {
        // TODO: type check
        if dx_type.name == "text" {
            Ok(Box::new(PrimitiveValue::Text(Value::to_string(self))))
        } else {
            Err(Error {
                message: format!("cannot cast to {dx_type}"),
            })
        }
    }
}

impl PrimitiveValue {
    // special colorized form
    pub fn to_string_colorized(&self) -> String {
        match &self {
            PrimitiveValue::Quantity(value) => value.to_string(true),
            PrimitiveValue::Endpoint(value) => value.to_string(),
            _ => Value::to_string(self),
        }
    }

    fn sum(&self, other: &PrimitiveValue) -> Result<PrimitiveValue, Error> {
        if self.is_number() && other.is_number() {
            match self {
                PrimitiveValue::Int8(val) => {
                    Ok(PrimitiveValue::Int8(val + other.get_as_integer() as i8))
                }
                PrimitiveValue::Int16(val) => Ok(PrimitiveValue::Int16(
                    val + other.get_as_integer() as i16,
                )),
                PrimitiveValue::Int32(val) => Ok(PrimitiveValue::Int32(
                    val + other.get_as_integer() as i32,
                )),
                PrimitiveValue::Int64(val) => Ok(PrimitiveValue::Int64(
                    val + other.get_as_integer() as i64,
                )),
                PrimitiveValue::Float64(val) => {
                    Ok(PrimitiveValue::Float64(val + other.get_as_float()))
                }
                _ => Err(Error {
                    message: "cannot perform an add operation".to_string(),
                }),
            }
        } else if self.is_text() && other.is_text() {
            return Ok(PrimitiveValue::Text(
                self.get_as_text().to_owned() + other.get_as_text(),
            ));
        } else {
            return Err(Error {
                message: "cannot perform an add operation".to_string(),
            });
        }
    }

    fn difference(
        &self,
        other: &PrimitiveValue,
    ) -> Result<PrimitiveValue, Error> {
        if self.is_number() && other.is_number() {
            match self {
                PrimitiveValue::Int8(val) => {
                    Ok(PrimitiveValue::Int8(val - other.get_as_integer() as i8))
                }
                PrimitiveValue::Int16(val) => Ok(PrimitiveValue::Int16(
                    val - other.get_as_integer() as i16,
                )),
                PrimitiveValue::Int32(val) => Ok(PrimitiveValue::Int32(
                    val - other.get_as_integer() as i32,
                )),
                PrimitiveValue::Int64(val) => Ok(PrimitiveValue::Int64(
                    val - other.get_as_integer() as i64,
                )),
                PrimitiveValue::Float64(val) => {
                    Ok(PrimitiveValue::Float64(val - other.get_as_float()))
                }
                _ => Err(Error {
                    message: "cannot perform a subtract operation".to_string(),
                }),
            }
        } else {
            Err(Error {
                message: "cannot perform a subtract operation".to_string(),
            })
        }
    }

    fn product(&self, other: &PrimitiveValue) -> Result<PrimitiveValue, Error> {
        if self.is_number() && other.is_number() {
            match self {
                PrimitiveValue::Int8(val) => {
                    Ok(PrimitiveValue::Int8(val * other.get_as_integer() as i8))
                }
                PrimitiveValue::Int16(val) => Ok(PrimitiveValue::Int16(
                    val * other.get_as_integer() as i16,
                )),
                PrimitiveValue::Int32(val) => Ok(PrimitiveValue::Int32(
                    val * other.get_as_integer() as i32,
                )),
                PrimitiveValue::Int64(val) => Ok(PrimitiveValue::Int64(
                    val * other.get_as_integer() as i64,
                )),
                PrimitiveValue::Float64(val) => {
                    Ok(PrimitiveValue::Float64(val * other.get_as_float()))
                }
                _ => Err(Error {
                    message: "cannot perform a subtract operation".to_string(),
                }),
            }
        } else {
            Err(Error {
                message: "cannot perform a subtract operation".to_string(),
            })
        }
    }

    fn quotient(
        &self,
        other: &PrimitiveValue,
    ) -> Result<PrimitiveValue, Error> {
        if self.is_number() && other.is_number() {
            match self {
                PrimitiveValue::Int8(val) => {
                    Ok(PrimitiveValue::Int8(val / other.get_as_integer() as i8))
                }
                PrimitiveValue::Int16(val) => Ok(PrimitiveValue::Int16(
                    val / other.get_as_integer() as i16,
                )),
                PrimitiveValue::Int32(val) => Ok(PrimitiveValue::Int32(
                    val / other.get_as_integer() as i32,
                )),
                PrimitiveValue::Int64(val) => Ok(PrimitiveValue::Int64(
                    val / other.get_as_integer() as i64,
                )),
                PrimitiveValue::Float64(val) => {
                    Ok(PrimitiveValue::Float64(val / other.get_as_float()))
                }
                _ => Err(Error {
                    message: "cannot perform a subtract operation".to_string(),
                }),
            }
        } else {
            Err(Error {
                message: "cannot perform a subtract operation".to_string(),
            })
        }
    }

    fn modulo(&self, other: &PrimitiveValue) -> Result<PrimitiveValue, Error> {
        if self.is_number() && other.is_number() {
            match self {
                PrimitiveValue::Int8(val) => {
                    Ok(PrimitiveValue::Int8(val % other.get_as_integer() as i8))
                }
                PrimitiveValue::Int16(val) => Ok(PrimitiveValue::Int16(
                    val % other.get_as_integer() as i16,
                )),
                PrimitiveValue::Int32(val) => Ok(PrimitiveValue::Int32(
                    val % other.get_as_integer() as i32,
                )),
                PrimitiveValue::Int64(val) => Ok(PrimitiveValue::Int64(
                    val % other.get_as_integer() as i64,
                )),
                PrimitiveValue::Float64(val) => {
                    Ok(PrimitiveValue::Float64(val % other.get_as_float()))
                }
                _ => Err(Error {
                    message: "cannot perform a subtract operation".to_string(),
                }),
            }
        } else {
            Err(Error {
                message: "cannot perform a subtract operation".to_string(),
            })
        }
    }

    fn power(&self, other: &PrimitiveValue) -> Result<PrimitiveValue, Error> {
        if self.is_number() && other.is_number() {
            match self {
                PrimitiveValue::Int8(val) => Ok(PrimitiveValue::Int8(
                    val.pow(other.get_as_integer() as u32),
                )),
                PrimitiveValue::Int16(val) => Ok(PrimitiveValue::Int16(
                    val.pow(other.get_as_integer() as u32),
                )),
                PrimitiveValue::Int32(val) => Ok(PrimitiveValue::Int32(
                    val.pow(other.get_as_integer() as u32),
                )),
                PrimitiveValue::Int64(val) => Ok(PrimitiveValue::Int64(
                    val.pow(other.get_as_integer() as u32),
                )),
                PrimitiveValue::Float64(val) => Ok(PrimitiveValue::Float64(
                    val.powf(other.get_as_integer() as f64),
                )),
                _ => Err(Error {
                    message: "cannot perform a subtract operation".to_string(),
                }),
            }
        } else {
            Err(Error {
                message: "cannot perform a subtract operation".to_string(),
            })
        }
    }

    pub fn is_number(&self) -> bool {
        match &self {
            PrimitiveValue::Int8(_) => true,
            PrimitiveValue::Int16(_) => true,
            PrimitiveValue::Int32(_) => true,
            PrimitiveValue::Int64(_) => true,
            PrimitiveValue::Float64(_) => true,
            PrimitiveValue::BigInt(_) => true,
            _ => false,
        }
    }

    pub fn is_text(&self) -> bool {
        match &self {
            PrimitiveValue::Text(_) => true,
            _ => false,
        }
    }

    pub fn get_as_text(&self) -> &str {
        match &self {
            PrimitiveValue::Text(value) => value,
            _ => "",
        }
    }

    pub fn get_as_buffer(&self) -> Vec<u8> {
        match &self {
            PrimitiveValue::Buffer(value) => value.to_vec(),
            _ => Vec::new(),
        }
    }

    pub fn get_as_integer(&self) -> isize {
        match &self {
            PrimitiveValue::Int8(value) => *value as isize,
            PrimitiveValue::Uint8(value) => *value as isize,
            PrimitiveValue::UInt16(value) => *value as isize,
            PrimitiveValue::Int16(value) => *value as isize,
            PrimitiveValue::Int32(value) => *value as isize,
            PrimitiveValue::UInt32(value) => *value as isize,
            PrimitiveValue::Int64(value) => *value as isize,
            PrimitiveValue::Float64(value) => *value as isize,
            _ => 0,
        }
    }

    pub fn get_as_unsigned_integer(&self) -> usize {
        match &self {
            PrimitiveValue::Int8(value) => *value as usize,
            PrimitiveValue::Uint8(value) => *value as usize,
            PrimitiveValue::UInt16(value) => *value as usize,
            PrimitiveValue::Int16(value) => *value as usize,
            PrimitiveValue::Int32(value) => *value as usize,
            PrimitiveValue::UInt32(value) => *value as usize,
            PrimitiveValue::Int64(value) => *value as usize,
            PrimitiveValue::Float64(value) => *value as usize,
            _ => 0,
        }
    }

    pub fn get_as_float(&self) -> f64 {
        match &self {
            PrimitiveValue::Int8(value) => *value as f64,
            PrimitiveValue::Uint8(value) => *value as f64,
            PrimitiveValue::UInt16(value) => *value as f64,
            PrimitiveValue::Int16(value) => *value as f64,
            PrimitiveValue::Int32(value) => *value as f64,
            PrimitiveValue::UInt32(value) => *value as f64,
            PrimitiveValue::Int64(value) => *value as f64,
            PrimitiveValue::Float64(value) => *value,
            _ => 0.0,
        }
    }

    // returns a string, omits quotes if possible (for keys)
    pub fn to_key_string(&self) -> String {
        match &self {
            PrimitiveValue::Text(value) => {
                let string = escape_string(value);
                // key:
                if KEY_CAN_OMIT_QUOTES.is_match(&string) {
                    string
                }
                // "key":
                else {
                    format!("\"{string}\"")
                }
            }
            _ => Value::to_string(self),
        }
    }

    // returns true if not a text, or if the text only contains A-Z,0-9...
    pub fn can_omit_quotes(&self) -> bool {
        match &self {
            PrimitiveValue::Text(value) => {
                KEY_CAN_OMIT_QUOTES.is_match(&escape_string(value))
            }
            _ => true,
        }
    }
}

lazy_static! {
    static ref KEY_CAN_OMIT_QUOTES: Regex =
        Regex::new(r"^[A-Za-z_][A-Za-z_0-9]*$").unwrap();
}
