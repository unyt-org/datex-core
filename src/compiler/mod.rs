use crate::compiler::error::CompilerError;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use std::fmt::Display;

use crate::compiler::ast_parser::{
    parse, DatexExpression, DatexScriptParser, TupleEntry, VariableType,
};
use crate::compiler::context::Context;
use crate::compiler::metadata::CompileMetadata;
use crate::compiler::scope::Scope;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::core_values::integer::integer::Integer;
use crate::datex_values::core_values::integer::typed_integer::TypedInteger;
use crate::datex_values::core_values::integer::utils::smallest_fitting_signed;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::ValueContainer;
use crate::global::binary_codes::InstructionCode;
use crate::utils::buffers::{
    append_f32, append_f64, append_i128, append_i16, append_i32, append_i64,
    append_i8, append_u128, append_u32, append_u8,
};
use binrw::BinWrite;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;
pub mod ast_parser;
pub mod context;
pub mod error;
mod lexer;
pub mod metadata;
pub mod scope;
use crate::compiler::ast_parser::ParserError;

pub fn compile_block(datex_script: &str) -> Result<Vec<u8>, CompilerError> {
    let (body, _) = compile_script(datex_script, CompileOptions::default())?;

    let routing_header = RoutingHeader {
        version: 2,
        flags: routing_header::Flags::new(),
        block_size_u16: Some(0),
        block_size_u32: None,
        sender: Endpoint::LOCAL,
        receivers: routing_header::Receivers {
            flags: routing_header::ReceiverFlags::new()
                .with_has_endpoints(false)
                .with_has_pointer_id(false)
                .with_has_endpoint_keys(false),
            pointer_id: None,
            endpoints: None,
            endpoints_with_keys: None,
        },
        ..RoutingHeader::default()
    };

    let block_header = BlockHeader::default();
    let encrypted_header = EncryptedHeader::default();

    let block =
        DXBBlock::new(routing_header, block_header, encrypted_header, body);

    let bytes = block
        .to_bytes()
        .map_err(CompilerError::SerializationError)?;
    Ok(bytes)
}

#[derive(Clone, Default)]
pub struct CompileOptions<'a> {
    pub parser: Option<&'a DatexScriptParser<'a>>,
    pub compile_scope: Scope,
}

impl CompileOptions<'_> {
    pub fn new_with_scope(compile_scope: Scope) -> Self {
        CompileOptions {
            parser: None,
            compile_scope,
        }
    }
}

/// Compiles a DATEX script text into a DXB body
pub fn compile_script<'a>(
    datex_script: &'a str,
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, Scope), CompilerError> {
    compile_template(datex_script, &[], options)
}

/// Directly extracts a static value from a DATEX script as a `ValueContainer`.
/// This only works if the script does not contain any dynamic values or operations.
/// All JSON-files can be compiled to static values, but not all DATEX scripts.
pub fn extract_static_value_from_script(
    datex_script: &str,
) -> Result<Option<ValueContainer>, CompilerError> {
    let res = parse(datex_script)?;
    extract_static_value_from_ast(res).map(Some)
}

fn extract_static_value_from_ast(
    ast: DatexExpression,
) -> Result<ValueContainer, CompilerError> {
    if let DatexExpression::Placeholder = ast {
        return Err(CompilerError::NonStaticValue);
    }
    ValueContainer::try_from(ast).map_err(|_| CompilerError::NonStaticValue)
}

/// Compiles a DATEX script template text with inserted values into a DXB body
/// The value containers are passed by reference
pub fn compile_template_with_refs<'a>(
    datex_script: &'a str,
    inserted_values: &[&ValueContainer],
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, Scope), CompilerError> {
    compile_template_or_return_static_value_with_refs(
        datex_script,
        inserted_values,
        false,
        options,
    )
    .map(|result| match result.0 {
        StaticValueOrDXB::StaticValue(_) => unreachable!(),
        StaticValueOrDXB::Dxb(dxb) => (dxb, result.1),
    })
}

