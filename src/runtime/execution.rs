use super::stack::{Scope, ScopeStack};

use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::comparison_operation::ComparisonOperator;
use crate::ast::unary_operation::UnaryOperator;
use crate::compiler::compile_value;
use crate::compiler::error::CompilerError;
use crate::global::binary_codes::{InstructionCode, InternalSlot};
use crate::global::protocol_structures::instructions::*;
use crate::network::com_hub::ResponseError;
use crate::parser::body;
use crate::parser::body::DXBParserError;
use crate::runtime::RuntimeInternal;
use crate::runtime::execution_context::RemoteExecutionContext;
use crate::utils::buffers::append_u32;
use crate::values::core_value::CoreValue;
use crate::values::core_values::array::Array;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::object::Object;
use crate::values::core_values::tuple::Tuple;
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::error::IllegalTypeError;
use crate::values::pointer::PointerAddress;
use crate::values::traits::identity::Identity;
use crate::values::traits::structural_eq::StructuralEq;
use crate::values::traits::value_eq::ValueEq;
use crate::values::value::Value;
use crate::values::value_container::{ValueContainer, ValueError};
use crate::values::value_reference::ValueReference;
use datex_core::values::reference::Reference;
use log::info;
use num_enum::TryFromPrimitive;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct ExecutionInput<'a> {
    pub options: ExecutionOptions,
    pub dxb_body: &'a [u8],
    pub end_execution: bool,
    pub context: Rc<RefCell<RuntimeExecutionContext>>,
}

// TODO #229: do we want a DatexProgram input enum like this for execution?
// #[derive(Debug, Clone)]
// pub enum DatexProgram {
//     Dxb(Vec<u8>),
//     Script(String),
// }

// impl From<Vec<u8>> for DatexProgram {
//     fn from(dxb: Vec<u8>) -> Self {
//         DatexProgram::Dxb(dxb)
//     }
// }
// impl From<String> for DatexProgram {
//     fn from(script: String) -> Self {
//         DatexProgram::Script(script)
//     }
// }

impl Default for ExecutionInput<'_> {
    fn default() -> Self {
        Self {
            options: ExecutionOptions::default(),
            dxb_body: &[],
            context: Rc::new(RefCell::new(RuntimeExecutionContext::default())),
            end_execution: true,
        }
    }
}

impl<'a> ExecutionInput<'a> {
    pub fn new_with_dxb_and_options(
        dxb_body: &'a [u8],
        options: ExecutionOptions,
    ) -> Self {
        Self {
            options,
            dxb_body,
            context: Rc::new(RefCell::new(RuntimeExecutionContext::default())),
            end_execution: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeExecutionContext {
    index: usize,
    scope_stack: ScopeStack,
    slots: RefCell<HashMap<u32, Option<ValueContainer>>>,
    // if set to true, the execution loop will pop the current scope before continuing with the next instruction
    pop_next_scope: bool,
    runtime_internal: Option<Rc<RuntimeInternal>>,
}

impl RuntimeExecutionContext {
    pub fn new(runtime_internal: Rc<RuntimeInternal>) -> Self {
        Self {
            runtime_internal: Some(runtime_internal),
            ..Default::default()
        }
    }

    pub fn reset_index(&mut self) {
        self.index = 0;
    }

    pub fn runtime_internal(&self) -> &Option<Rc<RuntimeInternal>> {
        &self.runtime_internal
    }

    pub fn set_runtime_internal(
        &mut self,
        runtime_internal: Rc<RuntimeInternal>,
    ) {
        self.runtime_internal = Some(runtime_internal);
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

pub fn execute_dxb_sync(
    input: ExecutionInput,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let interrupt_provider = Rc::new(RefCell::new(None));
    let runtime_internal =
        input.context.borrow_mut().runtime_internal().clone();

    for output in execute_loop(input, interrupt_provider.clone()) {
        match output? {
            ExecutionStep::Return(result) => return Ok(result),
            ExecutionStep::ResolvePointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveOriginPointer(address) => {
                todo!(
                    "block origin is needed here to resolve the pointer address"
                )
            }
            ExecutionStep::ResolveInternalPointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(
                        get_internal_pointer_value(&runtime_internal, address)?,
                    ));
            }
            ExecutionStep::GetInternalSlot(slot) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_internal_slot_value(
                        &runtime_internal,
                        slot,
                    )?));
            }
            _ => return Err(ExecutionError::RequiresAsyncExecution),
        }
    }

    Err(ExecutionError::RequiresAsyncExecution)
}

