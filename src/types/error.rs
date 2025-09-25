use std::fmt::Display;

#[derive(Debug)]
pub enum IllegalTypeError {
    MutableRef(String),
    TypeNotFound,
}

impl Display for IllegalTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IllegalTypeError::MutableRef(val) => {
                write!(f, "Cannot use mutable reference as type: {}", val)
            }
            IllegalTypeError::TypeNotFound => {
                write!(f, "Core type not found in memory")
            }
        }
    }
}
