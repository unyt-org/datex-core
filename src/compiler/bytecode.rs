use crate::compiler::operations::parse_operator;
use crate::compiler::parser::{DatexParser, Rule};
use crate::compiler::CompilerError;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::decimal::big_decimal::ExtendedBigDecimal;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
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
use log::info;
use num_traits::ToPrimitive;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use regex::Regex;
use std::cell::{Cell, RefCell};
use std::io::Cursor;

struct CompilationScope {
    index: Cell<usize>,
    inserted_value_index: Cell<usize>,
    buffer: RefCell<Vec<u8>>,
    inserted_values: RefCell<Vec<ValueContainer>>,
}

impl CompilationScope {
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
    const INT_128_BYTES: u8 = 16;

    const FLOAT_32_BYTES: u8 = 4;
    const FLOAT_64_BYTES: u8 = 8;

    fn insert_value_container(&self, value_container: &ValueContainer) {
        match value_container {
            ValueContainer::Value(val) => match &val.inner {
                CoreValue::TypedInteger(val)
                | CoreValue::Integer(Integer(val)) => {
                    match val.to_smallest_fitting() {
                        TypedInteger::I8(val) => {
                            self.insert_i8(val);
                        }
                        TypedInteger::I16(val) => {
                            self.insert_i16(val);
                        }
                        TypedInteger::I32(val) => {
                            self.insert_i32(val);
                        }
                        TypedInteger::I64(val) => {
                            self.insert_i64(val);
                        }
                        TypedInteger::I128(val) => {
                            self.insert_i128(val);
                        }
                        TypedInteger::U8(val) => {
                            self.insert_u8(val);
                        }
                        TypedInteger::U16(val) => {
                            self.insert_u16(val);
                        }
                        TypedInteger::U32(val) => {
                            self.insert_u32(val);
                        }
                        TypedInteger::U64(val) => {
                            self.insert_u64(val);
                        }
                        TypedInteger::U128(val) => {
                            self.insert_u128(val);
                        }
                    }
                }
                CoreValue::Decimal(Decimal(val)) => match val {
                    TypedDecimal::Big(val) => {
                        self.insert_big_decimal(val);
                    }
                    _ => unreachable!("Decimal must contain TypedDecimal::Big"),
                },
                CoreValue::TypedDecimal(val) => self.insert_decimal(val),
                CoreValue::Bool(val) => self.insert_boolean(val.0),
                CoreValue::Null => {
                    self.append_binary_code(InstructionCode::NULL)
                }
                CoreValue::Text(val) => {
                    self.insert_string(&val.0.clone());
                }
                CoreValue::Array(val) => {
                    self.append_binary_code(InstructionCode::ARRAY_START);
                    for item in val {
                        self.insert_value_container(item);
                    }
                    self.append_binary_code(InstructionCode::SCOPE_END);
                }
                CoreValue::Object(val) => {
                    self.append_binary_code(InstructionCode::OBJECT_START);
                    println!("Object: {val:?}");
                    for (key, value) in val {
                        self.insert_key_string(key);
                        self.insert_value_container(value);
                    }
                    self.append_binary_code(InstructionCode::SCOPE_END);
                }
                CoreValue::Tuple(val) => {
                    self.append_binary_code(InstructionCode::TUPLE_START);
                    let mut next_expected_integer_key: i128 = 0;
                    for (key, value) in val {
                        // if next expected integer key, ignore and just insert value
                        if let ValueContainer::Value(key) = key
                            && let CoreValue::Integer(Integer(integer)) =
                                key.inner
                            && let Some(int) = integer.as_i128()
                            && int == next_expected_integer_key
                        {
                            next_expected_integer_key += 1;
                            self.insert_value_container(value);
                        } else {
                            self.insert_key_value_pair(key, value);
                        }
                    }
                    self.append_binary_code(InstructionCode::SCOPE_END);
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    // value insert functions
    fn insert_boolean(&self, boolean: bool) {
        if boolean {
            self.append_binary_code(InstructionCode::TRUE);
        } else {
            self.append_binary_code(InstructionCode::FALSE);
        }
    }

    fn insert_string(&self, string: &str) {
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

    fn insert_key_value_pair(
        &self,
        key: &ValueContainer,
        value: &ValueContainer,
    ) {
        // insert key
        match key {
            // if text, insert_key_string, else dynamic
            ValueContainer::Value(Value {
                inner: CoreValue::Text(text),
                ..
            }) => {
                self.insert_key_string(&text.0);
            }
            _ => {
                self.append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
                self.insert_value_container(key);
            }
        }
        // insert value
        self.insert_value_container(value);
    }

    fn insert_key_string(&self, key_string: &str) {
        let unescaped_string = self.unescape_string(key_string);

        let bytes = unescaped_string.as_bytes();
        let len = bytes.len();

        if len < 256 {
            self.append_binary_code(InstructionCode::KEY_VALUE_SHORT_TEXT);
            self.append_u8(len as u8);
            self.append_buffer(bytes);
        } else {
            self.append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
            self.insert_string(key_string);
        }
    }

    fn unescape_string(&self, string: &str) -> String {
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

    fn insert_decimal(&self, decimal: &TypedDecimal) {
        fn insert_f32_or_f64(scope: &CompilationScope, decimal: &TypedDecimal) {
            match decimal {
                TypedDecimal::F32(val) => {
                    scope.insert_float32(val.into_inner());
                }
                TypedDecimal::F64(val) => {
                    scope.insert_float64(val.into_inner());
                }
                TypedDecimal::Big(val) => {
                    scope.insert_big_decimal(val);
                }
            }
        }

        match decimal.as_integer() {
            Some(int) => {
                let smallest = smallest_fitting_signed(int as i128);
                match smallest {
                    TypedInteger::I8(val) => {
                        self.insert_float_as_i16(val as i16);
                    }
                    TypedInteger::I16(val) => {
                        self.insert_float_as_i16(val);
                    }
                    TypedInteger::I32(val) => {
                        self.insert_float_as_i32(val);
                    }
                    _ => insert_f32_or_f64(self, decimal),
                }
            }
            None => insert_f32_or_f64(self, decimal),
        }
    }

    fn insert_float32(&self, float32: f32) {
        self.append_binary_code(InstructionCode::DECIMAL_F32);
        self.append_f32(float32);
    }
    fn insert_float64(&self, float64: f64) {
        self.append_binary_code(InstructionCode::DECIMAL_F64);
        self.append_f64(float64);
    }

    fn insert_big_decimal(&self, big_decimal: &ExtendedBigDecimal) {
        self.append_binary_code(InstructionCode::DECIMAL_BIG);
        // big_decimal binrw write into buffer
        let mut buffer = self.buffer.borrow_mut();
        let original_length = buffer.len();
        let mut buffer_writer = Cursor::new(&mut *buffer);
        // set writer position to end
        buffer_writer.set_position(original_length as u64);
        big_decimal
            .write_le(&mut buffer_writer)
            .expect("Failed to write big decimal");
        // get byte count of written data
        let byte_count = buffer_writer.position() as usize;
        // update index
        self.index.update(|x| x + byte_count - original_length);
    }

    fn insert_float_as_i16(&self, int: i16) {
        self.append_binary_code(InstructionCode::DECIMAL_AS_INT_16);
        self.append_i16(int);
    }
    fn insert_float_as_i32(&self, int: i32) {
        self.append_binary_code(InstructionCode::DECIMAL_AS_INT_32);
        self.append_i32(int);
    }

    fn insert_int(&self, int: i64) {
        if (CompilationScope::MIN_INT_8..=CompilationScope::MAX_INT_8)
            .contains(&int)
        {
            self.insert_i8(int as i8)
        } else if (CompilationScope::MIN_INT_16..=CompilationScope::MAX_INT_16)
            .contains(&int)
        {
            self.insert_i16(int as i16)
        } else if (CompilationScope::MIN_INT_32..=CompilationScope::MAX_INT_32)
            .contains(&int)
        {
            self.insert_i32(int as i32)
        } else {
            self.insert_i64(int)
        }
    }

    fn insert_i8(&self, int8: i8) {
        self.append_binary_code(InstructionCode::INT_8);
        self.append_i8(int8);
    }

    fn insert_i16(&self, int16: i16) {
        self.append_binary_code(InstructionCode::INT_16);
        self.append_i16(int16);
    }
    fn insert_i32(&self, int32: i32) {
        self.append_binary_code(InstructionCode::INT_32);
        self.append_i32(int32);
    }
    fn insert_i64(&self, int64: i64) {
        self.append_binary_code(InstructionCode::INT_64);
        self.append_i64(int64);
    }
    fn insert_i128(&self, int128: i128) {
        self.append_binary_code(InstructionCode::INT_128);
        self.append_i128(int128);
    }
    fn insert_u8(&self, uint8: u8) {
        self.append_binary_code(InstructionCode::INT_16);
        self.append_i16(uint8 as i16);
    }
    fn insert_u16(&self, uint16: u16) {
        self.append_binary_code(InstructionCode::INT_32);
        self.append_i32(uint16 as i32);
    }
    fn insert_u32(&self, uint32: u32) {
        self.append_binary_code(InstructionCode::INT_64);
        self.append_i64(uint32 as i64);
    }
    fn insert_u64(&self, uint64: u64) {
        self.append_binary_code(InstructionCode::INT_128);
        self.append_i128(uint64 as i128);
    }
    fn insert_u128(&self, uint128: u128) {
        self.append_binary_code(InstructionCode::UINT_128);
        self.append_i128(uint128 as i128);
    }
    fn append_u8(&self, u8: u8) {
        append_u8(self.buffer.borrow_mut().as_mut(), u8);
        self.index
            .update(|x| x + CompilationScope::INT_8_BYTES as usize);
    }
    fn append_u32(&self, u32: u32) {
        append_u32(self.buffer.borrow_mut().as_mut(), u32);
        self.index
            .update(|x| x + CompilationScope::INT_32_BYTES as usize);
    }
    fn append_i8(&self, i8: i8) {
        append_i8(self.buffer.borrow_mut().as_mut(), i8);
        self.index
            .update(|x| x + CompilationScope::INT_8_BYTES as usize);
    }
    fn append_i16(&self, i16: i16) {
        append_i16(self.buffer.borrow_mut().as_mut(), i16);
        self.index
            .update(|x| x + CompilationScope::INT_16_BYTES as usize);
    }
    fn append_i32(&self, i32: i32) {
        append_i32(self.buffer.borrow_mut().as_mut(), i32);
        self.index
            .update(|x| x + CompilationScope::INT_32_BYTES as usize);
    }
    fn append_i64(&self, i64: i64) {
        append_i64(self.buffer.borrow_mut().as_mut(), i64);
        self.index
            .update(|x| x + CompilationScope::INT_64_BYTES as usize);
    }
    fn append_i128(&self, i128: i128) {
        append_i128(self.buffer.borrow_mut().as_mut(), i128);
        self.index
            .update(|x| x + CompilationScope::INT_128_BYTES as usize);
    }

    fn append_u128(&self, u128: u128) {
        append_u128(self.buffer.borrow_mut().as_mut(), u128);
        self.index
            .update(|x| x + CompilationScope::INT_128_BYTES as usize);
    }

    fn append_f32(&self, f32: f32) {
        append_f32(self.buffer.borrow_mut().as_mut(), f32);
        self.index
            .update(|x| x + CompilationScope::FLOAT_32_BYTES as usize);
    }
    fn append_f64(&self, f64: f64) {
        append_f64(self.buffer.borrow_mut().as_mut(), f64);
        self.index
            .update(|x| x + CompilationScope::FLOAT_64_BYTES as usize);
    }
    fn append_string_utf8(&self, string: &str) {
        let bytes = string.as_bytes();
        (*self.buffer.borrow_mut()).extend_from_slice(bytes);
        self.index.update(|x| x + bytes.len());
    }
    fn append_buffer(&self, buffer: &[u8]) {
        (*self.buffer.borrow_mut()).extend_from_slice(buffer);
        self.index.update(|x| x + buffer.len());
    }

    fn append_binary_code(&self, binary_code: InstructionCode) {
        self.append_u8(binary_code as u8);
    }
}

/// Compiles a DATEX script text into a DXB body
pub fn compile_script(datex_script: &str) -> Result<Vec<u8>, CompilerError> {
    compile_template(datex_script, vec![])
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template(
    datex_script: &str,
    inserted_values: Vec<ValueContainer>,
) -> Result<Vec<u8>, CompilerError> {
    let pairs = DatexParser::parse(Rule::datex, datex_script)?; //.next().unwrap();

    let buffer = RefCell::new(Vec::with_capacity(256));
    let mut compilation_scope = CompilationScope {
        buffer,
        index: Cell::new(0),
        inserted_value_index: Cell::new(0),
        inserted_values: RefCell::new(inserted_values),
    };
    parse_statements(&mut compilation_scope, pairs)?;

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
            let script: String = $fmt.into();
            let values: Vec<$crate::datex_values::value_container::ValueContainer> = vec![$($arg.into()),*];

            $crate::compiler::bytecode::compile_template(&script, values)
        }
    }
}

fn parse_statements(
    compilation_scope: &mut CompilationScope,
    pairs: Pairs<'_, Rule>,
) -> Result<(), CompilerError> {
    for statement in pairs {
        match statement.as_rule() {
            Rule::EOI => {}
            _ => {
                parse_atom(compilation_scope, statement, false)?;
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
    compilation_scope: &CompilationScope,
    term: Pair<Rule>,
    scope_required_for_complex_expressions: bool,
) -> Result<(), CompilerError> {
    let rule = term.as_rule();
    info!(">> RULE {:?}", rule);

    let scoped =
        scope_required_for_complex_expressions && rule_must_be_scoped(rule);

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
            compilation_scope
                .append_binary_code(InstructionCode::CLOSE_AND_STORE);
        }

        // is either a Rule::term or a rule that could be inside a term (e.g. literal, integer, array, ...)
        _ => {
            parse_term(
                compilation_scope,
                term,
                scope_required_for_complex_expressions,
            )?;
        }
    }

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
    }

    Ok(())
}

/// A term can only contain a single value
fn parse_term(
    compilation_scope: &CompilationScope,
    pair: Pair<'_, Rule>,
    scope_required_for_complex_terms: bool,
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
        Rule::dec_integer => {
            let int = term.as_str().parse::<i64>().unwrap();
            compilation_scope.insert_int(int);
        }
        Rule::hex_integer => {
            insert_int_with_radix(compilation_scope, term.as_str(), 16)?;
        }
        Rule::oct_integer => {
            insert_int_with_radix(compilation_scope, term.as_str(), 8)?;
        }
        Rule::bin_integer => {
            insert_int_with_radix(compilation_scope, term.as_str(), 2)?;
        }
        Rule::decimal => {
            let decimal = ExtendedBigDecimal::from_string(term.as_str())
                .ok_or(CompilerError::BigDecimalOutOfBoundsError)?;
            match &decimal {
                ExtendedBigDecimal::Finite(big_decimal)
                    if big_decimal.is_integer() =>
                {
                    if let Some(int) = big_decimal.to_i16() {
                        compilation_scope.insert_float_as_i16(int);
                    } else if let Some(int) = big_decimal.to_i32() {
                        compilation_scope.insert_float_as_i32(int);
                    } else {
                        compilation_scope.insert_big_decimal(&decimal);
                    }
                }
                _ => {
                    compilation_scope.insert_big_decimal(&decimal);
                }
            }
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
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
        }
        Rule::tuple => {
            compilation_scope.append_binary_code(InstructionCode::TUPLE_START);
            let inner = term.into_inner();
            for item in inner {
                parse_atom(compilation_scope, item, true)?;
            }
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
        }
        Rule::object => {
            compilation_scope.append_binary_code(InstructionCode::OBJECT_START);
            let inner = term.into_inner();
            for item in inner {
                parse_atom(compilation_scope, item, true)?;
            }
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
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
                    compilation_scope
                        .append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
                    // insert dynamic key
                    parse_atom(compilation_scope, key, true)?;
                }
            }

            // insert value
            parse_atom(compilation_scope, value, true)?;
        }
        Rule::placeholder => {
            compilation_scope.insert_value_container(
                compilation_scope
                    .inserted_values
                    .borrow()
                    .get(compilation_scope.inserted_value_index.get())
                    .unwrap(),
            );
            compilation_scope.inserted_value_index.update(|x| x + 1);
        }
        _ => {
            return Err(CompilerError::UnexpectedTerm(term.as_rule()));
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

fn insert_int_with_radix(
    compilation_scope: &CompilationScope,
    int_str: &str,
    radix: u32,
) -> Result<(), CompilerError> {
    let is_negative = int_str.starts_with('-');
    let is_positive = int_str.starts_with('+');
    let int = i64::from_str_radix(
        &int_str[if is_negative || is_positive { 3 } else { 2 }..],
        radix,
    )
    .map_err(|_| CompilerError::IntegerOutOfBoundsError)?;
    if is_negative {
        compilation_scope.insert_int(-int);
    } else {
        compilation_scope.insert_int(int);
    }
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::compile_template;
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
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
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
        assert_eq!(result, expected,);
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
        assert_eq!(result, expected,);
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
        assert_eq!(result, expected,);
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
        assert_eq!(result, expected,);
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
            InstructionCode::SCOPE_START.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::SCOPE_END.into(),
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected,);
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
            InstructionCode::SCOPE_START.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::SCOPE_END.into(),
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected,);
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
        assert_eq!(result, expected,);
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
    fn test_compile() {
        init_logger();
        let result = compile_template("? + ?", vec![1.into(), 2.into()]);
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
        assert_eq!(result.unwrap(), vec![InstructionCode::INT_8.into(), 1,]);
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