pub async fn execute_dxb(
    input: ExecutionInput<'_>,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let interrupt_provider = Rc::new(RefCell::new(None));
    let runtime_internal =
        input.context.borrow_mut().runtime_internal().clone();

    for output in execute_loop(input, interrupt_provider.clone()) {
        match output? {
            ExecutionStep::Return(result) => return Ok(result),
            ExecutionStep::ResolvePointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveOriginPointer(address) => {
                todo!(
                    "block origin is needed here to resolve the pointer address"
                )
            }
            ExecutionStep::ResolveInternalPointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(
                        get_internal_pointer_value(&runtime_internal, address)?,
                    ));
            }
            ExecutionStep::RemoteExecution(receivers, body) => {
                if let Some(runtime) = &runtime_internal {
                    // assert that receivers is a single endpoint
                    // TODO #230: support advanced receivers
                    let receiver_endpoint = receivers
                        .to_value()
                        .borrow()
                        .cast_to_endpoint()
                        .unwrap();
                    let mut remote_execution_context =
                        RemoteExecutionContext::new(receiver_endpoint, true);
                    let res = RuntimeInternal::execute_remote(
                        runtime.clone(),
                        &mut remote_execution_context,
                        body,
                    )
                    .await?;
                    *interrupt_provider.borrow_mut() =
                        Some(InterruptProvider::Result(res));
                } else {
                    return Err(ExecutionError::RequiresRuntime);
                }
            }
            ExecutionStep::GetInternalSlot(slot) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_internal_slot_value(
                        &runtime_internal,
                        slot,
                    )?));
            }
            _ => todo!("#99 Undescribed by author."),
        }
    }

    unreachable!("Execution loop should always return a result");
}

fn get_internal_slot_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    slot: u32,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        // convert slot to InternalSlot enum
        let slot = InternalSlot::try_from_primitive(slot)
            .map_err(|_| ExecutionError::SlotNotAllocated(slot))?;
        let res = match slot {
            InternalSlot::ENDPOINT => {
                Some(ValueContainer::from(runtime.endpoint.clone()))
            }
        };
        Ok(res)
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

fn get_pointer_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    address: RawFullPointerAddress,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        let memory = runtime.memory.borrow();
        let resolved_address =
            memory.get_pointer_address_from_raw_full_address(address);
        // convert slot to InternalSlot enum
        Ok(memory
            .get_reference(&resolved_address)
            .map(|r| ValueContainer::Reference(r.clone())))
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

fn get_internal_pointer_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    address: RawInternalPointerAddress,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        // convert slot to InternalSlot enum
        Ok(runtime
            .memory
            .borrow()
            .get_reference(&PointerAddress::Internal(address.id))
            .map(|r| ValueContainer::Reference(r.clone())))
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidProgramError {
    InvalidScopeClose,
    InvalidKeyValuePair,
    // any unterminated sequence, e.g. missing key in key-value pair
    UnterminatedSequence,
    MissingRemoteExecutionReceiver,
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
            InvalidProgramError::MissingRemoteExecutionReceiver => {
                write!(f, "Missing remote execution receiver")
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
    RequiresAsyncExecution,
    RequiresRuntime,
    ResponseError(ResponseError),
    CompilerError(CompilerError),
    IllegalTypeError(IllegalTypeError),
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

impl From<IllegalTypeError> for ExecutionError {
    fn from(error: IllegalTypeError) -> Self {
        ExecutionError::IllegalTypeError(error)
    }
}

impl From<InvalidProgramError> for ExecutionError {
    fn from(error: InvalidProgramError) -> Self {
        ExecutionError::InvalidProgram(error)
    }
}

impl From<ResponseError> for ExecutionError {
    fn from(error: ResponseError) -> Self {
        ExecutionError::ResponseError(error)
    }
}

impl From<CompilerError> for ExecutionError {
    fn from(error: CompilerError) -> Self {
        ExecutionError::CompilerError(error)
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::CompilerError(err) => {
                write!(f, "Compiler error: {err}")
            }
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
            ExecutionError::RequiresAsyncExecution => {
                write!(f, "Program must be executed asynchronously")
            }
            ExecutionError::RequiresRuntime => {
                write!(f, "Execution requires a runtime to be set")
            }
            ExecutionError::ResponseError(err) => {
                write!(f, "Response error: {err}")
            }
            ExecutionError::IllegalTypeError(err) => {
                write!(f, "Illegal type: {err}")
            }
        }
    }
}

#[derive(Debug)]
pub enum ExecutionStep {
    InternalReturn(Option<ValueContainer>),
    Return(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveOriginPointer(RawOriginPointerAddress),
    ResolveInternalPointer(RawInternalPointerAddress),
    GetInternalSlot(u32),
    RemoteExecution(ValueContainer, Vec<u8>),
    Pause,
}

#[derive(Debug)]
pub enum InterruptProvider {
    Result(Option<ValueContainer>),
}

#[macro_export]
macro_rules! interrupt {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        $input.take().unwrap()
    }};
}

