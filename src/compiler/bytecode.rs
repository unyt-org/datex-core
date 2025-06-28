use crate::compiler::parser::{
    parse, BinaryOperator, DatexExpression, DatexScriptParser, TupleEntry,
    VariableType,
};
use crate::compiler::CompilerError;
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

#[derive(Clone, Default)]
pub struct CompileOptions<'a> {
    pub parser: Option<&'a DatexScriptParser<'a>>,
    pub compile_scope: CompileScope,
}

impl CompileOptions<'_> {
    pub fn new_with_scope(compile_scope: CompileScope) -> Self {
        CompileOptions {
            parser: None,
            compile_scope,
        }
    }
}

struct CompilationContext<'a> {
    index: Cell<usize>,
    inserted_value_index: Cell<usize>,
    buffer: RefCell<Vec<u8>>,
    inserted_values: RefCell<&'a [&'a ValueContainer]>,
    /// this flag is set to true if any non-static value is encountered
    has_non_static_value: RefCell<bool>,
}

impl<'a> CompilationContext<'a> {
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

    fn new(
        buffer: RefCell<Vec<u8>>,
        inserted_values: &'a [&'a ValueContainer],
    ) -> Self {
        CompilationContext {
            index: Cell::new(0),
            inserted_value_index: Cell::new(0),
            buffer,
            inserted_values: RefCell::new(inserted_values),
            has_non_static_value: RefCell::new(false),
        }
    }

    fn insert_value_container(&self, value_container: &ValueContainer) {
        self.mark_has_non_static_value();
        match value_container {
            ValueContainer::Value(value) => self.insert_value(value),
            ValueContainer::Reference(reference) => {
                // TODO: in this case, the ref might also be inserted by pointer id, depending on the compiler settings
                // add CREATE_REF instruction
                self.append_binary_code(InstructionCode::CREATE_REF);
                self.insert_value(&reference.borrow().current_resolved_value().borrow())
            } 
        }
    }
    
    fn insert_value(&self, value: &Value) {
        match &value.inner {
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
            CoreValue::Endpoint(endpoint) => self.insert_endpoint(endpoint),
            CoreValue::Decimal(decimal) => self.insert_decimal(decimal),
            CoreValue::TypedDecimal(val) => self.insert_typed_decimal(val),
            CoreValue::Bool(val) => self.insert_boolean(val.0),
            CoreValue::Null => {
                self.append_binary_code(InstructionCode::NULL)
            }
            CoreValue::Text(val) => {
                self.insert_text(&val.0.clone());
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
                // println!("Object: {val:?}");
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

    fn insert_text(&self, string: &str) {
        let bytes = string.as_bytes();
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
        let bytes = key_string.as_bytes();
        let len = bytes.len();

        if len < 256 {
            self.append_binary_code(InstructionCode::KEY_VALUE_SHORT_TEXT);
            self.append_u8(len as u8);
            self.append_buffer(bytes);
        } else {
            self.append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
            self.insert_text(key_string);
        }
    }

    fn insert_typed_decimal(&self, decimal: &TypedDecimal) {
        fn insert_f32_or_f64(
            scope: &CompilationContext,
            decimal: &TypedDecimal,
        ) {
            match decimal {
                TypedDecimal::F32(val) => {
                    scope.insert_float32(val.into_inner());
                }
                TypedDecimal::F64(val) => {
                    scope.insert_float64(val.into_inner());
                }
                TypedDecimal::Decimal(val) => {
                    scope.insert_decimal(val);
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

    fn insert_endpoint(&self, endpoint: &Endpoint) {
        self.append_binary_code(InstructionCode::ENDPOINT);
        self.append_buffer(&endpoint.to_binary());
    }

    fn insert_decimal(&self, decimal: &Decimal) {
        self.append_binary_code(InstructionCode::DECIMAL_BIG);
        // big_decimal binrw write into buffer
        let mut buffer = self.buffer.borrow_mut();
        let original_length = buffer.len();
        let mut buffer_writer = Cursor::new(&mut *buffer);
        // set writer position to end
        buffer_writer.set_position(original_length as u64);
        decimal
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
        if (CompilationContext::MIN_INT_8..=CompilationContext::MAX_INT_8)
            .contains(&int)
        {
            self.insert_i8(int as i8)
        } else if (CompilationContext::MIN_INT_16
            ..=CompilationContext::MAX_INT_16)
            .contains(&int)
        {
            self.insert_i16(int as i16)
        } else if (CompilationContext::MIN_INT_32
            ..=CompilationContext::MAX_INT_32)
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
            .update(|x| x + CompilationContext::INT_8_BYTES as usize);
    }
    fn append_u32(&self, u32: u32) {
        append_u32(self.buffer.borrow_mut().as_mut(), u32);
        self.index
            .update(|x| x + CompilationContext::INT_32_BYTES as usize);
    }
    fn append_i8(&self, i8: i8) {
        append_i8(self.buffer.borrow_mut().as_mut(), i8);
        self.index
            .update(|x| x + CompilationContext::INT_8_BYTES as usize);
    }
    fn append_i16(&self, i16: i16) {
        append_i16(self.buffer.borrow_mut().as_mut(), i16);
        self.index
            .update(|x| x + CompilationContext::INT_16_BYTES as usize);
    }
    fn append_i32(&self, i32: i32) {
        append_i32(self.buffer.borrow_mut().as_mut(), i32);
        self.index
            .update(|x| x + CompilationContext::INT_32_BYTES as usize);
    }
    fn append_i64(&self, i64: i64) {
        append_i64(self.buffer.borrow_mut().as_mut(), i64);
        self.index
            .update(|x| x + CompilationContext::INT_64_BYTES as usize);
    }
    fn append_i128(&self, i128: i128) {
        append_i128(self.buffer.borrow_mut().as_mut(), i128);
        self.index
            .update(|x| x + CompilationContext::INT_128_BYTES as usize);
    }

    fn append_u128(&self, u128: u128) {
        append_u128(self.buffer.borrow_mut().as_mut(), u128);
        self.index
            .update(|x| x + CompilationContext::INT_128_BYTES as usize);
    }

    fn append_f32(&self, f32: f32) {
        append_f32(self.buffer.borrow_mut().as_mut(), f32);
        self.index
            .update(|x| x + CompilationContext::FLOAT_32_BYTES as usize);
    }
    fn append_f64(&self, f64: f64) {
        append_f64(self.buffer.borrow_mut().as_mut(), f64);
        self.index
            .update(|x| x + CompilationContext::FLOAT_64_BYTES as usize);
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

    fn mark_has_non_static_value(&self) {
        self.has_non_static_value.replace(true);
    }

    fn append_binary_code(&self, binary_code: InstructionCode) {
        self.append_u8(binary_code as u8);
    }
}

/// Compiles a DATEX script text into a DXB body
pub fn compile_script<'a>(
    datex_script: &'a str,
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, CompileScope), CompilerError> {
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

fn extract_static_value_from_ast<'a>(
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
) -> Result<(Vec<u8>, CompileScope), CompilerError> {
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
) -> Result<(StaticValueOrDXB, CompileScope), CompilerError> {
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
) -> Result<(StaticValueOrDXB, CompileScope), CompilerError> {
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
    let compilation_context = CompilationContext::new(buffer, inserted_values);

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
) -> Result<(Vec<u8>, CompileScope), CompilerError> {
    compile_template_with_refs(
        datex_script,
        &inserted_values.iter().collect::<Vec<_>>(),
        options,
    )
}

pub fn compile_value<'a>(
    value: &ValueContainer,
) -> Result<Vec<u8>, CompilerError> {
    let buffer = RefCell::new(Vec::with_capacity(256));
    let compilation_scope = CompilationContext::new(buffer, &[]);

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

            $crate::compiler::bytecode::compile_template(&script, values, $crate::compiler::bytecode::CompileOptions::default())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompileScope {
    /// List of variables, mapped by name to their slot address and type.
    variables: HashMap<String, (u32, VariableType)>,
    // TODO: parent variables
    parent_scope: Option<Box<CompileScope>>,
    next_slot_address: u32,
}

impl CompileScope {
    fn register_variable_slot(
        &mut self,
        slot_address: u32,
        variable_type: VariableType,
        name: String,
    ) {
        self.variables
            .insert(name.clone(), (slot_address, variable_type));
    }

    fn get_next_variable_slot(&mut self) -> u32 {
        let slot_address = self.next_slot_address;
        self.next_slot_address += 1;
        slot_address
    }

    fn resolve_variable_slot(&self, name: &str) -> Option<(u32, VariableType)> {
        let mut variables = &self.variables;
        loop {
            if let Some(slot) = variables.get(name) {
                return Some(slot.clone());
            }
            if let Some(parent) = &self.parent_scope {
                variables = &parent.variables;
            } else {
                return None; // variable not found in this scope or any parent scope
            }
        }
    }

    /// Creates a new `CompileScope` that is a child of the current scope.
    fn push(self) -> CompileScope {
        CompileScope {
            next_slot_address: self.next_slot_address,
            parent_scope: Some(Box::new(self)),
            variables: HashMap::new(),
        }
    }

    /// Drops the current scope and returns to the parent scope and a list
    /// of all slot addresses that should be dropped.
    fn pop(self) -> Option<(CompileScope, Vec<u32>)> {
        if let Some(mut parent) = self.parent_scope {
            // update next_slot_address for parent scope
            parent.next_slot_address = self.next_slot_address;
            Some((
                *parent,
                self.variables.keys().map(|k| self.variables[k].0).collect(),
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CompileMetadata {
    scope_required_for_complex_expressions: bool,
    current_binary_operator: Option<BinaryOperator>,
    is_outer_context: bool,
}

impl CompileMetadata {
    fn outer() -> Self {
        CompileMetadata {
            is_outer_context: true,
            ..CompileMetadata::default()
        }
    }

    /// Create CompileMetadata with `scope_required_for_complex_expressions` set to true.
    fn with_scope_required() -> Self {
        CompileMetadata {
            scope_required_for_complex_expressions: true,
            is_outer_context: false,
            ..CompileMetadata::default()
        }
    }
    /// Creates CompileMetadata with the current binary operator set.
    /// Also sets `scope_required_for_complex_expressions` to true.
    fn with_current_binary_operator(operator: BinaryOperator) -> Self {
        CompileMetadata {
            scope_required_for_complex_expressions: true,
            is_outer_context: false,
            current_binary_operator: Some(operator),
        }
    }

    fn must_be_scoped(&self, ast: &DatexExpression) -> bool {
        self.scope_required_for_complex_expressions &&
            // matches a rule that must be scoped
            match ast {
                DatexExpression::BinaryOperation(operator,_,_) => {
                    // only scope if different operator than current
                    if let Some(current_operator) = &self.current_binary_operator {
                        operator != current_operator
                    } else {
                        true
                    }

                }
                _ => false
            }
    }
}

fn compile_ast(
    compilation_scope: &CompilationContext,
    ast: DatexExpression,
    scope: CompileScope,
) -> Result<CompileScope, CompilerError> {
    let scope = compile_expression(
        compilation_scope,
        ast,
        CompileMetadata::outer(),
        scope,
    )?;
    Ok(scope)
}

fn compile_expression(
    compilation_scope: &CompilationContext,
    ast: DatexExpression,
    mut meta: CompileMetadata,
    mut scope: CompileScope,
) -> Result<CompileScope, CompilerError> {
    let scoped = meta.must_be_scoped(&ast);

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_START);
        // immediately reset compile context
        meta = CompileMetadata::default();
    }

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
                    CompileMetadata::with_scope_required(),
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
                            CompileMetadata::with_scope_required(),
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
                let mut child_scope = if !meta.is_outer_context {
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
                if !meta.is_outer_context {
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
            if meta.current_binary_operator != Some(operator.clone()) {
                compilation_scope
                    .append_binary_code(InstructionCode::from(&operator));
            }
            scope = compile_expression(
                compilation_scope,
                *a,
                CompileMetadata::with_current_binary_operator(operator.clone()),
                scope,
            )?;
            scope = compile_expression(
                compilation_scope,
                *b,
                CompileMetadata::with_current_binary_operator(operator),
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
            // close allocation scope
            compilation_scope.append_binary_code(InstructionCode::SCOPE_END);

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

    if scoped {
        compilation_scope.append_binary_code(InstructionCode::SCOPE_END);
    }

    Ok(scope)
}

fn compile_key_value_entry<'a>(
    compilation_scope: &CompilationContext,
    key: DatexExpression,
    value: DatexExpression,
    mut scope: CompileScope,
) -> Result<CompileScope, CompilerError> {
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
                CompileMetadata::with_scope_required(),
                scope,
            )?;
        }
    };
    // insert value
    scope = compile_expression(
        compilation_scope,
        value,
        CompileMetadata::with_scope_required(),
        scope,
    )?;
    Ok(scope)
}

fn insert_int_with_radix<'a>(
    compilation_scope: &CompilationContext,
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
    use super::{
        compile_ast, compile_script, compile_script_or_return_static_value,
        compile_template, CompilationContext, CompileOptions, CompileScope,
        StaticValueOrDXB,
    };
    use std::cell::RefCell;
    use std::io::Read;
    use std::vec;

    use crate::{global::binary_codes::InstructionCode, logger::init_logger};
    use log::*;

    use crate::compiler::parser::parse;
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

    fn get_compilation_scope(script: &str) -> CompilationContext {
        let ast = parse(script);
        let ast = ast.unwrap();
        let buffer = RefCell::new(Vec::with_capacity(256));
        let compilation_scope = CompilationContext::new(buffer, &[]);
        compile_ast(&compilation_scope, ast, CompileScope::default()).unwrap();
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

        let datex_script = "val a = 42; val b = 69; a is b".to_string(); // a is b
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
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                // val b = 69;
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                69,
                InstructionCode::SCOPE_END.into(),
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

        // note: scope is automatically collapsed by the parser since this is all the same operation
        // TODO: we might need to change this to support nested additions, or maybe not if we only allow additions
        // of values of the same type?...
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                a,
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
                InstructionCode::SCOPE_START.into(),
                InstructionCode::SUBTRACT.into(),
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
                InstructionCode::SCOPE_END.into(),
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
                InstructionCode::SCOPE_END.into(),
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
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::SCOPE_END.into(),
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
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                41,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::SCOPE_END.into(),
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