/// Compiles a DATEX script template text with inserted values into a DXB body
/// If the script does not contain any dynamic values or operations, the static result value is
/// directly returned instead of the DXB body.
pub fn compile_script_or_return_static_value<'a>(
    datex_script: &'a str,
    options: CompileOptions<'a>,
) -> Result<(StaticValueOrDXB, Scope), CompilerError> {
    compile_template_or_return_static_value_with_refs(
        datex_script,
        &[],
        true,
        options,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticValueOrDXB {
    StaticValue(Option<ValueContainer>),
    Dxb(Vec<u8>),
}

impl From<Vec<u8>> for StaticValueOrDXB {
    fn from(dxb: Vec<u8>) -> Self {
        StaticValueOrDXB::Dxb(dxb)
    }
}

pub fn compile_template_or_return_static_value_with_refs<'a>(
    datex_script: &'a str,
    inserted_values: &[&ValueContainer],
    return_static_value: bool,
    options: CompileOptions<'a>,
) -> Result<(StaticValueOrDXB, Scope), CompilerError> {
    // shortcut if datex_script is "?" - call compile_value directly
    if datex_script == "?" {
        if inserted_values.len() != 1 {
            return Err(CompilerError::InvalidPlaceholderCount);
        }
        let result =
            compile_value(inserted_values[0]).map(StaticValueOrDXB::from)?;
        return Ok((result, options.compile_scope));
    }

    let ast = parse(datex_script)?;

    let buffer = RefCell::new(Vec::with_capacity(256));
    let compilation_context = Context::new(buffer, inserted_values);

    if return_static_value {
        let scope = compile_ast(
            &compilation_context,
            ast.clone(),
            options.compile_scope,
        )?;

        if !*compilation_context.has_non_static_value.borrow() {
            if let Ok(value) = ValueContainer::try_from(ast) {
                return Ok((
                    StaticValueOrDXB::StaticValue(Some(value.clone())),
                    scope,
                ));
            }
            Ok((StaticValueOrDXB::StaticValue(None), scope))
        } else {
            // return DXB body
            Ok((
                StaticValueOrDXB::Dxb(compilation_context.buffer.take()),
                scope,
            ))
        }
    } else {
        let scope =
            compile_ast(&compilation_context, ast, options.compile_scope)?;
        // return DXB body
        Ok((
            StaticValueOrDXB::Dxb(compilation_context.buffer.take()),
            scope,
        ))
    }
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template<'a>(
    datex_script: &'a str,
    inserted_values: &[ValueContainer],
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, Scope), CompilerError> {
    compile_template_with_refs(
        datex_script,
        &inserted_values.iter().collect::<Vec<_>>(),
        options,
    )
}

pub fn compile_value(value: &ValueContainer) -> Result<Vec<u8>, CompilerError> {
    let buffer = RefCell::new(Vec::with_capacity(256));
    let compilation_scope = Context::new(buffer, &[]);

    compilation_scope.insert_value_container(value);

    Ok(compilation_scope.buffer.take())
}

/// Macro for compiling a DATEX script template text with inserted values into a DXB body,
/// behaves like the format! macro.
/// Example:
/// ```
/// use datex_core::compile;
/// compile!("4 + ?", 42);
/// compile!("? + ?", 1, 2);
#[macro_export]
macro_rules! compile {
    ($fmt:literal $(, $arg:expr )* $(,)?) => {
        {
            let script: &str = $fmt.into();
            let values: &[$crate::datex_values::value_container::ValueContainer] = &[$($arg.into()),*];

            $crate::compiler::compile_template(&script, values, $crate::compiler::CompileOptions::default())
        }
    }
}

fn compile_ast(
    compilation_scope: &Context,
    ast: DatexExpression,
    scope: Scope,
) -> Result<Scope, CompilerError> {
    let scope = compile_expression(
        compilation_scope,
        ast,
        CompileMetadata::outer(),
        scope,
    )?;
    Ok(scope)
}

fn compile_expression(
    compilation_scope: &Context,
    ast: DatexExpression,
    meta: CompileMetadata,
    mut scope: Scope,
) -> Result<Scope, CompilerError> {
    match ast {
        DatexExpression::Integer(int) => {
            compilation_scope.insert_int(int.0.as_i64().unwrap());
        }
        DatexExpression::Decimal(decimal) => match &decimal {
            Decimal::Finite(big_decimal) if big_decimal.is_integer() => {
                if let Some(int) = big_decimal.to_i16() {
                    compilation_scope.insert_float_as_i16(int);
                } else if let Some(int) = big_decimal.to_i32() {
                    compilation_scope.insert_float_as_i32(int);
                } else {
                    compilation_scope.insert_decimal(&decimal);
                }
            }
            _ => {
                compilation_scope.insert_decimal(&decimal);
            }
        },
        DatexExpression::Text(text) => {
            compilation_scope.insert_text(&text);
        }
        DatexExpression::Boolean(boolean) => {
            compilation_scope.insert_boolean(boolean);
        }
        DatexExpression::Null => {
            compilation_scope.append_binary_code(InstructionCode::NULL);
        }
        DatexExpression::Array(array) => {
            compilation_scope.append_binary_code(InstructionCode::ARRAY_START);
            for item in array {
                scope = compile_expression(
                    compilation_scope,
                    item,
                    CompileMetadata::default(),
                    scope,
                )?;
            }
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
        }
        DatexExpression::Tuple(tuple) => {
            compilation_scope.append_binary_code(InstructionCode::TUPLE_START);
            for entry in tuple {
                match entry {
                    TupleEntry::KeyValue(key, value) => {
                        scope = compile_key_value_entry(
                            compilation_scope,
                            key,
                            value,
                            scope,
                        )?;
                    }
                    TupleEntry::Value(value) => {
                        scope = compile_expression(
                            compilation_scope,
                            value,
                            CompileMetadata::default(),
                            scope,
                        )?;
                    }
                }
            }
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
        }
        DatexExpression::Object(object) => {
            compilation_scope.append_binary_code(InstructionCode::OBJECT_START);
            for (key, value) in object {
                // compile key and value
                scope = compile_key_value_entry(
                    compilation_scope,
                    key,
                    value,
                    scope,
                )?;
            }
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
        }

        DatexExpression::Placeholder => {
            compilation_scope.insert_value_container(
                compilation_scope
                    .inserted_values
                    .borrow()
                    .get(compilation_scope.inserted_value_index.get())
                    .unwrap(),
            );
            compilation_scope.inserted_value_index.update(|x| x + 1);
        }

        // statements
        DatexExpression::Statements(mut statements) => {
            compilation_scope.mark_has_non_static_value();
            // if single statement and not terminated, just compile the expression
            if statements.len() == 1 && !statements[0].is_terminated {
                scope = compile_expression(
                    compilation_scope,
                    statements.remove(0).expression,
                    CompileMetadata::default(),
                    scope,
                )?;
            } else {
                // if not outer context, new scope
                let mut child_scope = if !meta.is_outer_context() {
                    compilation_scope
                        .append_binary_code(InstructionCode::SCOPE_START);
                    scope.push()
                } else {
                    scope
                };
                for statement in statements {
                    child_scope = compile_expression(
                        compilation_scope,
                        statement.expression,
                        CompileMetadata::default(),
                        child_scope,
                    )?;
                    // if statement is terminated, append close and store
                    if statement.is_terminated {
                        compilation_scope.append_binary_code(
                            InstructionCode::CLOSE_AND_STORE,
                        );
                    }
                }
                if !meta.is_outer_context() {
                    let scope_data = child_scope
                        .pop()
                        .ok_or(CompilerError::ScopePopError)?;
                    scope = scope_data.0; // set parent scope
                                          // drop all slot addresses that were allocated in this scope
                    for slot_address in scope_data.1 {
                        compilation_scope
                            .append_binary_code(InstructionCode::DROP_SLOT);
                        compilation_scope.append_u32(slot_address);
                    }
                    compilation_scope
                        .append_binary_code(InstructionCode::SCOPE_END);
                } else {
                    scope = child_scope;
                }
            }
        }

        // operations (add, subtract, multiply, divide, etc.)
        DatexExpression::BinaryOperation(operator, a, b) => {
            compilation_scope.mark_has_non_static_value();
            // append binary code for operation if not already current binary operator
            compilation_scope
                .append_binary_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_scope,
                *a,
                CompileMetadata::default(),
                scope,
            )?;
            scope = compile_expression(
                compilation_scope,
                *b,
                CompileMetadata::default(),
                scope,
            )?;
        }

        // apply
        DatexExpression::ApplyChain(val, operands) => {
            compilation_scope.mark_has_non_static_value();
            // TODO
        }

        // variables
        // declaration
        DatexExpression::VariableDeclaration(var_type, name, expression) => {
            compilation_scope.mark_has_non_static_value();
            // allocate new slot for variable
            let address = scope.get_next_variable_slot();
            compilation_scope
                .append_binary_code(InstructionCode::ALLOCATE_SLOT);
            compilation_scope.append_u32(address);
            // create reference
            if var_type == VariableType::Reference {
                compilation_scope
                    .append_binary_code(InstructionCode::CREATE_REF);
            }
            // compile expression
            scope = compile_expression(
                compilation_scope,
                *expression,
                CompileMetadata::default(),
                scope,
            )?;

            // register new variable
            scope.register_variable_slot(address, var_type, name);
        }

        // assignment
        DatexExpression::VariableAssignment(name, expression) => {
            compilation_scope.mark_has_non_static_value();
            // get variable slot address
            let (var_slot, var_type) =
                scope.resolve_variable_slot(&name).ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;

            // append binary code to load variable
            compilation_scope.append_binary_code(InstructionCode::UPDATE_SLOT);
            compilation_scope.append_u32(var_slot);
            // compile expression
            scope = compile_expression(
                compilation_scope,
                *expression,
                CompileMetadata::default(),
                scope,
            )?;
            // close assignment scope
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
        }

        // variable access
        DatexExpression::Variable(name) => {
            compilation_scope.mark_has_non_static_value();
            // get variable slot address
            let (var_slot, var_type) =
                scope.resolve_variable_slot(&name).ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;
            // append binary code to load variable
            compilation_scope.append_binary_code(InstructionCode::GET_SLOT);
            compilation_scope.append_u32(var_slot);
        }

        _ => return Err(CompilerError::UnexpectedTerm(ast)),
    }

    Ok(scope)
}

