#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberParseError {
    InvalidFormat,
    OutOfRange,
}

impl core::fmt::Display for NumberParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
