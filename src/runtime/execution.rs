use super::stack::{ActiveValue, ScopeStack, ScopeType};
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::array::Array;
use crate::datex_values::core_values::object::Object;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::{ValueContainer, ValueError};
use crate::global::protocol_structures::instructions::{
    Float64Data, Instruction, Int16Data, Int8Data, ShortTextData,
};
use crate::parser::body;
use crate::parser::body::ParserError;
use std::fmt::Display;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    dxb_body: Vec<u8>,
    options: ExecutionOptions,
    index: usize,
    scope_stack: ScopeStack,
}

pub fn execute_dxb(
    dxb_body: Vec<u8>,
    options: ExecutionOptions,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let context = ExecutionContext {
        dxb_body,
        options,
        ..ExecutionContext::default()
    };
    execute_loop(context)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidProgramError {
    InvalidScopeClose,
    InvalidKeyValuePair,
}

impl Display for InvalidProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidProgramError::InvalidScopeClose => {
                write!(f, "Invalid scope close")
            }
            InvalidProgramError::InvalidKeyValuePair => {
                write!(f, "Invalid key-value pair")
            }
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    ParserError(ParserError),
    ValueError(ValueError),
    InvalidProgram(InvalidProgramError),
    Unknown,
    NotImplemented(String),
}

impl From<ParserError> for ExecutionError {
    fn from(error: ParserError) -> Self {
        ExecutionError::ParserError(error)
    }
}

impl From<ValueError> for ExecutionError {
    fn from(error: ValueError) -> Self {
        ExecutionError::ValueError(error)
    }
}

impl From<InvalidProgramError> for ExecutionError {
    fn from(error: InvalidProgramError) -> Self {
        ExecutionError::InvalidProgram(error)
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::ParserError(err) => {
                write!(f, "Parser error: {err}")
            }
            ExecutionError::Unknown => write!(f, "Unknown execution error"),
            ExecutionError::ValueError(err) => write!(f, "Value error: {err}"),
            ExecutionError::InvalidProgram(err) => {
                write!(f, "Invalid program error: {err}")
            }
            ExecutionError::NotImplemented(msg) => {
                write!(f, "Not implemented: {msg}")
            }
        }
    }
}