#[macro_export]
macro_rules! interrupt_with_result {
    ($input:expr, $arg:expr) => {{
        yield Ok($arg);
        let res = $input.take().unwrap();
        match res {
            InterruptProvider::Result(value) => value,
        }
    }};
}

#[macro_export]
macro_rules! yield_unwrap {
    ($e:expr) => {{
        let res = $e;
        if let Ok(res) = res {
            res
        } else {
            return yield Err(res.unwrap_err().into());
        }
    }};
}

pub fn execute_loop(
    input: ExecutionInput,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        let dxb_body = input.dxb_body;
        let end_execution = input.end_execution;
        let context = input.context;

        let instruction_iterator = body::iterate_instructions(dxb_body);

        for instruction in instruction_iterator {
            // TODO #100: use ? operator instead of yield_unwrap once supported in gen blocks
            let instruction = yield_unwrap!(instruction);
            if input.options.verbose {
                info!("[Exec]: {instruction}");
            }

            // get initial value from instruction
            let mut result_value = None;

            for output in get_result_value_from_instruction(
                context.clone(),
                instruction,
                interrupt_provider.clone(),
            ) {
                match yield_unwrap!(output) {
                    ExecutionStep::InternalReturn(result) => {
                        result_value = result;
                    }
                    step => {
                        *interrupt_provider.borrow_mut() =
                            Some(interrupt!(interrupt_provider, step));
                    }
                }
            }

            // 1. if value is Some, handle it
            // 2. while pop_next_scope is true: pop current scope and repeat
            loop {
                let mut context_mut = context.borrow_mut();
                context_mut.pop_next_scope = false;
                if let Some(value) = result_value {
                    let res = handle_value(&mut context_mut, value);
                    drop(context_mut);
                    yield_unwrap!(res);
                } else {
                    drop(context_mut);
                }

                let mut context_mut = context.borrow_mut();

                if context_mut.pop_next_scope {
                    let res = context_mut.scope_stack.pop();
                    drop(context_mut);
                    result_value = yield_unwrap!(res);
                } else {
                    break;
                }
            }
        }

        if end_execution {
            // cleanup...
            // TODO #101: check for other unclosed stacks
            // if we have an active key here, this is invalid and leads to an error
            // if context.scope_stack.get_active_key().is_some() {
            //     return Err(ExecutionError::InvalidProgram(
            //         InvalidProgramError::UnterminatedSequence,
            //     ));
            // }

            // removes the current active value from the scope stack
            let res = match context.borrow_mut().scope_stack.pop_active_value()
            {
                None => ExecutionStep::Return(None),
                Some(val) => ExecutionStep::Return(Some(val)),
            };
            yield Ok(res);
        } else {
            yield Ok(ExecutionStep::Pause);
        }
    }
}

