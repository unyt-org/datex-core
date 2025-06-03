use std::fmt::Display;
use std::ops::Add;
use log::info;
use crate::parser::body;
use crate::datex_values::value_container::{ValueContainer, ValueError};
use crate::global::protocol_structures::instructions::{Instruction, Int8Data};
use crate::parser::body::ParserError;
use super::stack::ScopeStack;

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

pub fn execute_dxb(dxb_body: Vec<u8>, options: ExecutionOptions) -> Result<ValueContainer, ExecutionError> {
    let context = ExecutionContext {
        dxb_body,
        options,
        ..ExecutionContext::default()
    };
    execute_loop(context)
}


#[derive(Debug)]
pub enum ExecutionError {
    ParserError(ParserError),
    Unknown,
    ValueError(ValueError),
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

impl Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::ParserError(err) => write!(f, "Parser error: {err}"),
            ExecutionError::Unknown => write!(f, "Unknown execution error"),
            ExecutionError::ValueError(err) => write!(f, "Value error: {err}"),
        }
    }
}


fn execute_loop(
    context: ExecutionContext,
) -> Result<ValueContainer, ExecutionError> {
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
                scope_stack.create_scope();
                None
            }

            Instruction::ScopeEnd => {
                // pop scope and return value
                info!("Scope end reached, returning value");
                Some(scope_stack.pop())
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
                if active_value == &ValueContainer::Void {
                    // set active value to operation result
                    scope_stack.set_active_value(val);
                } else {
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

    // clear_stack(&mut stack);

    Ok(scope_stack.pop())
}

#[cfg(test)]
mod tests {
    use crate::compiler::bytecode::compile_script;
    use super::*;

    fn execute_dxb_debug(datex_script: &str) -> ValueContainer {
        let dxb = compile_script(&datex_script).unwrap();
        let options = ExecutionOptions { verbose: true };
        execute_dxb(dxb, options).unwrap_or_else(|err| {
            panic!("Execution failed: {err}");
        })
    }

    #[test]
    fn test_empty_script() {
        assert_eq!(
            execute_dxb_debug(""),
            ValueContainer::Void
        );
    }

    #[test]
    fn test_empty_script_semicolon() {
        assert_eq!(
            execute_dxb_debug(";;;"),
            ValueContainer::Void
        );
    }

    #[test]
    fn test_single_value() {
        assert_eq!(
            execute_dxb_debug("42"),
            ValueContainer::from(42)
        );
    }

    #[test]
    fn test_single_value_semicolon() {
        assert_eq!(
            execute_dxb_debug("42;"),
            ValueContainer::Void
        )
    }

    #[test]
    fn test_single_value_scope() {
        assert_eq!(
            execute_dxb_debug("(42)"),
            ValueContainer::from(42)
        );
    }

    #[test]
    fn test_add() {
        let result = execute_dxb_debug("1 + 2");
        assert_eq!(result, ValueContainer::from(3));
    }
    
    #[test]
    fn test_nested_scope() {
        let result = execute_dxb_debug("1 + (2 + 3)");
        assert_eq!(result, ValueContainer::from(6));
    }
}



//
// // reset stack
// // clear from end and set final value as first stack value of new stack
// fn clear_stack(stack: &mut Stack) -> Option<Error> {
//     if stack.size() == 0 {
//         return None;
//     }; // nothing to clear
//
//     let mut current: Box<dyn Value> = stack.pop_or_void(); // get last stack value
//
//     while stack.size() != 0 {
//         let next = stack.pop_or_void();
//
//         // type cast
//         if next.is::<Type>() {
//             debug!("cast {next} {current}");
//             let dx_type = next.downcast::<Type>();
//             if dx_type.is_ok() {
//                 let res = current.cast(*dx_type.ok().unwrap());
//                 if res.is_ok() {
//                     current = res.ok().unwrap();
//                 } else {
//                     return res.err();
//                 }
//             } else {
//                 return Some(Error {
//                     message: "rust downcasting error".to_string(),
//                 });
//             }
//         }
//         // other apply
//         else {
//             debug!("apply {next} {current}");
//         }
//     }
//
//     stack.push(current);
//
//     None
// }
//
// // operator handlers
//
// fn binary_operation(code: BinaryCode, stack: &mut Stack) -> Option<Error> {
//     stack.print();
//
//     // pop 2 operands from stack
//     let _s1 = stack.pop();
//     if _s1.is_err() {
//         return _s1.err();
//     }
//     let s1 = _s1.ok().unwrap();
//
//     let _s2 = stack.pop();
//     if _s2.is_err() {
//         return _s2.err();
//     }
//     let s2 = _s2.ok().unwrap();
//
//     // binary operation
//     match s2.binary_operation(code, s1) {
//         Ok(result) => {
//             info!("binary op result: {result}");
//             stack.push(result);
//             None
//         }
//         Err(err) => Some(err),
//     }
// }