fn execute_loop(
    context: ExecutionContext,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let dxb_body = context.dxb_body;
    let mut scope_stack = context.scope_stack;

    let instruction_iterator = body::iterate_instructions(&dxb_body);

    for instruction in instruction_iterator {
        let instruction = instruction?;
        if context.options.verbose {
            println!("[Exec]: {instruction}");
        }

        let mut is_scope_start = false;

        let value: ActiveValue = match instruction {
            // boolean
            Instruction::True => true.into(),
            Instruction::False => false.into(),

            // integers
            Instruction::Int8(integer) => integer.0.into(),
            Instruction::Int16(integer) => integer.0.into(),
            Instruction::Int32(integer) => integer.0.into(),
            Instruction::Int64(integer) => integer.0.into(),
            Instruction::Int128(integer) => integer.0.into(),

            // unsigned integers
            Instruction::UInt8(integer) => integer.0.into(),
            Instruction::UInt16(integer) => integer.0.into(),
            Instruction::UInt32(integer) => integer.0.into(),
            Instruction::UInt64(integer) => integer.0.into(),
            Instruction::UInt128(integer) => integer.0.into(),

            // floats
            Instruction::Float64(Float64Data(f64)) => f64.into(),

            // text
            Instruction::ShortText(ShortTextData(text)) => text.into(),

            // operations
            Instruction::Add => {
                scope_stack.set_active_operation(Instruction::Add);
                ActiveValue::None
            }

            Instruction::CloseAndStore => {
                scope_stack.clear_active_value();
                ActiveValue::None
            }

            Instruction::ScopeStart => {
                scope_stack.create_scope(ScopeType::Default);
                ActiveValue::None
            }

            Instruction::ArrayStart => {
                scope_stack.create_scope(ScopeType::Array);
                is_scope_start = true;
                Array::default().into()
            }

            Instruction::ObjectStart => {
                scope_stack.create_scope(ScopeType::Object);
                is_scope_start = true;
                Object::default().into()
            }

            Instruction::KeyValueShortText(ShortTextData(key)) => {
                scope_stack.set_active_key(key.into());
                ActiveValue::None
            }

            Instruction::KeyValueDynamic => {
                scope_stack.set_active_key(ActiveValue::None);
                ActiveValue::None
            }

            Instruction::ScopeEnd => {
                // pop scope and return value
                println!("Scope end reached, returning value");
                scope_stack.pop()?
            }

            i => {
                return Err(ExecutionError::NotImplemented(
                    format!("Instruction {i}").to_string(),
                ));
            }
        };

        match value {
            ActiveValue::ValueContainer(value_container) => {
                // TODO: try to optimize and initialize variables only when needed, currently leeds to borrow errors
                let active_operation =
                    scope_stack.get_active_operation().cloned();
                let scope_type = scope_stack.get_current_scope_type().clone();
                let active_key = scope_stack.get_active_key();
                let active_value = scope_stack.get_active_value_mut();

                // check if active_key_value_pair exists
                if let Some(active_key) = active_key {
                    println!(
                        "Adding key-value pair: {active_key:?} , {value_container}"
                    );

                    match active_key {
                        // set key for key-value pair (for dynamic keys)
                        ActiveValue::None => {
                            scope_stack.set_active_key(value_container.into());
                        }

                        // set value for key-value pair
                        ActiveValue::ValueContainer(key) => {
                            // insert key value pair into active object
                            match active_value {
                                ActiveValue::ValueContainer(
                                    ValueContainer::Value(Value {
                                        inner: CoreValue::Object(object),
                                        ..
                                    }),
                                ) => {
                                    // make sure key is a string
                                    match key {
                                        ValueContainer::Value(Value {
                                            inner: CoreValue::Text(key_str),
                                            ..
                                        }) => {
                                            object.set(
                                                &key_str.0,
                                                value_container,
                                            );
                                        }
                                        _ => {
                                            return Err(ExecutionError::InvalidProgram(InvalidProgramError::InvalidKeyValuePair));
                                        }
                                    }
                                }
                                // TODO: tuple
                                _ => {
                                    unreachable!("Expected active value object or tuple to collect key value pairs, but got: {}", active_value);
                                }
                            }
                        }
                    }
                } else {
                    match active_value {
                        ActiveValue::None => {
                            // TODO: unary operations

                            // set active value to new value
                            scope_stack
                                .set_active_value_container(value_container);
                        }

                        // value and active value exists
                        ActiveValue::ValueContainer(
                            ref mut active_value_container,
                        ) => {
                            // binary operation
                            if let Some(operation) = active_operation {
                                // apply operation to active value
                                let res = match operation {
                                    Instruction::Add => {
                                        active_value_container as &_
                                            + &value_container
                                    }
                                    _ => {
                                        unreachable!("Instruction {:?} is not a valid operation", operation);
                                    }
                                };
                                if let Ok(val) = res {
                                    // set active value to operation result
                                    scope_stack.set_active_value_container(val);
                                } else {
                                    // handle error
                                    return Err(ExecutionError::ValueError(
                                        res.unwrap_err(),
                                    ));
                                }
                            }
                            // special scope: Array
                            else if !is_scope_start
                                && scope_type == ScopeType::Array
                            {
                                // add value to array scope
                                match active_value_container {
                                    ValueContainer::Value(Value {
                                        inner: CoreValue::Array(array),
                                        ..
                                    }) => {
                                        // append value to array
                                        array.push(value_container);
                                    }
                                    _ => {
                                        unreachable!("Expected active value in array scope to be an array, but got: {}", active_value_container);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            ActiveValue::None => {}
        }
        // let _slot = instruction.slot.unwrap_or_default();
        // let has_primitive_value = instruction.value.is_some();
        // let has_value = instruction.value.is_some();
        // //
        // let error = match code {
        //     // BinaryCode::ADD => binary_operation(code, &mut stack),
        //     // BinaryCode::SUBTRACT => binary_operation(code, &mut stack),
        //     // BinaryCode::MULTIPLY => binary_operation(code, &mut stack),
        //     // BinaryCode::DIVIDE => binary_operation(code, &mut stack),
        //     // BinaryCode::MODULO => binary_operation(code, &mut stack),
        //     // BinaryCode::POWER => binary_operation(code, &mut stack),
        //     // BinaryCode::AND => binary_operation(code, &mut stack),
        //     // BinaryCode::OR => binary_operation(code, &mut stack),
        //
        //     // BinaryCode::CLOSE_AND_STORE => clear_stack(&mut stack),
        //
        //     _ => {
        //         // add value to stack
        //
        //         // if has_value && let Some(value) = instruction.value{
        //         //     stack.push(value)
        //         // } else if has_primitive_value {
        //         //     let primitive_value =
        //         //         instruction.primitive_value.unwrap_or_default();
        //         //     stack.push(Box::new(primitive_value));
        //         // };
        //         None
        //     }
        // };
        //
        // if error.is_some() {
        //     let error_val = error.unwrap();
        //     error!("error: {}", &error_val);
        //     return Err(ExecutionError::Unknown); //TODO
        // }

        // enter new subscope - continue at index?
        // if instruction.subscope_continue {
        //     let sub_result = execute_loop(dxb_body, index, is_end_instruction);
        //
        //     // propagate error from subscope
        //     if sub_result.is_err() {
        //         return Err(sub_result.err().unwrap());
        //     }
        //     // push subscope result to stack
        //     else {
        //         let res = sub_result.ok().unwrap();
        //         info!("sub result: {res:?}");
        //         stack.push(res);
        //     }
        // }
    }

    Ok(match scope_stack.pop_last()? {
        ActiveValue::None => None,
        ActiveValue::ValueContainer(val) => Some(val),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::bytecode::compile_script;
    use crate::global::binary_codes::InstructionCode;

    fn execute_datex_script_debug(
        datex_script: &str,
    ) -> Option<ValueContainer> {
        let dxb = compile_script(datex_script).unwrap();
        let options = ExecutionOptions { verbose: true };
        execute_dxb(dxb, options).unwrap_or_else(|err| {
            panic!("Execution failed: {err}");
        })
    }

    fn execute_datex_script_debug_with_result(
        datex_script: &str,
    ) -> ValueContainer {
        execute_datex_script_debug(datex_script).unwrap()
    }

    fn execute_dxb_debug(
        dxb_body: Vec<u8>,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let options = ExecutionOptions { verbose: true };
        execute_dxb(dxb_body, options)
    }

    #[test]
    fn test_empty_script() {
        assert_eq!(execute_datex_script_debug(""), None);
    }

    #[test]
    fn test_empty_script_semicolon() {
        assert_eq!(execute_datex_script_debug(";;;"), None);
    }

    #[test]
    fn test_single_value() {
        assert_eq!(execute_datex_script_debug_with_result("42"), 42.into());
    }

    #[test]
    fn test_single_value_semicolon() {
        assert_eq!(execute_datex_script_debug("42;"), None)
    }

    #[test]
    fn test_single_value_scope() {
        assert_eq!(execute_datex_script_debug_with_result("(42)"), 42.into());
    }

    #[test]
    fn test_add() {
        let result = execute_datex_script_debug_with_result("1 + 2");
        assert_eq!(result, 3.into());
    }

    #[test]
    fn test_nested_scope() {
        let result = execute_datex_script_debug_with_result("1 + (2 + 3)");
        assert_eq!(result, 6.into());
    }

    #[test]
    fn test_invalid_scope_close() {
        let result = execute_dxb_debug(vec![
            InstructionCode::SCOPE_START.into(),
            InstructionCode::SCOPE_END.into(),
            InstructionCode::SCOPE_END.into(), // Invalid close, no matching start
        ]);
        assert!(matches!(
            result,
            Err(ExecutionError::InvalidProgram(
                InvalidProgramError::InvalidScopeClose
            ))
        ));
    }

    #[test]
    fn test_empty_array() {
        let result = execute_datex_script_debug_with_result("[]");
        assert_eq!(result, Vec::<ValueContainer>::new().into());
    }

    #[test]
    fn test_array_with_values() {
        let result = execute_datex_script_debug_with_result("[1, 2, 3]");
        let expected: Vec<ValueContainer> = vec![1.into(), 2.into(), 3.into()];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn test_array_with_nested_scope() {
        let result = execute_datex_script_debug_with_result("[1, (2 + 3), 4]");
        let expected: Vec<ValueContainer> = vec![1.into(), 5.into(), 4.into()];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn test_boolean() {
        let result = execute_datex_script_debug_with_result("true");
        assert_eq!(result, true.into());

        let result = execute_datex_script_debug_with_result("false");
        assert_eq!(result, false.into());
    }

    #[test]
    fn test_decimal() {
        let result = execute_datex_script_debug_with_result("3.14");
        assert_eq!(result, 3.14.into());

        let result = execute_datex_script_debug_with_result("2.71828");
        assert_eq!(result, 2.71828.into());
    }
}