#[inline]
fn get_result_value_from_instruction(
    context: Rc<RefCell<RuntimeExecutionContext>>,
    instruction: Instruction,
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        yield Ok(ExecutionStep::InternalReturn(match instruction {
            // boolean
            Instruction::True => Some(true.into()),
            Instruction::False => Some(false.into()),

            // integers
            Instruction::Int8(integer) => Some(Integer::from(integer.0).into()),
            Instruction::Int16(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::Int32(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::Int64(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::Int128(integer) => {
                Some(Integer::from(integer.0).into())
            }

            // unsigned integers
            Instruction::UInt8(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::UInt16(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::UInt32(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::UInt64(integer) => {
                Some(Integer::from(integer.0).into())
            }
            Instruction::UInt128(integer) => {
                Some(Integer::from(integer.0).into())
            }

            // big integers
            Instruction::BigInteger(IntegerData(integer)) => {
                Some(integer.into())
            }

            // specific floats
            Instruction::DecimalF32(Float32Data(f32)) => {
                Some(TypedDecimal::from(f32).into())
            }
            Instruction::DecimalF64(Float64Data(f64)) => {
                Some(TypedDecimal::from(f64).into())
            }

            // default decimals (big decimals)
            Instruction::DecimalAsInt16(FloatAsInt16Data(i16)) => {
                Some(Decimal::from(i16 as f32).into())
            }
            Instruction::DecimalAsInt32(FloatAsInt32Data(i32)) => {
                Some(Decimal::from(i32 as f32).into())
            }
            Instruction::Decimal(DecimalData(big_decimal)) => {
                Some(big_decimal.into())
            }

            // endpoint
            Instruction::Endpoint(endpoint) => Some(endpoint.into()),

            // null
            Instruction::Null => Some(Value::null().into()),

            // text
            Instruction::ShortText(ShortTextData(text)) => Some(text.into()),
            Instruction::Text(TextData(text)) => Some(text.into()),

            // binary operations
            Instruction::Add
            | Instruction::Subtract
            | Instruction::Multiply
            | Instruction::Divide
            | Instruction::Union => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::BinaryOperation {
                        operator: BinaryOperator::from(instruction),
                    },
                );
                None
            }

            // equality operations
            Instruction::Is
            | Instruction::Matches
            | Instruction::StructuralEqual
            | Instruction::Equal
            | Instruction::NotStructuralEqual
            | Instruction::NotEqual => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::ComparisonOperation {
                        operator: ComparisonOperator::from(instruction),
                    },
                );
                None
            }

            Instruction::ExecutionBlock(block) => {
                // build dxb

                let mut buffer = Vec::with_capacity(256);
                for (addr, local_slot) in
                    block.injected_slots.into_iter().enumerate()
                {
                    buffer.push(InstructionCode::ALLOCATE_SLOT as u8);
                    append_u32(&mut buffer, addr as u32);

                    if let Some(vc) = yield_unwrap!(
                        context.borrow().get_slot_value(local_slot).map_err(
                            |_| ExecutionError::SlotNotAllocated(local_slot),
                        )
                    ) {
                        buffer.extend_from_slice(&yield_unwrap!(
                            compile_value(&vc)
                        ));
                    } else {
                        return yield Err(ExecutionError::SlotNotInitialized(
                            local_slot,
                        ));
                    }
                }
                buffer.extend_from_slice(&block.body);

                let maybe_receivers =
                    context.borrow_mut().scope_stack.pop_active_value();

                if let Some(receivers) = maybe_receivers {
                    interrupt_with_result!(
                        interrupt_provider,
                        ExecutionStep::RemoteExecution(receivers, buffer)
                    )
                } else {
                    // should not happen, receivers must be set
                    yield Err(ExecutionError::InvalidProgram(
                        InvalidProgramError::MissingRemoteExecutionReceiver,
                    ));
                    None
                }
            }

            Instruction::CloseAndStore => {
                let _ = context.borrow_mut().scope_stack.pop_active_value();
                None
            }

            Instruction::ScopeStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::Default);
                None
            }

            Instruction::ArrayStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        Array::default().into(),
                    );
                None
            }

            Instruction::ObjectStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        Object::default().into(),
                    );
                None
            }

            Instruction::TupleStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        Tuple::default().into(),
                    );
                None
            }

            Instruction::KeyValueShortText(ShortTextData(key)) => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::KeyValuePair,
                        key.into(),
                    );
                None
            }

            Instruction::KeyValueDynamic => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::KeyValuePair);
                None
            }

            Instruction::ScopeEnd => {
                // pop scope and return value
                yield_unwrap!(context.borrow_mut().scope_stack.pop())
            }

            // slots
            Instruction::AllocateSlot(SlotAddress(address)) => {
                let mut context = context.borrow_mut();
                context.allocate_slot(address, None);
                context
                    .scope_stack
                    .create_scope(Scope::SlotAssignment { address });
                None
            }
            Instruction::GetSlot(SlotAddress(address)) => {
                // if address is >= 0xffffff00, resolve internal slot
                if address >= 0xffffff00 {
                    interrupt_with_result!(
                        interrupt_provider,
                        ExecutionStep::GetInternalSlot(address)
                    )
                }
                // else handle normal slot
                else {
                    let res = context.borrow_mut().get_slot_value(address);
                    // get value from slot
                    let slot_value = yield_unwrap!(res);
                    if slot_value.is_none() {
                        return yield Err(ExecutionError::SlotNotInitialized(
                            address,
                        ));
                    }
                    slot_value
                }
            }
            Instruction::SetSlot(SlotAddress(address)) => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::SlotAssignment { address });
                None
            }

            Instruction::GetRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolvePointer(address)
                )
            }

            Instruction::GetOriginRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolveOriginPointer(address)
                )
            }

            Instruction::GetInternalRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolveInternalPointer(address)
                )
            }

            Instruction::AddAssign(SlotAddress(address)) => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::AssignmentOperation {
                        address,
                        operator: AssignmentOperator::AddAssign,
                    },
                );
                None
            }

            Instruction::SubtractAssign(SlotAddress(address)) => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::AssignmentOperation {
                        address,
                        operator: AssignmentOperator::SubstractAssign,
                    },
                );
                None
            }

            // refs
            Instruction::CreateRef => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::CreateRef,
                    },
                );
                None
            }

            Instruction::CreateRefMut => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::CreateRefMut,
                    },
                );
                None
            }

            // remote execution
            Instruction::RemoteExecution => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::RemoteExecution);
                None
            }

            Instruction::DropSlot(SlotAddress(address)) => {
                // remove slot from slots
                let res = context.borrow_mut().drop_slot(address);
                yield_unwrap!(res);
                None
            }

            i => {
                return yield Err(ExecutionError::NotImplemented(
                    format!("Instruction {i}").to_string(),
                ));
            }
        }))
    }
}

