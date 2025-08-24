#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberParseError {
    InvalidFormat,
    OutOfRange,
}

impl std::fmt::Display for NumberParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberParseError::InvalidFormat => {
                write!(f, "The number format is invalid.")
            }
            NumberParseError::OutOfRange => {
                write!(f, "The number is out of range for the specified type.")
            }
        }
    }
}
