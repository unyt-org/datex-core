use std::fmt::Display;
use std::ops::Add;
use log::info;
use crate::datex_values::core_values::array::DatexArray;
use crate::datex_values::value::{DatexValueInner, Value};
use crate::parser::body;
use crate::datex_values::value_container::{ValueContainer, ValueError};
use crate::global::protocol_structures::instructions::{Instruction, Int8Data, ShortTextData};
use crate::parser::body::ParserError;
use super::stack::{ScopeStack, ScopeType};

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    dxb_body: Vec<u8>,
    options: ExecutionOptions,
    index: usize,
    scope_stack: ScopeStack
}

pub fn execute_dxb(dxb_body: Vec<u8>, options: ExecutionOptions) -> Result<Option<ValueContainer>, ExecutionError> {
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
}

impl Display for InvalidProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidProgramError::InvalidScopeClose => write!(f, "Invalid scope close"),
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    ParserError(ParserError),
    ValueError(ValueError),
    InvalidProgram(InvalidProgramError),
    Unknown,
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
            ExecutionError::ParserError(err) => write!(f, "Parser error: {err}"),
            ExecutionError::Unknown => write!(f, "Unknown execution error"),
            ExecutionError::ValueError(err) => write!(f, "Value error: {err}"),
            ExecutionError::InvalidProgram(err) => {
                write!(f, "Invalid program error: {err}")
            }
        }
    }
}


fn execute_loop(
    context: ExecutionContext,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let dxb_body = context.dxb_body;
    let mut scope_stack = context.scope_stack;

    let instruction_iterator =
        body::iterate_instructions(&dxb_body);

    for instruction in instruction_iterator {
        let instruction = instruction?;
        if context.options.verbose {
            println!("[Exec]: {:?}", &instruction);
        }

        let value: Option<ValueContainer> = match instruction {

            Instruction::Int8(Int8Data(i8)) => {
                Some(i8.into())
            }
            
            Instruction::ShortText(ShortTextData(text)) => {
                Some(text.into())
            }
            
            // operations
            Instruction::Add => {
                scope_stack.set_active_operation(Instruction::Add);
                None
            }

            Instruction::CloseAndStore => {
                scope_stack.clear_active_value();
                None
            }
            
            Instruction::ScopeStart => {
                scope_stack.create_scope(ScopeType::Default);
                None
            }
            
            Instruction::ArrayStart => {
                info!("Array start reached, creating new scope for array");
                scope_stack.create_scope(ScopeType::Array);
                scope_stack.set_active_value(Value::from(DatexArray::default()).into());
                None
            }

            Instruction::ScopeEnd => {
                // pop scope and return value
                info!("Scope end reached, returning value");
                scope_stack.pop()?
            }

            i => {
                info!("Instruction not implemented: {i:?}");
                None
            }
        };

        // has processable value
        if let Some(val) = value {

            // unary operations....

            // operation
            if let Some(operation) = scope_stack.get_active_operation() {
                let active_value = scope_stack.get_active_value();
                if active_value.is_none() {
                    // set active value to operation result
                    scope_stack.set_active_value(val);
                } else if let Some(active_value) = active_value  {
                    // apply operation to active value
                    let res = active_value + &val;
                    if let Ok(val) = res {
                        // set active value to operation result
                        scope_stack.set_active_value(val);
                    } else {
                        // handle error
                        return Err(ExecutionError::ValueError(res.unwrap_err()));
                    }
                }
            }
            // set active value in current scope
            else {
                scope_stack.set_active_value(val);
            }
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

    Ok(scope_stack.pop_last()?)
}

#[cfg(test)]
mod tests {
    use crate::compiler::bytecode::compile_script;
    use crate::global::binary_codes::InstructionCode;
    use super::*;

    fn execute_datex_script_debug(datex_script: &str) -> Option<ValueContainer> {
        let dxb = compile_script(datex_script).unwrap();
        let options = ExecutionOptions { verbose: true };
        execute_dxb(dxb, options).unwrap_or_else(|err| {
            panic!("Execution failed: {err}");
        })
    }
    
    fn execute_dxb_debug(dxb_body: Vec<u8>) -> Result<Option<ValueContainer>, ExecutionError> {
        let options = ExecutionOptions { verbose: true };
        execute_dxb(dxb_body, options)
    }

    #[test]
    fn test_empty_script() {
        assert_eq!(
            execute_datex_script_debug(""),
            None
        );
    }

    #[test]
    fn test_empty_script_semicolon() {
        assert_eq!(
            execute_datex_script_debug(";;;"),
            None
        );
    }

    #[test]
    fn test_single_value() {
        assert_eq!(
            execute_datex_script_debug("42"),
            ValueContainer::from(42).into()
        );
    }

    #[test]
    fn test_single_value_semicolon() {
        assert_eq!(
            execute_datex_script_debug("42;"),
            None
        )
    }

    #[test]
    fn test_single_value_scope() {
        assert_eq!(
            execute_datex_script_debug("(42)"),
            ValueContainer::from(42).into()
        );
    }

    #[test]
    fn test_add() {
        let result = execute_datex_script_debug("1 + 2");
        assert_eq!(result, ValueContainer::from(3).into());
    }
    
    #[test]
    fn test_nested_scope() {
        let result = execute_datex_script_debug("1 + (2 + 3)");
        assert_eq!(result, ValueContainer::from(6).into());
    }
    
    #[test]
    fn test_invalid_scope_close() {
        let result = execute_dxb_debug(
            vec![
                InstructionCode::SCOPE_START.into(),
                InstructionCode::SCOPE_END.into(),
                InstructionCode::SCOPE_END.into(), // Invalid close, no matching start
            ]
        );
        assert!(matches!(result, Err(ExecutionError::InvalidProgram(InvalidProgramError::InvalidScopeClose))));
    }
}
