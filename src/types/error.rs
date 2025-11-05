use crate::stdlib::string::String;
use core::fmt::Display;
use core::prelude::rust_2024::*;

#[derive(Debug)]
pub enum IllegalTypeError {
    MutableRef(String),
    TypeNotFound,
}

impl Display for IllegalTypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IllegalTypeError::MutableRef(val) => {
                core::write!(f, "Cannot use mutable reference as type: {}", val)
            }
            IllegalTypeError::TypeNotFound => {
                core::write!(f, "Core type not found in memory")
            }
        }
    }
}