fn compile_key_value_entry(
    compilation_scope: &Context,
    key: DatexExpression,
    value: DatexExpression,
    mut scope: Scope,
) -> Result<Scope, CompilerError> {
    match key {
        // text -> insert key string
        DatexExpression::Text(text) => {
            compilation_scope.insert_key_string(&text);
        }
        // other -> insert key as dynamic
        _ => {
            compilation_scope
                .append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
            scope = compile_expression(
                compilation_scope,
                key,
                CompileMetadata::default(),
                scope,
            )?;
        }
    };
    // insert value
    scope = compile_expression(
        compilation_scope,
        value,
        CompileMetadata::default(),
        scope,
    )?;
    Ok(scope)
}

#[cfg(test)]
pub mod tests {
    use super::{
        compile_ast, compile_script, compile_script_or_return_static_value,
        compile_template, CompileOptions, Context, Scope, StaticValueOrDXB,
    };
    use std::cell::RefCell;
    use std::io::Read;
    use std::vec;

    use crate::{global::binary_codes::InstructionCode, logger::init_logger};
    use log::*;

    use crate::compiler::ast_parser::parse;
    use crate::datex_values::core_values::integer::integer::Integer;

    fn compile_and_log(datex_script: &str) -> Vec<u8> {
        init_logger();
        let (result, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        info!(
            "{:?}",
            result
                .iter()
                .map(|x| InstructionCode::try_from(*x).map(|x| x.to_string()))
                .map(|x| x.unwrap_or_else(|_| "Unknown".to_string()))
                .collect::<Vec<_>>()
        );
        result
    }

    fn get_compilation_scope(script: &str) -> Context {
        let ast = parse(script);
        let ast = ast.unwrap();
        let buffer = RefCell::new(Vec::with_capacity(256));
        let compilation_scope = Context::new(buffer, &[]);
        compile_ast(&compilation_scope, ast, Scope::default()).unwrap();
        compilation_scope
    }

    #[test]
    fn test_simple_multiplication() {
        init_logger();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} * {rhs}"); // 1 * 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn test_simple_multiplication_close() {
        init_logger();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} * {rhs};"); // 1 * 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
                InstructionCode::CLOSE_AND_STORE.into()
            ]
        );
    }

    #[test]
    fn test_is_operator() {
        init_logger();

        // TODO: compare refs
        let datex_script = "1 is 2".to_string();
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::IS.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2
            ]
        );

        let datex_script = "ref a = 42; ref b = 69; a is b".to_string(); // a is b
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                // val a = 42;
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::CLOSE_AND_STORE.into(),
                // val b = 69;
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::CREATE_REF.into(),
                InstructionCode::INT_8.into(),
                69,
                InstructionCode::CLOSE_AND_STORE.into(),
                // a is b
                InstructionCode::IS.into(),
                InstructionCode::GET_SLOT.into(),
                0,
                0,
                0,
                0, // slot address for a
                InstructionCode::GET_SLOT.into(),
                1,
                0,
                0,
                0, // slot address for b
            ]
        );
    }

    #[test]
    fn test_equality_operator() {
        init_logger();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} == {rhs}"); // 1 == 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::STRUCTURAL_EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );

        let datex_script = format!("{lhs} === {rhs}"); // 1 === 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );

        let datex_script = format!("{lhs} != {rhs}"); // 1 != 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::NOT_STRUCTURAL_EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );
        let datex_script = format!("{lhs} !== {rhs}"); // 1 !== 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::NOT_EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn test_simple_addition() {
        init_logger();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} + {rhs}"); // 1 + 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs
            ]
        );

        let datex_script = format!("{lhs} + {rhs};"); // 1 + 2;
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
                InstructionCode::CLOSE_AND_STORE.into()
            ]
        );
    }

    #[test]
    fn test_multi_addition() {
        init_logger();

        let op1: u8 = 1;
        let op2: u8 = 2;
        let op3: u8 = 3;
        let op4: u8 = 4;

        let datex_script = format!("{op1} + {op2} + {op3} + {op4}"); // 1 + 2 + 3 + 4
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::ADD.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                op1,
                InstructionCode::INT_8.into(),
                op2,
                InstructionCode::INT_8.into(),
                op3,
                InstructionCode::INT_8.into(),
                op4,
            ]
        );
    }

    #[test]
    fn test_mixed_calculation() {
        init_logger();

        let op1: u8 = 1;
        let op2: u8 = 2;
        let op3: u8 = 3;
        let op4: u8 = 4;

        let datex_script = format!("{op1} * {op2} + {op3} * {op4}"); // 1 + 2 + 3 + 4
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                op1,
                InstructionCode::INT_8.into(),
                op2,
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                op3,
                InstructionCode::INT_8.into(),
                op4,
            ]
        );
    }

    #[test]
    fn test_complex_addition() {
        init_logger();

        let a: u8 = 1;
        let b: u8 = 2;
        let c: u8 = 3;
        let datex_script = format!("{a} + ({b} + {c})"); // 1 + (2 + 3)
        let result = compile_and_log(&datex_script);

        // note: scope is automatically collapsed by the parser since this is all the same operation
        // TODO: we might need to change this to support nested additions, or maybe not if we only allow additions
        // of values of the same type?...
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                a,
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                b,
                InstructionCode::INT_8.into(),
                c,
            ]
        );
    }

    #[test]
    fn test_complex_addition_and_subtraction() {
        init_logger();

        let a: u8 = 1;
        let b: u8 = 2;
        let c: u8 = 3;
        let datex_script = format!("{a} + ({b} - {c})"); // 1 + (2 - 3)
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                a,
                InstructionCode::SUBTRACT.into(),
                InstructionCode::INT_8.into(),
                b,
                InstructionCode::INT_8.into(),
                c,
            ]
        );
    }

    // Test for integer/u8
    #[test]
    fn test_integer_u8() {
        init_logger();
        let val: u8 = 42;
        let datex_script = format!("{val}"); // 42
        let result = compile_and_log(&datex_script);
        assert_eq!(result, vec![InstructionCode::INT_8.into(), val,]);
    }

    // Test for decimal
    #[test]
    fn test_decimal() {
        init_logger();
        let datex_script = "42.0";
        let result = compile_and_log(datex_script);
        let bytes = 42_i16.to_le_bytes();

        let mut expected: Vec<u8> =
            vec![InstructionCode::DECIMAL_AS_INT_16.into()];
        expected.extend(bytes);

        assert_eq!(result, expected);
    }

    /// Test for test that is less than 256 characters
    #[test]
    fn test_short_text() {
        init_logger();
        let val = "unyt";
        let datex_script = format!("\"{val}\""); // "42"
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> =
            vec![InstructionCode::SHORT_TEXT.into(), val.len() as u8];
        expected.extend(val.bytes());
        assert_eq!(result, expected);
    }

    // Test empty array
    #[test]
    fn test_empty_array() {
        init_logger();
        let datex_script = "[]";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::ARRAY_START.into(),
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // Test array with single element
    #[test]
    fn test_single_element_array() {
        init_logger();
        let datex_script = "[42]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test array with multiple elements
    #[test]
    fn test_multi_element_array() {
        init_logger();
        let datex_script = "[1, 2, 3]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test nested arrays
    #[test]
    fn test_nested_arrays() {
        init_logger();
        let datex_script = "[1, [2, 3], 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test array with expressions inside
    #[test]
    fn test_array_with_expressions() {
        init_logger();
        let datex_script = "[1 + 2, 3 * 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test array with mixed expressions
    #[test]
    fn test_array_with_mixed_expressions() {
        init_logger();
        let datex_script = "[1, 2, 3 + 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test tuple
    #[test]
    fn test_tuple() {
        init_logger();
        let datex_script = "(1, 2, 3)";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::TUPLE_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Nested tuple
    #[test]
    fn test_nested_tuple() {
        init_logger();
        let datex_script = "(1, (2, 3), 4)";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::TUPLE_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::TUPLE_START.into(),
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Tuple without parentheses
    #[test]
    fn test_tuple_without_parentheses() {
        init_logger();
        let datex_script = "1, 2, 3";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::TUPLE_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // key-value pair
    #[test]
    fn test_key_value_tuple() {
        init_logger();
        let datex_script = "key: 42";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // key-value pair with string key
    #[test]
    fn test_key_value_string() {
        init_logger();
        let datex_script = "\"key\": 42";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // key-value pair with integer key
    #[test]
    fn test_key_value_integer() {
        init_logger();
        let datex_script = "10: 42";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::INT_8.into(),
            10,
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // key-value pair with long text key (>255 bytes)
    #[test]
    fn test_key_value_long_text() {
        init_logger();
        let long_key = "a".repeat(300);
        let datex_script = format!("\"{long_key}\": 42");
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::TEXT.into(),
        ];
        expected.extend((long_key.len() as u32).to_le_bytes());
        expected.extend(long_key.as_bytes());
        expected.extend(vec![
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ]);
        assert_eq!(result, expected);
    }

    // dynamic key-value pair
    #[test]
    fn test_dynamic_key_value() {
        init_logger();
        let datex_script = "(1 + 2): 42";
        let result = compile_and_log(datex_script);
        let expected = [
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // multiple key-value pairs
    #[test]
    fn test_multiple_key_value_pairs() {
        init_logger();
        let datex_script = "key: 42, 4: 43, (1 + 2): 44";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::INT_8.into(),
            4,
            InstructionCode::INT_8.into(),
            43,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // key value pair with parentheses
    #[test]
    fn test_key_value_with_parentheses() {
        init_logger();
        let datex_script = "(key: 42)";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // empty object
    #[test]
    fn test_empty_object() {
        init_logger();
        let datex_script = "{}";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::OBJECT_START.into(),
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // object with single key-value pair
    #[test]
    fn test_single_key_value_object() {
        init_logger();
        let datex_script = "{key: 42}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::OBJECT_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // object with multiple key-value pairs
    #[test]
    fn test_multi_key_value_object() {
        init_logger();
        let datex_script = "{key1: 42, \"key2\": 43, 'key3': 44}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::OBJECT_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key1"
            b'k',
            b'e',
            b'y',
            b'1',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key2"
            b'k',
            b'e',
            b'y',
            b'2',
            InstructionCode::INT_8.into(),
            43,
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key3"
            b'k',
            b'e',
            b'y',
            b'3',
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_allocate_slot() {
        init_logger();
        let script = "val a = 42";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn test_allocate_slot_with_value() {
        init_logger();
        let script = "val a = 42; a + 1";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ADD.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn test_allocate_scoped_slots() {
        init_logger();
        let script = "val a = 42; (val a = 43; a); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::DROP_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn test_allocate_scoped_slots_with_parent_variables() {
        init_logger();
        let script = "val a = 42; val b = 41; (val a = 43; a; b); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                41,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::DROP_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn test_allocate_ref() {
        init_logger();
        let script = "ref a = 42";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF.into(),
                InstructionCode::INT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn test_read_ref() {
        init_logger();
        let script = "ref a = 42; a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn test_compile() {
        init_logger();
        let result = compile_template(
            "? + ?",
            &[1.into(), 2.into()],
            CompileOptions::default(),
        );
        assert_eq!(
            result.unwrap().0,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2
            ]
        );
    }

    #[test]
    fn test_compile_macro() {
        init_logger();
        let a = 1;
        let result = compile!("?", a);
        assert_eq!(result.unwrap().0, vec![InstructionCode::INT_8.into(), 1,]);
    }

    #[test]
    fn test_compile_macro_multi() {
        init_logger();
        let result = compile!("? + ?", 1, 2);
        assert_eq!(
            result.unwrap().0,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2
            ]
        );
    }

    fn get_json_test_string(file_path: &str) -> String {
        // read json from test file
        let file_path = format!("benches/json/{file_path}");
        let file_path = std::path::Path::new(&file_path);
        let file =
            std::fs::File::open(file_path).expect("Failed to open test.json");
        let mut reader = std::io::BufReader::new(file);
        let mut json_string = String::new();
        reader
            .read_to_string(&mut json_string)
            .expect("Failed to read test.json");
        json_string
    }

    #[test]
    fn test_json_to_dxb_large_file() {
        let json = get_json_test_string("test2.json");
        println!("JSON file read");
        let (dxb, _) = compile_script(&json, CompileOptions::default())
            .expect("Failed to parse JSON string");
        println!("DXB: {:?}", dxb.len());
    }

    #[test]
    fn test_static_value_detection() {
        init_logger();

        // non-static
        let script = "1 + 2";
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = "a b";
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = "1;2";
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = r#"{("x" + "y"): 1}"#;
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        // static
        let script = "1";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "[]";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "{}";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "[1,2,3]";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "{a: 2}";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());
    }

    #[test]
    fn test_compile_auto_static_value_detection() {
        let script = "1";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            res,
            StaticValueOrDXB::StaticValue(Some(Integer::from(1).into()))
        );

        let script = "1 + 2";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            res,
            StaticValueOrDXB::Dxb(vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
            ])
        );
    }
}