/// Takes a produced value and handles it according to the current scope
fn handle_value(
    context: &mut RuntimeExecutionContext,
    value_container: ValueContainer,
) -> Result<(), ExecutionError> {
    let scope_container = context.scope_stack.get_current_scope_mut();

    let result_value = match &scope_container.scope {
        Scope::KeyValuePair => {
            let key = &scope_container.active_value;
            match key {
                // set key as active_value for key-value pair (for dynamic keys)
                None => Some(value_container),

                // set value for key-value pair
                Some(_) => {
                    let key = context.scope_stack.pop()?.unwrap();
                    match context.scope_stack.get_active_value_mut() {
                        Some(collector) => {
                            // handle active value collector
                            handle_key_value_pair(
                                collector,
                                key,
                                value_container,
                            )?;
                        }
                        None => unreachable!(
                            "Expected active value for key-value pair, but got None"
                        ),
                    }
                    None
                }
            }
        }

        Scope::SlotAssignment { address } => {
            // set value for slot
            let address = *address;
            context.set_slot_value(address, value_container.clone())?;
            Some(value_container)
        }

        Scope::AssignmentOperation { operator, address } => {
            let operator = *operator;
            let address = *address;
            let lhs = if let Ok(Some(val)) = context.get_slot_value(address) {
                val
            } else {
                return Err(ExecutionError::SlotNotInitialized(address));
            };
            let res =
                handle_assignment_operation(lhs, value_container, operator)?;
            context.set_slot_value(address, res.clone())?;
            Some(res)
        }

        Scope::UnaryOperation { operator } => {
            let operator = *operator;
            context.pop_next_scope = true;
            Some(handle_unary_operation(operator, value_container))
        }

        Scope::BinaryOperation { operator } => {
            let active_value = &scope_container.active_value;
            match active_value {
                Some(active_value_container) => {
                    let res = handle_binary_operation(
                        active_value_container,
                        value_container,
                        *operator,
                    );
                    if let Ok(val) = res {
                        // set val as active value
                        context.pop_next_scope = true;
                        Some(val)
                    } else {
                        // handle error
                        return Err(res.unwrap_err());
                    }
                }
                None => Some(value_container),
            }
        }

        Scope::ComparisonOperation { operator } => {
            let active_value = &scope_container.active_value;
            match active_value {
                Some(active_value_container) => {
                    let res = handle_comparison_operation(
                        active_value_container,
                        value_container,
                        *operator,
                    );
                    if let Ok(val) = res {
                        // set val as active value
                        context.pop_next_scope = true;
                        Some(val)
                    } else {
                        // handle error
                        return Err(res.unwrap_err());
                    }
                }
                None => Some(value_container),
            }
        }

        Scope::Collection => {
            let active_value = &mut scope_container.active_value;
            match active_value {
                Some(active_value_container) => {
                    // handle active value collector
                    handle_collector(active_value_container, value_container);
                    None
                }
                None => {
                    unreachable!(
                        "Expected active value for collection scope, but got None"
                    );
                }
            }
        }

        _ => Some(value_container),
    };

    if let Some(result_value) = result_value {
        context.scope_stack.set_active_value_container(result_value);
    }

    Ok(())
}

fn handle_collector(collector: &mut ValueContainer, value: ValueContainer) {
    match collector {
        ValueContainer::Value(Value {
            inner: CoreValue::Array(array),
            ..
        }) => {
            // append value to array
            array.push(value);
        }
        ValueContainer::Value(Value {
            inner: CoreValue::Tuple(tuple),
            ..
        }) => {
            // FIXME shall we consider a special case for tuples indexing?
            // automatic tuple keys are always default integer values
            let index = CoreValue::Integer(Integer::from(tuple.next_int_key()));
            tuple.set(index, value);
        }
        _ => {
            unreachable!(
                "Expected active value in array scope to be an array, but got: {}",
                collector
            );
        }
    }
}

