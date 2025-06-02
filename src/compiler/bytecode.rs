use log::info;
use pest::error::Error;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use regex::Regex;
use crate::compiler::CompilerError;
use crate::compiler::operations::parse_operator;
use crate::compiler::parser::{DatexParser, Rule};
use crate::datex_values::value::DatexValueInner;
use crate::datex_values::value_container::ValueContainer;
use crate::global::binary_codes::InstructionCode;
use crate::utils::buffers::{append_f64, append_i16, append_i32, append_i64, append_i8, append_u32, append_u8};

struct CompilationScope<'a> {
    index: usize,
    inserted_value_index: usize,
    buffer: &'a mut Vec<u8>,
    inserted_values: Vec<ValueContainer>
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
            self.append_binary_code(InstructionCode::TRUE);
        } else {
            self.append_binary_code(InstructionCode::FALSE);
        }
    }

    fn insert_string(&mut self, string: &str) {
        let unescaped_string = self.unescape_string(string);

        let bytes = unescaped_string.as_bytes();
        let len = bytes.len();

        if len < 256 {
            self.append_binary_code(InstructionCode::SHORT_TEXT);
            self.append_u8(len as u8);
        } else {
            self.append_binary_code(InstructionCode::TEXT);
            self.append_u32(len as u32);
        }

        self.append_buffer(bytes);
    }

    fn insert_key_string(&mut self, key_string: &str) {
        let unescaped_string = self.unescape_string(key_string);

        let bytes = unescaped_string.as_bytes();
        let len = bytes.len();

        if len < 256 {
            self.append_binary_code(InstructionCode::KEY_VALUE_SHORT_TEXT);
            self.append_u8(len as u8);
        } else {
            self.append_binary_code(InstructionCode::KEY_VALUE_TEXT);
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
        self.append_binary_code(InstructionCode::FLOAT_64);
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
        self.append_binary_code(InstructionCode::INT_8);
        self.append_i8(int8);
    }
    fn insert_int16(&mut self, int16: i16) {
        self.append_binary_code(InstructionCode::INT_16);
        self.append_i16(int16);
    }
    fn insert_int32(&mut self, int32: i32) {
        self.append_binary_code(InstructionCode::INT_32);
        self.append_i32(int32);
    }
    fn insert_int64(&mut self, int64: i64) {
        self.append_binary_code(InstructionCode::INT_64);
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

    fn append_binary_code(&mut self, binary_code: InstructionCode) {
        self.append_u8(binary_code as u8);
    }
}

/// Compiles a DATEX script text into a DXB body
pub fn compile_script(datex_script: &str) -> Result<Vec<u8>, CompilerError> {
    compile_template(
        datex_script,
        vec![]
    )
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template(
    datex_script: &str,
    inserted_values: Vec<ValueContainer>,
) -> Result<Vec<u8>, CompilerError> {
    let pairs =
        DatexParser::parse(Rule::datex, datex_script)?; //.next().unwrap();

    let mut buffer = Vec::with_capacity(256);
    let compilation_scope = CompilationScope {
        buffer: &mut buffer,
        index: 0,
        inserted_value_index: 0,
        inserted_values,
    };
    parse_statements(compilation_scope, pairs)?;

    Ok(buffer)
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
            let script: String = $fmt.into();
            let values: Vec<$crate::datex_values::value_container::ValueContainer> = vec![$($arg.into()),*];

            $crate::compiler::bytecode::compile_template(&script, values)
        }
    }
}



fn parse_statements(
    mut compilation_scope: CompilationScope,
    pairs: Pairs<'_, Rule>,
) -> Result<(), CompilerError> {
    for statement in pairs {
        match statement.as_rule() {
            Rule::EOI => {}
            _ => {
                parse_atom(&mut compilation_scope, statement, false)?;
            }
        }
    }
    Ok(())
}

fn rule_must_be_scoped(rule: Rule) -> bool {
    matches!(rule, Rule::level_1_operation | Rule::level_2_operation)
}

