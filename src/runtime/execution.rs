use super::stack::{ActiveValue, ScopeStack, ScopeType};
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::array::Array;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::datex_values::core_values::integer::integer::Integer;
use crate::datex_values::core_values::object::Object;
use crate::datex_values::core_values::tuple::Tuple;
use crate::datex_values::traits::structural_eq::StructuralEq;
use crate::datex_values::traits::value_eq::ValueEq;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::{ValueContainer, ValueError};
use crate::global::protocol_structures::instructions::{
    DecimalData, Float32Data, Float64Data, FloatAsInt16Data, FloatAsInt32Data,
    Instruction, ShortTextData, SlotAddress, TextData,
};
use crate::parser::body;
use crate::parser::body::DXBParserError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionInput<'a> {
    pub options: ExecutionOptions,
    pub dxb_body: &'a [u8],
    pub context: ExecutionContext,
}

impl<'a> ExecutionInput<'a> {
    pub fn new_with_dxb_and_options(
        dxb_body: &'a [u8],
        options: ExecutionOptions,
    ) -> Self {
        Self {
            options,
            dxb_body,
            context: ExecutionContext::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    index: usize,
    scope_stack: ScopeStack,
    slots: RefCell<HashMap<u32, Option<ValueContainer>>>,
}

impl ExecutionContext {
    pub fn reset_index(&mut self) {
        self.index = 0;
    }

    /// Allocates a new slot with the given slot address.
    fn allocate_slot(&self, address: u32, value: Option<ValueContainer>) {
        self.slots.borrow_mut().insert(address, value);
    }

    /// Drops a slot by its address, returning the value if it existed.
    /// If the slot is not allocated, it returns an error.
    fn drop_slot(
        &self,
        address: u32,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .borrow_mut()
            .remove(&address)
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }

    /// Sets the value of a slot, returning the previous value if it existed.
    /// If the slot is not allocated, it returns an error.
    fn set_slot_value(
        &self,
        address: u32,
        value: ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .borrow_mut()
            .insert(address, Some(value))
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }

    /// Retrieves the value of a slot by its address.
    /// If the slot is not allocated, it returns an error.
    fn get_slot_value(
        &self,
        address: u32,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        self.slots
            .borrow_mut()
            .get(&address)
            .cloned()
            .ok_or(())
            .map_err(|_| ExecutionError::SlotNotAllocated(address))
    }
}

pub fn execute_dxb(
    input: ExecutionInput,
) -> Result<(Option<ValueContainer>, ExecutionContext), ExecutionError> {
    execute_loop(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidProgramError {
    InvalidScopeClose,
    InvalidKeyValuePair,
    // any unterminated sequence, e.g. missing key in key-value pair
    UnterminatedSequence,
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
            InvalidProgramError::UnterminatedSequence => {
                write!(f, "Unterminated sequence")
            }
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    ParserError(DXBParserError),
    ValueError(ValueError),
    InvalidProgram(InvalidProgramError),
    Unknown,
    NotImplemented(String),
    SlotNotAllocated(u32),
    SlotNotInitialized(u32),
}

impl From<DXBParserError> for ExecutionError {
    fn from(error: DXBParserError) -> Self {
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
            ExecutionError::SlotNotAllocated(address) => {
                write!(
                    f,
                    "Tried to access unallocated slot at address {address}"
                )
            }
            ExecutionError::SlotNotInitialized(address) => {
                write!(
                    f,
                    "Tried to access uninitialized slot at address {address}"
                )
            }
        }
    }
}

pub fn execute_loop(
    input: ExecutionInput,
) -> Result<(Option<ValueContainer>, ExecutionContext), ExecutionError> {
    let dxb_body = input.dxb_body;
    let mut context = input.context;

    let instruction_iterator = body::iterate_instructions(dxb_body);

    for instruction in instruction_iterator {
        let instruction = instruction?;
        if input.options.verbose {
            println!("[Exec]: {instruction}");
        }

        let mut is_scope_start = false;

        let value: ActiveValue = match instruction {
            // boolean
            Instruction::True => true.into(),
            Instruction::False => false.into(),

            // integers
            Instruction::Int8(integer) => Integer::from(integer.0).into(),
            Instruction::Int16(integer) => Integer::from(integer.0).into(),
            Instruction::Int32(integer) => Integer::from(integer.0).into(),
            Instruction::Int64(integer) => Integer::from(integer.0).into(),
            Instruction::Int128(integer) => Integer::from(integer.0).into(),

            // unsigned integers
            Instruction::UInt128(integer) => Integer::from(integer.0).into(),

            // specific floats
            Instruction::DecimalF32(Float32Data(f32)) => {
                TypedDecimal::from(f32).into()
            }
            Instruction::DecimalF64(Float64Data(f64)) => {
                TypedDecimal::from(f64).into()
            }

            // default decimals (big decimals)
            Instruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                Decimal::from(i16 as f32).into()
            }
            Instruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                Decimal::from(i32 as f32).into()
            }
            Instruction::Decimal(DecimalData(big_decimal)) => {
                big_decimal.into()
            }

            // endpoint
            Instruction::Endpoint(endpoint) => endpoint.into(),

            // null
            Instruction::Null => Value::null().into(),

            // text
            Instruction::ShortText(ShortTextData(text)) => text.into(),
            Instruction::Text(TextData(text)) => text.into(),

            // operations
            Instruction::Add => {
                context.scope_stack.set_active_operation(Instruction::Add);
                ActiveValue::None
            }

            Instruction::Subtract => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::Subtract);
                ActiveValue::None
            }

            Instruction::Multiply => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::Multiply);
                ActiveValue::None
            }

            Instruction::Divide => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::Divide);
                ActiveValue::None
            }

            Instruction::EqualValue => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::EqualValue);
                ActiveValue::None
            }
            Instruction::StrictEqual => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::StrictEqual);
                ActiveValue::None
            }
            Instruction::NotEqualValue => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::NotEqualValue);
                ActiveValue::None
            }
            Instruction::StrictNotEqual => {
                context
                    .scope_stack
                    .set_active_operation(Instruction::StrictNotEqual);
                ActiveValue::None
            }

            Instruction::CloseAndStore => {
                let active = context.scope_stack.pop_active_value();
                ActiveValue::None
                // try_assign_active_value_to_active_slot(&mut context)?
            }

            Instruction::ScopeStart => {
                context.scope_stack.create_scope(ScopeType::Default);
                ActiveValue::None
            }

            Instruction::ArrayStart => {
                context.scope_stack.create_scope(ScopeType::Array);
                is_scope_start = true;
                Array::default().into()
            }

            Instruction::ObjectStart => {
                context.scope_stack.create_scope(ScopeType::Object);
                is_scope_start = true;
                Object::default().into()
            }

            Instruction::TupleStart => {
                context.scope_stack.create_scope(ScopeType::Tuple);
                is_scope_start = true;
                Tuple::default().into()
            }

            Instruction::KeyValueShortText(ShortTextData(key)) => {
                context.scope_stack.set_active_key(key.into());
                ActiveValue::None
            }

            Instruction::KeyValueDynamic => {
                context.scope_stack.set_active_key(ActiveValue::None);
                ActiveValue::None
            }

            Instruction::ScopeEnd => {
                // if has active_slot, assign value
                if let Some(active_slot) = context.scope_stack.get_active_slot()
                    && let ActiveValue::ValueContainer(active) =
                        context.scope_stack.get_active_value()
                {
                    // write to slot
                    context.set_slot_value(active_slot, active.clone())?;
                }

                // pop scope and return value
                context.scope_stack.pop()?
            }

            // slots
            Instruction::AllocateSlot(SlotAddress(address)) => {
                context.allocate_slot(address, None);
                context.scope_stack.create_scope(ScopeType::SlotAssignment);
                context.scope_stack.set_active_slot(address);
                ActiveValue::None
            }
            Instruction::GetSlot(SlotAddress(address)) => {
                // get value from slot
                let slot_value = context.get_slot_value(address)?;
                if slot_value.is_none() {
                    return Err(ExecutionError::SlotNotInitialized(address));
                }
                slot_value.into()
            }
            Instruction::UpdateSlot(SlotAddress(address)) => {
                context.scope_stack.create_scope(ScopeType::SlotAssignment);
                context.scope_stack.set_active_slot(address);
                ActiveValue::None
            }

            Instruction::DropSlot(SlotAddress(address)) => {
                // remove slot from slots
                context.drop_slot(address)?;
                ActiveValue::None
            }

            i => {
                return Err(ExecutionError::NotImplemented(
                    format!("Instruction {i}").to_string(),
                ));
            }
        };

        handle_value(&mut context, is_scope_start, value)?;
    }

    // final cleanup of the current scope:

    // // try to assign the remaining active value to the active slot
    // if context.scope_stack.get_active_slot().is_some() {
    //     let active_value = try_assign_active_value_to_active_slot(&mut context)?;
    //     if active_value.is_some() {
    //         handle_value(
    //             &mut context,
    //             false,
    //             active_value,
    //         )?;
    //     }
    // }

    // if we have an active key here, this is invalid and leads to an error
    if context.scope_stack.get_active_key().is_some() {
        return Err(ExecutionError::InvalidProgram(
            InvalidProgramError::UnterminatedSequence,
        ));
    }
    // clear active operation if any
    context.scope_stack.clear_active_operation();

    // removes the current active value from the scope stack
    Ok(match context.scope_stack.pop_active_value() {
        ActiveValue::None => (None, context),
        ActiveValue::ValueContainer(val) => (Some(val), context),
    })
}

