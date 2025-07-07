use core::fmt;
use serde::de::Error;
use serde::ser::StdError;
use serde::ser::{self};
use std::fmt::Display;
use std::io;

// TODO: Add deserialization error and wrap compiler error and execution error into it

#[derive(Debug)]
pub struct SerializationError(pub String);
impl ser::Error for SerializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerializationError(msg.to_string())
    }
}
impl Error for SerializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerializationError(msg.to_string())
    }
}

impl From<io::Error> for SerializationError {
    fn from(e: io::Error) -> Self {
        SerializationError(e.to_string())
    }
}
impl StdError for SerializationError {}
impl Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SerializationError: {}", self.0)
    }
}