fn handle_key_value_pair(
    active_container: &mut ValueContainer,
    key: ValueContainer,
    value: ValueContainer,
) -> Result<(), ExecutionError> {
    // insert key value pair into active object/tuple
    match active_container {
        // object
        ValueContainer::Value(Value {
            inner: CoreValue::Object(object),
            ..
        }) => {
            // make sure key is a string
            match key {
                ValueContainer::Value(Value {
                    inner: CoreValue::Text(key_str),
                    ..
                }) => {
                    object.set(&key_str.0, value);
                }
                _ => {
                    return Err(ExecutionError::InvalidProgram(
                        InvalidProgramError::InvalidKeyValuePair,
                    ));
                }
            }
        }
        // tuple
        ValueContainer::Value(Value {
            inner: CoreValue::Tuple(tuple),
            ..
        }) => {
            // set key-value pair in tuple
            tuple.set(key, value);
        }
        _ => {
            unreachable!(
                "Expected active value object or tuple to collect key value pairs, but got: {}",
                active_container
            );
        }
    }

    Ok(())
}

fn handle_unary_operation(
    operator: UnaryOperator,
    value_container: ValueContainer,
) -> ValueContainer {
    match operator {
        UnaryOperator::CreateRef => {
            ValueContainer::Reference(Reference::from(value_container))
        }
        UnaryOperator::CreateRefMut => {
            todo!("mutable references are not yet supported");
            // ValueContainer::Reference(Reference::try_mut_from(value_container).expect("Could not create mutable reference"))
        }
        _ => todo!("#102 Unary instruction not implemented: {operator:?}"),
    }
}

