#[cfg(feature = "compiler")]
use crate::compiler::error::SpannedCompilerError;
#[cfg(feature = "parser")]
use crate::parser::errors::SpannedParserError;
use crate::{
    core_compiler::core_compilation_context::DXBWithSharedValues,
    prelude::*,
    runtime::{
        Runtime,
        execution::{ExecutionError, context::ScriptExecutionError},
    },
    traits::{
        classification::Classification,
        convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind},
    },
    values::value_container::{
        ValueContainer,
        borrowed_value_container::{
            BorrowedValueContainer, BorrowedValueContainerMut,
        },
    },
};

#[derive(Debug, PartialEq, Eq)]
pub enum DeserializationError {
    NoValue,
    InvalidValue,
    ExecutionError(Box<ExecutionError>),
    CanNotReadFile(String),
    #[cfg(feature = "parser")]
    ParserError(SpannedParserError),
    #[cfg(feature = "compiler")]
    CompilerError(Box<SpannedCompilerError>),
    NoStaticValueFound,
}

impl From<ExecutionError> for DeserializationError {
    fn from(err: ExecutionError) -> DeserializationError {
        DeserializationError::ExecutionError(Box::new(err))
    }
}

#[cfg(feature = "parser")]
impl From<SpannedParserError> for DeserializationError {
    fn from(err: SpannedParserError) -> DeserializationError {
        DeserializationError::ParserError(err)
    }
}

impl From<ScriptExecutionError> for DeserializationError {
    fn from(err: ScriptExecutionError) -> DeserializationError {
        match err {
            ScriptExecutionError::ExecutionError(e) => {
                DeserializationError::ExecutionError(e)
            }
            #[cfg(feature = "compiler")]
            ScriptExecutionError::CompilerError(e) => {
                DeserializationError::CompilerError(e)
            }
        }
    }
}

/// This traits allows converting types to and from [ValueContainer]s
/// No value conversions are performed, only downcasts to more specific types or upcasts to more general types are performed
pub trait ConvertValueContainer {
    /// Convert the value to a [ValueContainer]
    fn to_value_container(self) -> ValueContainer;

    /// Borrow the value as a [BorrowedValueContainer]
    fn as_borrowed_value_container(&self) -> BorrowedValueContainer<'_>;

    /// Borrow the value as a mutable [BorrowedValueContainer]
    fn as_borrowed_value_container_mut(
        &mut self,
    ) -> BorrowedValueContainerMut<'_>;

    /// Tries to downcast a [ValueContainer] into [Self]
    fn try_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ValueContainer>
    where
        Self: Sized;

    /// Tries to downcast a [ValueContainer] into a reference of [Self]
    fn try_borrow_from_value_container(
        value_container: &ValueContainer,
    ) -> Result<&Self, ()>
    where
        Self: Sized;

    /// Tries to downcast a [ValueContainer] into a mutable reference of [Self]
    fn try_borrow_mut_from_value_container(
        value_container: &mut ValueContainer,
    ) -> Result<&mut Self, ()>
    where
        Self: Sized;

    /// Deserialize a value of type T from a byte slice containing DXB data
    fn try_from_bytes(
        dxb: Vec<u8>,
        runtime: &Runtime,
    ) -> Result<Self, DeserializationError>
    where
        Self: Sized + FromParts,
    {
        let value = runtime.execute_dxb_sync(
            DXBWithSharedValues::new(dxb, vec![]),
            None,
            None,
            true,
        )?;
        if let Some(value) = value {
            let config = Self::try_convert_from_value_container(value)
                .map_err(|_| DeserializationError::InvalidValue)?;
            Ok(config)
        } else {
            Err(DeserializationError::NoValue)
        }
    }

    #[cfg(feature = "compiler")]
    fn try_from_script(
        script: &str,
        runtime: &Runtime,
    ) -> Result<Self, DeserializationError>
    where
        Self: Sized + FromParts,
    {
        let value = runtime.execute_sync(script, &[], None)?;

        if let Some(value) = value {
            let config = Self::try_convert_from_value_container(value)
                .map_err(|_| DeserializationError::InvalidValue)?;
            Ok(config)
        } else {
            Err(DeserializationError::NoValue)
        }
    }

    #[cfg(all(feature = "std", feature = "compiler"))]
    fn try_from_dx_file(
        path: &std::path::Path,
        runtime: &Runtime,
    ) -> Result<Self, DeserializationError>
    where
        Self: Sized + FromParts,
    {
        let script = std::fs::read_to_string(path)
            .map_err(|e| DeserializationError::CanNotReadFile(e.to_string()))?;
        Self::try_from_script(&script, runtime)
    }

    /// Create a value from a DX script string
    /// This will extract a static value from the script without executing it
    /// and use that value for deserialization
    /// If no static value is found, an error is returned
    /// This is useful for deserializing simple values like integer, text, map and list
    /// without the need to execute the script
    /// Note: This does not support expressions or computations in the script
    /// For example, the script `{ "key": 42 }` will work, but the script `{ "key": 40 + 2 }` will not
    /// because the latter requires execution to evaluate the expression
    /// and extract the value
    #[cfg(feature = "compiler")]
    fn try_from_static_script(
        script: &str,
    ) -> Result<Self, DeserializationError>
    where
        Self: Sized,
    {
        let value = crate::compiler::extract_static_value_from_script(script)?
            .ok_or(DeserializationError::NoStaticValueFound)?;
        Self::try_from_value_container(value)
            .map_err(|_| DeserializationError::InvalidValue)
    }

    /// Tries to cast a [ValueContainer] into the specified type [T].
    /// First, it attempts a direct conversion (downcast) of the [ValueContainer]
    /// into the type [T].
    /// If that fails, it tries to convert the [ValueContainer] into map or list parts and then into the type [T].
    fn try_convert_from_value_container(
        value_container: ValueContainer,
    ) -> Result<Self, ()>
    where
        Self: FromParts + Sized,
    {
        // first try to convert (downcast) the value container directly into the type T
        let val = Self::try_from_value_container(value_container);
        match val {
            Ok(value) => Ok(value),
            // otherwise, try to convert the value container into map or list parts and then into the type T
            Err(value_container) => {
                let classification =
                    value_container.unresolved_classification();
                let tag = classification.tag_str();

                match value_container.parts_kind() {
                    PartsKind::Map => {
                        let map_parts =
                            Box::new(value_container).try_into_map_parts();
                        if let Ok(map) = map_parts {
                            Self::try_from_map_parts_with_tag(map, tag)
                        } else {
                            Err(())
                        }
                    }
                    PartsKind::List => {
                        let list_parts =
                            Box::new(value_container).try_into_list_parts();
                        if let Ok(list) = list_parts {
                            Self::try_from_list_parts_with_tag(list, tag)
                        } else {
                            Err(())
                        }
                    }
                    PartsKind::SingleValue => {
                        let single_value_parts =
                            Box::new(value_container).try_into_single_value();
                        if let Ok(single_value) = single_value_parts {
                            Self::try_from_single_value_with_tag(
                                single_value,
                                tag,
                            )
                        } else {
                            Err(())
                        }
                    }
                    PartsKind::None => Err(()),
                }
            }
        }
    }
}
