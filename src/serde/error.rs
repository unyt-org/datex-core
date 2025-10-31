use core::fmt;
use serde::de::Error;
use serde::ser::StdError;
use serde::ser::{self};
use core::fmt::Display;
use crate::stdlib::io;

use crate::compiler::error::{CompilerError, SpannedCompilerError};
use crate::runtime::execution::ExecutionError;

#[derive(Debug)]
pub enum SerializationError {
    Custom(String),
    CanNotSerialize(String),
    CompilerError(CompilerError),
}
impl ser::Error for SerializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerializationError::Custom(msg.to_string())
    }
}
impl Error for SerializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerializationError::Custom(msg.to_string())
    }
}

impl From<io::Error> for SerializationError {
    fn from(e: io::Error) -> Self {
        SerializationError::Custom(e.to_string())
    }
}
impl From<CompilerError> for SerializationError {
    fn from(e: CompilerError) -> Self {
        SerializationError::CompilerError(e)
    }
}
impl StdError for SerializationError {}
impl Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationError::Custom(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            SerializationError::CanNotSerialize(msg) => {
                write!(f, "Can not serialize value: {}", msg)
            }
            SerializationError::CompilerError(err) => {
                write!(f, "Compiler error: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum DeserializationError {
    Custom(String),
    CanNotDeserialize(String),
    ExecutionError(ExecutionError),
    CanNotReadFile(String),
    CompilerError(SpannedCompilerError),
    NoStaticValueFound,
}
impl ser::Error for DeserializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DeserializationError::Custom(msg.to_string())
    }
}
impl Error for DeserializationError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DeserializationError::Custom(msg.to_string())
    }
}

impl From<io::Error> for DeserializationError {
    fn from(e: io::Error) -> Self {
        DeserializationError::Custom(e.to_string())
    }
}
impl From<ExecutionError> for DeserializationError {
    fn from(e: ExecutionError) -> Self {
        DeserializationError::ExecutionError(e)
    }
}

impl StdError for DeserializationError {}
impl Display for DeserializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeserializationError::Custom(msg) => {
                write!(f, "Deserialization error: {}", msg)
            }
            DeserializationError::CanNotDeserialize(msg) => {
                write!(f, "Can not deserialize value: {}", msg)
            }
            DeserializationError::ExecutionError(err) => {
                write!(f, "Execution error: {}", err)
            }
            DeserializationError::CanNotReadFile(msg) => {
                write!(f, "Can not read file: {}", msg)
            }
            DeserializationError::CompilerError(err) => {
                write!(f, "Compiler error: {}", err)
            }
            DeserializationError::NoStaticValueFound => {
                write!(f, "No static value found in script")
            }
        }
    }
}