use log::info;
use crate::compiler::parser::DatexParser;
use crate::compiler::parser::Rule;
use crate::global::binary_codes::BinaryCode;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use crate::utils::buffers::append_f64;
use crate::utils::buffers::append_i16;
use crate::utils::buffers::append_i32;
use crate::utils::buffers::append_i64;
use crate::utils::buffers::append_i8;
use strum::Display;

use crate::utils::buffers::append_u32;
use crate::utils::buffers::append_u8;

pub mod parser;
mod operations;

use crate::datex_values::core_values::endpoint::Endpoint;
use pest::error::Error;
use pest::iterators::Pair;
use pest::iterators::Pairs;
use pest::Parser;
use regex::Regex;
use crate::compiler::operations::parse_operator;

#[derive(Debug, Display)]
pub enum CompilationError {
    InvalidRule(String),
    SerializationError(binrw::Error),
}

pub fn compile(datex_script: &str) -> Result<Vec<u8>, CompilationError> {
    let body = compile_body(datex_script)
        .map_err(|e| CompilationError::InvalidRule(e.to_string()))?;

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
        .map_err(CompilationError::SerializationError)?;
    Ok(bytes)
}

struct CompilationScope<'a> {
    index: usize,
    buffer: &'a mut Vec<u8>,
}

impl<'a> CompilationScope<'a> {
    const MAX_INT_32: i64 = 2_147_483_647;
    const MIN_INT_32: i64 = -2_147_483_648;

    const MAX_INT_8: i64 = 127;
    const MIN_INT_8: i64 = -128;

    const MAX_INT_16: i64 = 32_767;
    const MIN_INT_16: i64 = -32_768;

    const MAX_UINT_16: i64 = 65_535;

    const INT_8_BYTES: u8 = 1;
    const INT_16_BYTES: u8 = 2;
    const INT_32_BYTES: u8 = 4;
    const INT_64_BYTES: u8 = 8;
    const UINT_8_BYTES: u8 = 1;
    const UINT_16_BYTES: u8 = 2;
    const UINT_32_BYTES: u8 = 4;
    const UINT_64_BYTES: u8 = 8;
    const FLOAT_64_BYTES: u8 = 8;

    // value insert functions
    fn insert_boolean(&mut self, boolean: bool) {
        if boolean {
            self.append_binary_code(BinaryCode::TRUE);
        } else {
            self.append_binary_code(BinaryCode::FALSE);
        }
    }

    fn insert_string(&mut self, string: &str) {
        let unescaped_string = self.unescape_string(string);

        let bytes = unescaped_string.as_bytes();
        let len = bytes.len();

        if len < 256 {
            self.append_binary_code(BinaryCode::SHORT_TEXT);
            self.append_u8(len as u8);
        } else {
            self.append_binary_code(BinaryCode::TEXT);
            self.append_u32(len as u32);
        }

        self.append_buffer(bytes);
    }

    fn unescape_string(&mut self, string: &str) -> String {
        let re = Regex::new(r"\\(.)").unwrap();

        // TODO: escape, unicode, hex, octal?
        re.replace_all(
            &string
                .replace("\\b", "\u{0008}")
                .replace("\\f", "\u{000c}")
                .replace("\\r", "\r")
                .replace("\\t", "\t")
                .replace("\\v", "\u{000b}")
                .replace("\\n", "\n"),
            "$1",
        )
        .into_owned()
    }

    fn insert_float64(&mut self, float64: f64) {
        self.append_binary_code(BinaryCode::FLOAT_64);
        self.append_f64(float64);
    }

    fn insert_int(&mut self, int: i64) {
        if (CompilationScope::MIN_INT_8..=CompilationScope::MAX_INT_8)
            .contains(&int)
        {
            self.insert_int8(int as i8)
        } else if (CompilationScope::MIN_INT_16..=CompilationScope::MAX_INT_16)
            .contains(&int)
        {
            self.insert_int16(int as i16)
        } else if (CompilationScope::MIN_INT_32..=CompilationScope::MAX_INT_32)
            .contains(&int)
        {
            self.insert_int32(int as i32)
        } else {
            self.insert_int64(int)
        }
    }