// apply | term (statements or ident)
fn parse_atom(
    compilation_scope: &mut CompilationScope,
    term: Pair<Rule>,
    scope_required_for_complex_expressions: bool
) -> Result<(), CompilerError> {
    let rule = term.as_rule();
    info!(">> RULE {:?}", rule);

    let scoped = scope_required_for_complex_expressions && rule_must_be_scoped(rule);

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_START);
    }

    match rule {
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
                        compilation_scope
                            .append_binary_code(operation_mode.into());
                    }
                    parse_atom(compilation_scope, prev_operand, true)?;
                    prev_operand = inner.next().unwrap();
                }
                // no more operator, add last remaining operand
                else {
                    parse_atom(compilation_scope, prev_operand, true)?;
                    break;
                }
            }
        }
        Rule::end_of_statement => {
            compilation_scope.append_binary_code(InstructionCode::CLOSE_AND_STORE);
        }

        // is either a Rule::term or a rule that could be inside a term (e.g. literal, integer, array, ...)
        _ => {
            parse_term(compilation_scope, term, scope_required_for_complex_expressions)?;
        }
    }

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
    }

    Ok(())
}

/// A term can only contain a single value
fn parse_term(
    compilation_scope: &mut CompilationScope,
    pair: Pair<'_, Rule>,
    scope_required_for_complex_terms: bool
) -> Result<(), CompilerError> {
    // if Rule::term, get inner rule, else keep rule
    let term = match pair.as_rule() {
        Rule::term => pair.into_inner().next().unwrap(),
        _ => pair,
    };

    let rule = term.as_rule();
    let scoped = scope_required_for_complex_terms && rule_must_be_scoped(rule);

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_START);
    }

    match term.as_rule() {
        Rule::integer => {
            let int = term.as_str().parse::<i64>().unwrap();
            compilation_scope.insert_int(int);
        }
        Rule::decimal => {
            let decimal = term.as_str().parse::<f64>().unwrap();
            compilation_scope.insert_float64(decimal);
        }
        Rule::text => {
            let string = term.as_str();
            let inner_string = &string[1..string.len() - 1];
            compilation_scope.insert_string(inner_string);
        }
        Rule::boolean => {
            let boolean = term.as_str() == "true";
            compilation_scope.insert_boolean(boolean);
        }
        Rule::null => {
            compilation_scope.append_binary_code(InstructionCode::NULL);
        }
        Rule::array => {
            compilation_scope.append_binary_code(InstructionCode::ARRAY_START);
            let inner = term.into_inner();
            for item in inner {
                parse_atom(compilation_scope, item, true)?;
            }
            compilation_scope.append_binary_code(InstructionCode::ARRAY_END);
        }
        Rule::tuple => {
            compilation_scope.append_binary_code(InstructionCode::TUPLE_START);
            let inner = term.into_inner();
            for item in inner {
                parse_atom(compilation_scope, item, true)?;
            }
            compilation_scope.append_binary_code(InstructionCode::TUPLE_END);
        }
        Rule::object => {
            compilation_scope.append_binary_code(InstructionCode::OBJECT_START);
            let inner = term.into_inner();
            for item in inner {
                parse_atom(compilation_scope, item, true)?;
            }
            compilation_scope.append_binary_code(InstructionCode::OBJECT_END);
        }
        Rule::key_value => {
            let mut inner = term.into_inner();
            let key = inner.next().unwrap();
            let value = inner.next().unwrap();

            // select key value type based on key
            // text key: text | literal_key
            // integer key: integer
            // dynamic key: any other type
            let key_rule = key.as_rule();
            match key_rule {
                Rule::text => {
                    let string = key.as_str();
                    let inner_string = &string[1..string.len() - 1];
                    compilation_scope.insert_key_string(inner_string);
                }
                Rule::literal_key => {
                    compilation_scope.insert_key_string(key.as_str());
                }
                _ => {
                    compilation_scope.append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
                    // insert dynamic key
                    parse_atom(compilation_scope, key, true)?;
                }
            }

            // insert value
            parse_atom(compilation_scope, value, true)?;
        }
        Rule::placeholder => {
            let value_container = compilation_scope.inserted_values
                .get(compilation_scope.inserted_value_index)
                .unwrap(); // TODO: bubble up error
            compilation_scope.inserted_value_index += 1;
            match value_container {
                ValueContainer::Value(val) => {
                    match &val.inner {
                        DatexValueInner::I8(val) => {
                            compilation_scope.insert_int8(val.0);
                        }
                        _ => todo!(),
                    }
                }
                _ => todo!(),
            }
        }
        _ => {
            return Err(CompilerError::UnexpectedTerm(term.as_rule()))
            // unreachable!(
            //     "Unexpected term {:?}",
            //     term.as_rule()
            // );
        }
    }

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::{compile_template};
    use std::vec;

    use crate::{global::binary_codes::InstructionCode, logger::init_logger};
    use log::*;

    fn compile_and_log(datex_script: &str) -> Vec<u8> {
        init_logger();
        let result = super::compile_script(datex_script).unwrap();
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

    #[test]
    fn test_simple_multiplication() {
        init_logger();

        // compile("", vec![Datex]);
        //
        // compile!("[{23}]");

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
                InstructionCode::SCOPE_START.into(),
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                op1,
                InstructionCode::INT_8.into(),
                op2,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                op3,
                InstructionCode::INT_8.into(),
                op4,
                InstructionCode::SCOPE_END.into(),
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
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                a,
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                b,
                InstructionCode::INT_8.into(),
                c,
                InstructionCode::SCOPE_END.into(),
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
                InstructionCode::INT_8.into(),
                val,
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

        let mut expected: Vec<u8> =
            vec![InstructionCode::FLOAT_64.into()];
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
        let mut expected: Vec<u8> = vec![
            InstructionCode::SHORT_TEXT.into(),
            val.len() as u8,
        ];
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
            InstructionCode::ARRAY_END.into(),
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
                InstructionCode::ARRAY_END.into(),
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
                InstructionCode::ARRAY_END.into(),
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
                InstructionCode::ARRAY_END.into(),
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::ARRAY_END.into(),
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
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::ARRAY_END.into(),
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
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::ARRAY_END.into(),
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
                InstructionCode::TUPLE_END.into(),
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
                InstructionCode::TUPLE_END.into(),
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::TUPLE_END.into(),
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
                InstructionCode::TUPLE_END.into(),
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
            b'k', b'e', b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::TUPLE_END.into(),
        ];
        assert_eq!(
            result,
            expected,
        );
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
            b'k', b'e', b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::TUPLE_END.into(),
        ];
        assert_eq!(
            result,
            expected,
        );
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
            InstructionCode::TUPLE_END.into(),
        ];
        assert_eq!(
            result,
            expected,
        );
    }

    // dynamic key-value pair
    #[test]
    fn test_dynamic_key_value() {
        init_logger();
        let datex_script = "(1+2): 42";
        let result = compile_and_log(datex_script);
        let expected = [InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::SCOPE_START.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::SCOPE_END.into(),
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::TUPLE_END.into()];
        assert_eq!(
            result,
            expected,
        );
    }

    // multiple key-value pairs
    #[test]
    fn test_multiple_key_value_pairs() {
        init_logger();
        let datex_script = "key: 42, 4: 43, (1+2): 44";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k', b'e', b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::INT_8.into(),
            4,
            InstructionCode::INT_8.into(),
            43,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::SCOPE_START.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::SCOPE_END.into(),
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::TUPLE_END.into(),
        ];
        assert_eq!(
            result,
            expected,
        );
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
            b'k', b'e', b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::TUPLE_END.into(),
        ];
        assert_eq!(
            result,
            expected,
        );
    }

    // empty object
    #[test]
    fn test_empty_object() {
        init_logger();
        let datex_script = "{}";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::OBJECT_START.into(),
            InstructionCode::OBJECT_END.into(),
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
            b'k', b'e', b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::OBJECT_END.into(),
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
            b'k', b'e', b'y', b'1',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key2"
            b'k', b'e', b'y', b'2',
            InstructionCode::INT_8.into(),
            43,
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key3"
            b'k', b'e', b'y', b'3',
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::OBJECT_END.into(),
        ];
        assert_eq!(result, expected);
    }



    #[test]
    fn test_compile() {
        init_logger();
        let result = compile_template(
            "? + ?", vec![1.into(), 2.into()]);
        assert_eq!(
            result.unwrap(),
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
        let result = compile!("?", 1);
        assert_eq!(
            result.unwrap(),
            vec![
                InstructionCode::INT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn test_compile_macro_multi() {
        init_logger();
        let result = compile!("? + ?", 1, 2);
        assert_eq!(
            result.unwrap(),
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2
            ]
        );
    }
}