/// Takes a produced value and handles it according to the current context, scope and active operation.
fn handle_value(
    context: &mut ExecutionContext,
    is_scope_start: bool,
    value: ActiveValue,
) -> Result<(), ExecutionError> {
    match value {
        ActiveValue::ValueContainer(value_container) => {
            // TODO: try to optimize and initialize variables only when needed, currently leeds to borrow errors
            let active_operation =
                context.scope_stack.get_active_operation().cloned();
            let scope_type =
                context.scope_stack.get_current_scope_type().clone();
            let active_key = context.scope_stack.get_active_key();
            let active_value = context.scope_stack.get_active_value_mut();

            // check if active_key_value_pair exists
            if let Some(active_key) = active_key {
                match active_key {
                    // set key for key-value pair (for dynamic keys)
                    ActiveValue::None => {
                        context
                            .scope_stack
                            .set_active_key(value_container.into());
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
                                        object.set(&key_str.0, value_container);
                                    }
                                    _ => {
                                        return Err(ExecutionError::InvalidProgram(InvalidProgramError::InvalidKeyValuePair));
                                    }
                                }
                            }
                            // tuple
                            ActiveValue::ValueContainer(
                                ValueContainer::Value(Value {
                                    inner: CoreValue::Tuple(tuple),
                                    ..
                                }),
                            ) => {
                                // set key-value pair in tuple
                                tuple.set(key, value_container);
                            }
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
                        context
                            .scope_stack
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
                                Instruction::Subtract => {
                                    active_value_container as &_
                                        - &value_container
                                }
                                Instruction::EqualValue => {
                                    let val = active_value_container
                                        .value_eq(&value_container);
                                    Ok(ValueContainer::from(val))
                                }
                                Instruction::StrictEqual => {
                                    let val = active_value_container
                                        .structural_eq(&value_container);
                                    Ok(ValueContainer::from(val))
                                }
                                Instruction::NotEqualValue => {
                                    let val = !active_value_container
                                        .value_eq(&value_container);
                                    Ok(ValueContainer::from(val))
                                }
                                Instruction::StrictNotEqual => {
                                    let val = !active_value_container
                                        .structural_eq(&value_container);
                                    Ok(ValueContainer::from(val))
                                }
                                _ => {
                                    unreachable!("Instruction {:?} is not a valid operation", operation);
                                }
                            };
                            if let Ok(val) = res {
                                // set active value to operation result
                                context
                                    .scope_stack
                                    .set_active_value_container(val);
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
                        // special scope: Tuple
                        else if !is_scope_start
                            && scope_type == ScopeType::Tuple
                        {
                            // add value to array scope
                            match active_value_container {
                                ValueContainer::Value(Value {
                                    inner: CoreValue::Tuple(tuple),
                                    ..
                                }) => {
                                    // automatic tuple keys are always default integer values
                                    let index = CoreValue::Integer(
                                        Integer::from(tuple.next_int_key()),
                                    );
                                    tuple.set(index, value_container);
                                }
                                _ => {
                                    unreachable!("Expected active value in tuple scope to be a tuple, but got: {}", active_value_container);
                                }
                            }
                        }
                    }
                }
            }
        }

        ActiveValue::None => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::vec;

    use log::debug;

    use super::*;
    use crate::compiler::bytecode::{compile_script, CompileOptions};
    use crate::datex_values::traits::structural_eq::StructuralEq;
    use crate::global::binary_codes::InstructionCode;
    use crate::logger::init_logger;
    use crate::{assert_structural_eq, datex_array};

    fn execute_datex_script_debug(
        datex_script: &str,
    ) -> Option<ValueContainer> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context = ExecutionInput::new_with_dxb_and_options(
            &dxb,
            ExecutionOptions { verbose: true },
        );
        execute_dxb(context)
            .unwrap_or_else(|err| {
                panic!("Execution failed: {err}");
            })
            .0
    }

    fn execute_datex_script_debug_with_result(
        datex_script: &str,
    ) -> ValueContainer {
        execute_datex_script_debug(datex_script).unwrap()
    }

    fn execute_dxb_debug(
        dxb_body: &[u8],
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let context = ExecutionInput::new_with_dxb_and_options(
            dxb_body,
            ExecutionOptions { verbose: true },
        );
        execute_dxb(context).map(|(result, _)| result)
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
        assert_eq!(
            execute_datex_script_debug_with_result("42"),
            Integer::from(42).into()
        );
    }

    #[test]
    fn test_single_value_semicolon() {
        assert_eq!(execute_datex_script_debug("42;"), None)
    }

    #[test]
    fn test_equality() {
        let result = execute_datex_script_debug_with_result("1 == 1");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("1 == 2");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));

        let result = execute_datex_script_debug_with_result("1 != 2");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("1 != 1");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
        let result = execute_datex_script_debug_with_result("1 === 1");
        assert_eq!(result, true.into());

        assert_structural_eq!(result, ValueContainer::from(true));
        let result = execute_datex_script_debug_with_result("1 !== 2");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("1 !== 1");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn test_single_value_scope() {
        let result = execute_datex_script_debug_with_result("(42)");
        assert_eq!(result, Integer::from(42).into());
        assert_structural_eq!(result, ValueContainer::from(42_u128));
    }

    #[test]
    fn test_add() {
        let result = execute_datex_script_debug_with_result("1 + 2");
        assert_structural_eq!(result, ValueContainer::from(3_u128));
        assert_eq!(result, Integer::from(3).into());
    }

    #[test]
    fn test_nested_scope() {
        let result = execute_datex_script_debug_with_result("1 + (2 + 3)");
        assert_eq!(result, Integer::from(6).into());
    }

    #[test]
    fn test_invalid_scope_close() {
        let result = execute_dxb_debug(&vec![
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
        let array: Array = result.to_value().borrow().cast_to_array().unwrap();
        assert_eq!(array.len(), 0);
        assert_eq!(result, Vec::<ValueContainer>::new().into());
        assert_eq!(result, ValueContainer::from(Vec::<ValueContainer>::new()));
    }

    #[test]
    fn test_array() {
        let result = execute_datex_script_debug_with_result("[1, 2, 3]");
        let array: Array = result.to_value().borrow().cast_to_array().unwrap();
        let expected =
            datex_array![Integer::from(1), Integer::from(2), Integer::from(3)];
        assert_eq!(array.len(), 3);
        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1, 2, 3]));
        assert_structural_eq!(result, ValueContainer::from(vec![1, 2, 3]));
    }

    #[test]
    fn test_array_with_nested_scope() {
        init_logger();
        let result = execute_datex_script_debug_with_result("[1, (2 + 3), 4]");
        let expected =
            datex_array![Integer::from(1), Integer::from(5), Integer::from(4)];

        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1_u8, 5_u8, 4_u8]));
        assert_structural_eq!(
            result,
            ValueContainer::from(vec![1_u8, 5_u8, 4_u8])
        );
    }

    #[test]
    fn test_boolean() {
        let result = execute_datex_script_debug_with_result("true");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("false");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn test_decimal() {
        let result = execute_datex_script_debug_with_result("1.5");
        assert_eq!(result, Decimal::from_string("1.5").into());
        assert_structural_eq!(result, ValueContainer::from(1.5));
    }

    #[test]
    fn test_decimal_and_integer() {
        let result = execute_datex_script_debug_with_result("-2341324.0");
        assert_eq!(result, Decimal::from_string("-2341324").into());
        assert_structural_eq!(result, ValueContainer::from(-2341324));
    }

    #[test]
    fn test_integer_2() {
        init_logger();
        let result = execute_datex_script_debug_with_result("2");
        assert_eq!(result, Integer::from(2).into());
        assert_ne!(result, 2_u8.into());
        assert_structural_eq!(result, ValueContainer::from(2_u8));
    }

    #[test]
    fn test_null() {
        let result = execute_datex_script_debug_with_result("null");
        assert_eq!(result, ValueContainer::from(CoreValue::Null));
        assert_eq!(result, CoreValue::Null.into());
        assert_structural_eq!(result, ValueContainer::from(CoreValue::Null));
    }

    #[test]
    fn test_tuple() {
        init_logger();
        let result = execute_datex_script_debug_with_result("(x: 1, 2, 42)");
        let tuple: CoreValue = result.clone().to_value().borrow().clone().inner;
        let tuple: Tuple = tuple.try_into().unwrap();

        // form and size
        assert_eq!(tuple.to_string(), "(\"x\": 1, 0: 2, 1: 42)");
        assert_eq!(tuple.size(), 3);

        // access by key
        assert_eq!(tuple.get(&"x".into()), Some(&Integer::from(1).into()));
        assert_eq!(
            tuple.get(&Integer::from(0_u32).into()),
            Some(&Integer::from(2).into())
        );
        assert_eq!(
            tuple.get(&Integer::from(1_u32).into()),
            Some(&Integer::from(42).into())
        );

        // structural equality checks
        let expected_se: Tuple = Tuple::from(vec![
            ("x".into(), 1.into()),
            (0.into(), 2.into()),
            (1.into(), 42.into()),
        ]);
        assert_structural_eq!(tuple, expected_se);

        // strict equality checks
        let expected_strict: Tuple = Tuple::from(vec![
            ("x".into(), Integer::from(1_u32).into()),
            (0_u32.into(), Integer::from(2_u32).into()),
            (1_u32.into(), Integer::from(42_u32).into()),
        ]);
        debug!("Expected tuple: {}", expected_strict);
        debug!("Tuple result: {}", tuple);
        // FIXME type information gets lost on compile
        // assert_eq!(result, expected.into());
    }
}