    fn insert_int8(&mut self, int8: i8) {
        self.append_binary_code(BinaryCode::INT_8);
        self.append_i8(int8);
    }
    fn insert_int16(&mut self, int16: i16) {
        self.append_binary_code(BinaryCode::INT_16);
        self.append_i16(int16);
    }
    fn insert_int32(&mut self, int32: i32) {
        self.append_binary_code(BinaryCode::INT_32);
        self.append_i32(int32);
    }
    fn insert_int64(&mut self, int64: i64) {
        self.append_binary_code(BinaryCode::INT_64);
        self.append_i64(int64);
    }

    // buffer functions
    fn append_u8(&mut self, u8: u8) {
        append_u8(self.buffer, u8);
        self.index += CompilationScope::UINT_8_BYTES as usize;
    }
    fn append_u32(&mut self, u32: u32) {
        append_u32(self.buffer, u32);
        self.index += CompilationScope::UINT_32_BYTES as usize;
    }
    fn append_i8(&mut self, i8: i8) {
        append_i8(self.buffer, i8);
        self.index += CompilationScope::INT_8_BYTES as usize;
    }
    fn append_i16(&mut self, i16: i16) {
        append_i16(self.buffer, i16);
        self.index += CompilationScope::INT_16_BYTES as usize;
    }
    fn append_i32(&mut self, i32: i32) {
        append_i32(self.buffer, i32);
        self.index += CompilationScope::INT_32_BYTES as usize;
    }
    fn append_i64(&mut self, i64: i64) {
        append_i64(self.buffer, i64);
        self.index += CompilationScope::INT_64_BYTES as usize;
    }
    fn append_f64(&mut self, f64: f64) {
        append_f64(self.buffer, f64);
        self.index += CompilationScope::FLOAT_64_BYTES as usize;
    }
    fn append_string_utf8(&mut self, string: &str) {
        let bytes = string.as_bytes();
        self.buffer.extend_from_slice(bytes);
        self.index += bytes.len()
    }
    fn append_buffer(&mut self, buffer: &[u8]) {
        self.buffer.extend_from_slice(buffer);
        self.index += buffer.len()
    }

    fn append_binary_code(&mut self, binary_code: BinaryCode) {
        self.append_u8(binary_code as u8);
    }
}

pub fn compile_body(datex_script: &str) -> Result<Vec<u8>, Box<Error<Rule>>> {
    let pairs = DatexParser::parse(Rule::datex, datex_script).map_err(Box::new)?; //.next().unwrap();

    let mut buffer = Vec::with_capacity(256);
    let compilation_scope = CompilationScope {
        buffer: &mut buffer,
        index: 0,
    };

    parse_statements(compilation_scope, pairs);

    Ok(buffer)
}

fn parse_statements(
    mut compilation_scope: CompilationScope,
    pairs: Pairs<'_, Rule>,
) {
    for statement in pairs {
        parse_atom(&mut compilation_scope, statement);
        // match statement.as_rule() {
        //     Rule::expression => {
        //         parse_expression(&mut compilation_scope, statement);
        //     }
        //     Rule::EOI => {
        //         //
        //     }
        //     _ => unreachable!(),
        // }
    }
}

// apply | term (statements or ident)
fn parse_atom(compilation_scope: &mut CompilationScope, term: Pair<Rule>) {
    let rule = term.as_rule();
    info!(">> RULE {:?}", rule);
    match term.as_rule() {
        Rule::term => {
            for inner in term.into_inner() {
                parse_atom(compilation_scope, inner);
            }
        },
        Rule::ident => {
            parse_ident(compilation_scope, term);
        },

        Rule::level_1_operation | Rule::level_2_operation => {
            let mut inner = term.into_inner();

            let mut prev_operand = inner.next().unwrap();
            let mut current_operator = None;

            loop {
                // every loop iteration: operator, operand
                let operator = inner.next();
                if let Some(operator) = operator {
                    let operation_mode = parse_operator(operator);
                    if current_operator != Some(operation_mode.clone()) {
                        current_operator = Some(operation_mode.clone());
                        compilation_scope.append_binary_code(operation_mode.into());
                    }
                    parse_atom(compilation_scope, prev_operand);
                    prev_operand = inner.next().unwrap();
                }
                // no more operator, add last remaining operand
                else {
                    parse_atom(compilation_scope, prev_operand);
                    break;
                }
            }
        }
        // Rule::expression => {
        //     compilation_scope.append_binary_code(BinaryCode::SCOPE_START);
        //     parse_expression(compilation_scope, term);
        //     compilation_scope.append_binary_code(BinaryCode::SCOPE_END);
        // }

        Rule::EOI => {
            info!("End of input");
        }

        _ => {
            unreachable!(
                "Expected Rule::ident, but found {:?}",
                term.as_rule()
            );
        }
    }
}