fn handle_comparison_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: ComparisonOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        ComparisonOperator::StructuralEqual => {
            let val = active_value_container.structural_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Equal => {
            let val = active_value_container.value_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::NotStructuralEqual => {
            let val = !active_value_container.structural_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::NotEqual => {
            let val = !active_value_container.value_eq(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Is => {
            // TODO #103 we should throw a runtime error when one of lhs or rhs is a value
            // instead of a ref. Identity checks using the is operator shall be only allowed
            // for references.
            // @benstre: or keep as always false ? - maybe a compiler check would be better
            let val = active_value_container.identical(&value_container);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Matches => {
            let v_type = value_container.actual_type(); // Type::try_from(value_container)?;
            let val = v_type.value_matches(active_value_container);
            Ok(ValueContainer::from(val))
        }
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

fn handle_assignment_operation(
    lhs: ValueContainer,
    rhs: ValueContainer,
    operator: AssignmentOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        AssignmentOperator::AddAssign => Ok((lhs + rhs)?),
        AssignmentOperator::SubstractAssign => Ok((lhs - rhs)?),
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

fn handle_binary_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: BinaryOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        BinaryOperator::Add => Ok((active_value_container + &value_container)?),
        BinaryOperator::Subtract => {
            Ok((active_value_container - &value_container)?)
        }
        BinaryOperator::Union => {
            // if right is already a union, prepend left value to options

            todo!("implement union operation");
            // if let ValueContainer::Value(Value {
            //     inner: CoreValue::Union(right_union),
            //     ..
            // }) = value_container
            // {
            //     // TODO: no clone here
            //     let mut new_options = right_union.options.clone();
            //     new_options.insert(0, active_value_container.clone());
            //     Ok(ValueContainer::from(Union::new(new_options)))
            // }
            // // else create new union with both values
            // else {
            //     Ok(ValueContainer::from(Union::new(vec![
            //         active_value_container.clone(),
            //         value_container,
            //     ])))
            // }
        }
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use std::vec;

    use log::debug;

    use super::*;
    use crate::compiler::{CompileOptions, compile_script};
    use crate::global::binary_codes::InstructionCode;
    use crate::logger::init_logger_debug;
    use crate::values::traits::structural_eq::StructuralEq;
    use crate::{assert_structural_eq, assert_value_eq, datex_array};

    fn execute_datex_script_debug(
        datex_script: &str,
    ) -> Option<ValueContainer> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context = ExecutionInput::new_with_dxb_and_options(
            &dxb,
            ExecutionOptions { verbose: true },
        );
        execute_dxb_sync(context).unwrap_or_else(|err| {
            panic!("Execution failed: {err}");
        })
    }

    fn execute_datex_script_debug_with_error(
        datex_script: &str,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        let (dxb, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
        let context = ExecutionInput::new_with_dxb_and_options(
            &dxb,
            ExecutionOptions { verbose: true },
        );
        execute_dxb_sync(context)
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
        execute_dxb_sync(context)
    }

    #[test]
    fn empty_script() {
        assert_eq!(execute_datex_script_debug(""), None);
    }

    #[test]
    fn empty_script_semicolon() {
        assert_eq!(execute_datex_script_debug(";;;"), None);
    }

    #[test]
    fn single_value() {
        assert_eq!(
            execute_datex_script_debug_with_result("42"),
            Integer::from(42i8).into()
        );
    }

    #[test]
    fn single_value_semicolon() {
        assert_eq!(execute_datex_script_debug("42;"), None)
    }

    #[test]
    fn is() {
        let result = execute_datex_script_debug_with_result("1 is 1");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn equality() {
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
    fn single_value_scope() {
        let result = execute_datex_script_debug_with_result("(42)");
        assert_eq!(result, Integer::from(42i8).into());
        assert_structural_eq!(result, ValueContainer::from(42_u128));
    }

    #[test]
    fn add() {
        let result = execute_datex_script_debug_with_result("1 + 2");
        assert_eq!(result, Integer::from(3i8).into());
        assert_structural_eq!(result, ValueContainer::from(3i8));
    }

    #[test]
    fn nested_scope() {
        let result = execute_datex_script_debug_with_result("1 + (2 + 3)");
        assert_eq!(result, Integer::from(6i8).into());
    }

    #[test]
    fn invalid_scope_close() {
        let result = execute_dxb_debug(&[
            InstructionCode::SCOPE_START.into(),
            InstructionCode::SCOPE_END.into(),
            InstructionCode::SCOPE_END.into(),
        ]);
        assert!(matches!(
            result,
            Err(ExecutionError::InvalidProgram(
                InvalidProgramError::InvalidScopeClose
            ))
        ));
    }

    #[test]
    fn empty_array() {
        let result = execute_datex_script_debug_with_result("[]");
        let array: Array = result.to_value().borrow().cast_to_array().unwrap();
        assert_eq!(array.len(), 0);
        assert_eq!(result, Vec::<ValueContainer>::new().into());
        assert_eq!(result, ValueContainer::from(Vec::<ValueContainer>::new()));
    }

    #[test]
    fn array() {
        let result = execute_datex_script_debug_with_result("[1, 2, 3]");
        let array: Array = result.to_value().borrow().cast_to_array().unwrap();
        let expected = datex_array![
            Integer::from(1i8),
            Integer::from(2i8),
            Integer::from(3i8)
        ];
        assert_eq!(array.len(), 3);
        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1, 2, 3]));
        assert_structural_eq!(result, ValueContainer::from(vec![1, 2, 3]));
    }

    #[test]
    fn array_with_nested_scope() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("[1, (2 + 3), 4]");
        let expected = datex_array![
            Integer::from(1i8),
            Integer::from(5i8),
            Integer::from(4i8)
        ];

        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1_u8, 5_u8, 4_u8]));
        assert_structural_eq!(
            result,
            ValueContainer::from(vec![1_u8, 5_u8, 4_u8])
        );
    }

    #[test]
    fn boolean() {
        let result = execute_datex_script_debug_with_result("true");
        assert_eq!(result, true.into());
        assert_structural_eq!(result, ValueContainer::from(true));

        let result = execute_datex_script_debug_with_result("false");
        assert_eq!(result, false.into());
        assert_structural_eq!(result, ValueContainer::from(false));
    }

    #[test]
    fn decimal() {
        let result = execute_datex_script_debug_with_result("1.5");
        assert_eq!(result, Decimal::from_string("1.5").unwrap().into());
        assert_structural_eq!(result, ValueContainer::from(1.5));
    }

    #[test]
    fn decimal_and_integer() {
        let result = execute_datex_script_debug_with_result("-2341324.0");
        assert_eq!(result, Decimal::from_string("-2341324").unwrap().into());
        assert!(!result.structural_eq(&ValueContainer::from(-2341324)));
    }

    #[test]
    fn integer() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("2");
        assert_eq!(result, Integer::from(2).into());
        assert_ne!(result, 2_u8.into());
        assert_structural_eq!(result, ValueContainer::from(2_i8));
    }

    // FIXME these shall produce TypedInteger values, not Integer
    // but this will only work once the compiler supports the type system
    #[test]
    fn typed_integer() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("-2i16");
        assert_eq!(result, Integer::from(-2i16).into());
        assert_structural_eq!(result, ValueContainer::from(-2_i16));

        let result = execute_datex_script_debug_with_result("2i32");
        assert_eq!(result, Integer::from(2i32).into());
        assert_structural_eq!(result, ValueContainer::from(2_i32));

        let result = execute_datex_script_debug_with_result("-2i64");
        assert_eq!(result, Integer::from(-2i64).into());
        assert_structural_eq!(result, ValueContainer::from(-2_i64));

        let result = execute_datex_script_debug_with_result("2i128");
        assert_eq!(result, Integer::from(2i128).into());
        assert_structural_eq!(result, ValueContainer::from(2_i128));

        let result = execute_datex_script_debug_with_result("2u8");
        assert_eq!(result, Integer::from(2_u8).into());
        assert_structural_eq!(result, ValueContainer::from(2_u8));

        let result = execute_datex_script_debug_with_result("2u16");
        assert_eq!(result, Integer::from(2_u16).into());
        assert_structural_eq!(result, ValueContainer::from(2_u16));

        let result = execute_datex_script_debug_with_result("2u32");
        assert_eq!(result, Integer::from(2_u32).into());
        assert_structural_eq!(result, ValueContainer::from(2_u32));

        let result = execute_datex_script_debug_with_result("2u64");
        assert_eq!(result, Integer::from(2_u64).into());
        assert_structural_eq!(result, ValueContainer::from(2_u64));

        let result = execute_datex_script_debug_with_result("2u128");
        assert_eq!(result, Integer::from(2_u128).into());
        assert_structural_eq!(result, ValueContainer::from(2_u128));

        let result = execute_datex_script_debug_with_result("2big");
        assert_eq!(result, Integer::from(2).into());
        assert_structural_eq!(result, ValueContainer::from(2));
    }

    #[test]
    fn null() {
        let result = execute_datex_script_debug_with_result("null");
        assert_eq!(result, ValueContainer::from(CoreValue::Null));
        assert_eq!(result, CoreValue::Null.into());
        assert_structural_eq!(result, ValueContainer::from(CoreValue::Null));
    }

    #[test]
    fn tuple() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("(x: 1, 2, 42)");
        let tuple: CoreValue = result.clone().to_value().borrow().clone().inner;
        let tuple: Tuple = tuple.try_into().unwrap();

        // form and size
        assert_eq!(tuple.to_string(), "(\"x\": 1, 0: 2, 1: 42)");
        assert_eq!(tuple.size(), 3);

        info!("Tuple: {:?}", tuple);

        // access by key
        assert_eq!(tuple.get(&"x".into()), Some(&Integer::from(1i8).into()));
        assert_eq!(
            tuple.get(&Integer::from(0).into()),
            Some(&Integer::from(2i8).into())
        );
        assert_eq!(
            tuple.get(&Integer::from(1).into()),
            Some(&Integer::from(42i8).into())
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
            (0.into(), Integer::from(2_u32).into()),
            (1.into(), Integer::from(42_u32).into()),
        ]);
        debug!("Expected tuple: {expected_strict}");
        debug!("Tuple result: {tuple}");
        // FIXME #104 type information gets lost on compile
        // assert_eq!(result, expected.into());
    }

    #[test]
    fn val_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("const x = 42; x");
        assert_eq!(result, Integer::from(42i8).into());
    }

    #[test]
    fn val_assignment_with_addition() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = 1 + 2; x");
        assert_eq!(result, Integer::from(3i8).into());
    }

    #[test]
    fn val_assignment_inside_scope() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("[const x = 42, 2, x]");
        let expected = datex_array![
            Integer::from(42i8),
            Integer::from(2i8),
            Integer::from(42i8)
        ];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn ref_assignment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &mut 42; x");
        assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(42i8)));
    }

    #[test]
    fn ref_add_assignment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &mut 42; x += 1");
        assert_value_eq!(result, ValueContainer::from(Integer::from(43i8)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; x += 1; x",
        );

        // FIXME due to addition the resulting value container of the slot
        // is no longer a reference but a value what is incorrect.
        // assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(43i8)));
    }

    #[test]
    fn ref_sub_assignment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &mut 42; x -= 1");
        assert_value_eq!(result, ValueContainer::from(Integer::from(41i8)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; x -= 1; x",
        );

        // FIXME due to addition the resulting value container of the slot
        // is no longer a reference but a value what is incorrect.
        // assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(41i8)));
    }

    #[test]
    fn endpoint_slot() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_error("#endpoint");
        assert_matches!(result.unwrap_err(), ExecutionError::RequiresRuntime);
    }

    #[test]
    fn shebang() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("#!datex\n42");
        assert_eq!(result, Integer::from(42i8).into());
    }

    #[test]
    fn single_line_comment() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("// this is a comment\n42");
        assert_eq!(result, Integer::from(42i8).into());

        let result = execute_datex_script_debug_with_result(
            "// this is a comment\n// another comment\n42",
        );
        assert_eq!(result, Integer::from(42i8).into());
    }

    #[test]
    fn multi_line_comment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "/* this is a comment */\n42",
        );
        assert_eq!(result, Integer::from(42i8).into());

        let result = execute_datex_script_debug_with_result(
            "/* this is a comment\n   with multiple lines */\n42",
        );
        assert_eq!(result, Integer::from(42i8).into());

        let result = execute_datex_script_debug_with_result("[1, /* 2, */ 3]");
        let expected = datex_array![Integer::from(1i8), Integer::from(3i8)];
        assert_eq!(result, expected.into());
    }
}