/// An ident can only contain a single value
fn parse_ident(compilation_scope: &mut CompilationScope, pair: Pair<'_, Rule>) {
    assert_eq!(pair.as_rule(), Rule::ident, "Expected Rule::ident");

    let ident = pair.into_inner().next().unwrap();
    match ident.as_rule() {
        Rule::integer => {
            let int = ident.as_str().parse::<i64>().unwrap();
            compilation_scope.insert_int(int);
        }
        Rule::decimal => {
            let decimal = ident.as_str().parse::<f64>().unwrap();
            compilation_scope.insert_float64(decimal);
        }
        Rule::text => {
            let string = ident.as_str();
            let inner_string = &string[1..string.len() - 1];
            compilation_scope.insert_string(inner_string);
        }
        _ => {
            unreachable!(
                "Expected Rule::integer, Rule::decimal or Rule::text, but found {:?}",
                ident.as_rule()
            );
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::vec;

    use crate::{global::binary_codes::BinaryCode, logger::init_logger};
    use log::*;

    fn compile_and_log(datex_script: &str) -> Vec<u8> {
        init_logger();
        let result = super::compile_body(datex_script).unwrap();
        debug!(
            "{:?}",
            result
                .iter()
                .map(|x| BinaryCode::try_from(*x).map(|x| x.to_string()))
                .map(|x| x.unwrap_or_else(|_| "Unknown".to_string()))
                .collect::<Vec<_>>()
        );
        result
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
                BinaryCode::MULTIPLY.into(),
                BinaryCode::INT_8.into(),
                lhs,
                BinaryCode::INT_8.into(),
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
                BinaryCode::MULTIPLY.into(),
                BinaryCode::INT_8.into(),
                lhs,
                BinaryCode::INT_8.into(),
                rhs,
                BinaryCode::CLOSE_AND_STORE.into()
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
                BinaryCode::SCOPE_START.into(),
                BinaryCode::INT_8.into(),
                lhs,
                BinaryCode::ADD.into(),
                BinaryCode::INT_8.into(),
                rhs,
                BinaryCode::SCOPE_END.into()
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

        assert_eq!(
            result,
            vec![
                // (
                BinaryCode::SCOPE_START.into(),
                // a
                BinaryCode::INT_8.into(),
                a,
                // +
                BinaryCode::ADD.into(),
                // (
                BinaryCode::SCOPE_START.into(),
                // b
                BinaryCode::INT_8.into(),
                b,
                // +
                BinaryCode::ADD.into(),
                // c
                BinaryCode::INT_8.into(),
                c,
                // )
                BinaryCode::SCOPE_END.into(),
                // )
                BinaryCode::SCOPE_END.into(),
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
        assert_eq!(
            result,
            vec![
                BinaryCode::SCOPE_START.into(),
                BinaryCode::INT_8.into(),
                val,
                BinaryCode::SCOPE_END.into()
            ]
        );
    }

    // Test for decimal
    #[test]
    fn test_decimal() {
        init_logger();
        let val: f64 = 42.1;
        let datex_script = format!("{val}"); // 42.1
        let result = compile_and_log(&datex_script);
        let bytes = val.to_le_bytes();

        let mut expected: Vec<u8> = vec![
            BinaryCode::SCOPE_START.into(),
            BinaryCode::FLOAT_64.into(),
        ];
        expected.extend(bytes);
        expected.push(BinaryCode::SCOPE_END.into());

        assert_eq!(result, expected);
    }

    /// Test for test that is less than 256 characters
    #[test]
    fn test_short_text() {
        init_logger();
        let val = "unyt";
        let datex_script = format!("\"{val}\""); // "42"
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> = vec![
            BinaryCode::SCOPE_START.into(),
            BinaryCode::SHORT_TEXT.into(),
            val.len() as u8,
        ];
        expected.extend(val.bytes());
        expected.push(BinaryCode::SCOPE_END.into());
        assert_eq!(result, expected);
    }
}
