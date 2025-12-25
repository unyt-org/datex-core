use super::stack::{Scope, ScopeStack};

use crate::global::operators::assignment::AssignmentOperator;

use crate::collections::HashMap;
use crate::core_compiler::value_compiler::compile_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::global::operators::BinaryOperator;
use crate::global::operators::ComparisonOperator;
use crate::global::operators::binary::{
    ArithmeticOperator, BitwiseOperator, LogicalOperator,
};
use crate::global::operators::{
    ArithmeticUnaryOperator, BitwiseUnaryOperator, LogicalUnaryOperator,
    ReferenceUnaryOperator, UnaryOperator,
};
use crate::global::protocol_structures::instructions::*;
use crate::global::slots::InternalSlot;
use crate::libs::core::{CoreLibPointerId, get_core_lib_type_reference};
use crate::network::com_hub::ResponseError;
use crate::parser::body;
use crate::parser::body::DXBParserError;
use crate::references::reference::Reference;
use crate::references::reference::{AssignmentError, ReferenceCreationError};
use crate::runtime::RuntimeInternal;
use crate::runtime::execution_context::RemoteExecutionContext;
use crate::stdlib::format;
use crate::stdlib::rc::Rc;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::traits::apply::Apply;
use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::types::error::IllegalTypeError;
use crate::types::type_container::TypeContainer;
use crate::utils::buffers::append_u32;
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::core_values::range::Range;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::{ValueContainer, ValueError};
use core::cell::RefCell;
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::unimplemented;
use core::unreachable;
use core::writeln;
use itertools::Itertools;
use log::info;
use num_enum::TryFromPrimitive;

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

pub struct MemoryDump {
    pub slots: Vec<(u32, Option<ValueContainer>)>,
}

#[cfg(feature = "compiler")]
impl Display for MemoryDump {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (address, value) in &self.slots {
            match value {
                Some(vc) => {
                    let decompiled = crate::decompiler::decompile_value(
                        vc,
                        crate::decompiler::DecompileOptions::colorized(),
                    );
                    writeln!(f, "#{address}: {decompiled}")?
                }
                None => writeln!(f, "#{address}: <uninitialized>")?,
            }
        }
        if self.slots.is_empty() {
            writeln!(f, "<no slots allocated>")?;
        }
        Ok(())
    }
}

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

    /// Returns a memory dump of the current slots and their values.
    pub fn memory_dump(&self) -> MemoryDump {
        MemoryDump {
            slots: self
                .slots
                .borrow()
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .sorted_by_key(|(k, _)| *k)
                .collect(),
        }
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
            ExecutionStep::ResolveLocalPointer(address) => {
                // TODO #401: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_local_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveInternalPointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(
                        get_internal_pointer_value(address)?,
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
            ExecutionStep::ResolveLocalPointer(address) => {
                // TODO #402: in the future, local pointer addresses should be relative to the block sender, not the local runtime
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(get_local_pointer_value(
                        &runtime_internal,
                        address,
                    )?));
            }
            ExecutionStep::ResolveInternalPointer(address) => {
                *interrupt_provider.borrow_mut() =
                    Some(InterruptProvider::Result(
                        get_internal_pointer_value(address)?,
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
            _ => core::todo!("#99 Undescribed by author."),
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
    address: RawInternalPointerAddress,
) -> Result<Option<ValueContainer>, ExecutionError> {
    let core_lib_id =
        CoreLibPointerId::try_from(&PointerAddress::Internal(address.id));
    core_lib_id
        .map_err(|_| ExecutionError::ReferenceNotFound)
        .map(|id| {
            Some(ValueContainer::Reference(Reference::TypeReference(
                get_core_lib_type_reference(id),
            )))
        })
}

fn get_local_pointer_value(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    address: RawLocalPointerAddress,
) -> Result<Option<ValueContainer>, ExecutionError> {
    if let Some(runtime) = &runtime_internal {
        // convert slot to InternalSlot enum
        Ok(runtime
            .memory
            .borrow()
            .get_reference(&PointerAddress::Local(address.id))
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvalidProgramError::InvalidScopeClose => {
                core::write!(f, "Invalid scope close")
            }
            InvalidProgramError::InvalidKeyValuePair => {
                core::write!(f, "Invalid key-value pair")
            }
            InvalidProgramError::UnterminatedSequence => {
                core::write!(f, "Unterminated sequence")
            }
            InvalidProgramError::MissingRemoteExecutionReceiver => {
                core::write!(f, "Missing remote execution receiver")
            }
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    DXBParserError(DXBParserError),
    ValueError(ValueError),
    InvalidProgram(InvalidProgramError),
    Unknown,
    NotImplemented(String),
    SlotNotAllocated(u32),
    SlotNotInitialized(u32),
    RequiresAsyncExecution,
    RequiresRuntime,
    ResponseError(ResponseError),
    IllegalTypeError(IllegalTypeError),
    ReferenceNotFound,
    DerefOfNonReference,
    InvalidTypeCast,
    AssignmentError(AssignmentError),
    ReferenceFromValueContainerError(ReferenceCreationError),
}
impl From<ReferenceCreationError> for ExecutionError {
    fn from(error: ReferenceCreationError) -> Self {
        ExecutionError::ReferenceFromValueContainerError(error)
    }
}

impl From<DXBParserError> for ExecutionError {
    fn from(error: DXBParserError) -> Self {
        ExecutionError::DXBParserError(error)
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

impl From<AssignmentError> for ExecutionError {
    fn from(error: AssignmentError) -> Self {
        ExecutionError::AssignmentError(error)
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExecutionError::ReferenceFromValueContainerError(err) => {
                core::write!(f, "Reference from value container error: {err}")
            }
            ExecutionError::ReferenceNotFound => {
                core::write!(f, "Reference not found")
            }
            ExecutionError::DXBParserError(err) => {
                core::write!(f, "Parser error: {err}")
            }
            ExecutionError::Unknown => {
                core::write!(f, "Unknown execution error")
            }
            ExecutionError::ValueError(err) => {
                core::write!(f, "Value error: {err}")
            }
            ExecutionError::InvalidProgram(err) => {
                core::write!(f, "Invalid program error: {err}")
            }
            ExecutionError::NotImplemented(msg) => {
                core::write!(f, "Not implemented: {msg}")
            }
            ExecutionError::SlotNotAllocated(address) => {
                core::write!(
                    f,
                    "Tried to access unallocated slot at address {address}"
                )
            }
            ExecutionError::SlotNotInitialized(address) => {
                core::write!(
                    f,
                    "Tried to access uninitialized slot at address {address}"
                )
            }
            ExecutionError::RequiresAsyncExecution => {
                core::write!(f, "Program must be executed asynchronously")
            }
            ExecutionError::RequiresRuntime => {
                core::write!(f, "Execution requires a runtime to be set")
            }
            ExecutionError::ResponseError(err) => {
                core::write!(f, "Response error: {err}")
            }
            ExecutionError::IllegalTypeError(err) => {
                core::write!(f, "Illegal type: {err}")
            }
            ExecutionError::DerefOfNonReference => {
                core::write!(f, "Tried to dereference a non-reference value")
            }
            ExecutionError::AssignmentError(err) => {
                core::write!(f, "Assignment error: {err}")
            }
            ExecutionError::InvalidTypeCast => {
                core::write!(f, "Invalid type cast")
            }
        }
    }
}

#[derive(Debug)]
pub enum ExecutionStep {
    InternalReturn(Option<ValueContainer>),
    InternalTypeReturn(Option<TypeContainer>),
    Return(Option<ValueContainer>),
    ResolvePointer(RawFullPointerAddress),
    ResolveLocalPointer(RawLocalPointerAddress),
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
                    ExecutionStep::InternalTypeReturn(result) => {
                        result_value = result.map(ValueContainer::from);
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

            Instruction::Range(range) => {
                Some(Range::new(range.start.into(), range.end.into()).into())
            }

            // text
            Instruction::ShortText(ShortTextData(text)) => Some(text.into()),
            Instruction::Text(TextData(text)) => Some(text.into()),

            // binary operations
            Instruction::Add
            | Instruction::Subtract
            | Instruction::Multiply
            | Instruction::Divide => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::BinaryOperation {
                        operator: BinaryOperator::from(instruction),
                    },
                );
                None
            }

            // unary operations
            Instruction::UnaryPlus => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Plus,
                        ),
                    },
                );
                None
            }
            Instruction::UnaryMinus => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Minus,
                        ),
                    },
                );
                None
            }
            Instruction::BitwiseNot => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Bitwise(
                            BitwiseUnaryOperator::Negation,
                        ),
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
                        buffer.extend_from_slice(&compile_value_container(&vc));
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

            Instruction::Apply(ApplyData { arg_count }) => {
                context.borrow_mut().scope_stack.create_scope(Scope::Apply {
                    arg_count,
                    args: vec![],
                });
                None
            }

            Instruction::ScopeStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope(Scope::Default);
                None
            }

            Instruction::ListStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        List::default().into(),
                    );
                None
            }

            Instruction::MapStart => {
                context
                    .borrow_mut()
                    .scope_stack
                    .create_scope_with_active_value(
                        Scope::Collection,
                        Map::default().into(),
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

            Instruction::AssignToReference(operator) => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::AssignToReference {
                        reference: None,
                        operator,
                    },
                );
                None
            }

            Instruction::Deref => {
                context.borrow_mut().scope_stack.create_scope(Scope::Deref);
                None
            }

            Instruction::GetRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolvePointer(address)
                )
            }

            Instruction::GetLocalRef(address) => {
                interrupt_with_result!(
                    interrupt_provider,
                    ExecutionStep::ResolveLocalPointer(address)
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
                        operator: AssignmentOperator::SubtractAssign,
                    },
                );
                None
            }

            // refs
            Instruction::CreateRef => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Reference(
                            ReferenceUnaryOperator::CreateRef,
                        ),
                    },
                );
                None
            }

            Instruction::CreateRefMut => {
                context.borrow_mut().scope_stack.create_scope(
                    Scope::UnaryOperation {
                        operator: UnaryOperator::Reference(
                            ReferenceUnaryOperator::CreateRefMut,
                        ),
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

            Instruction::TypeInstructions(instructions) => {
                for output in
                    iterate_type_instructions(interrupt_provider, instructions)
                {
                    // TODO #403: handle type here
                    yield output;
                }
                return;
            }

            // type(...)
            Instruction::TypeExpression(instructions) => {
                for output in
                    iterate_type_instructions(interrupt_provider, instructions)
                {
                    yield output;
                }
                return;
            }

            i => {
                return yield Err(ExecutionError::NotImplemented(
                    format!("Instruction {i}").to_string(),
                ));
            }
        }))
    }
}

fn iterate_type_instructions(
    interrupt_provider: Rc<RefCell<Option<InterruptProvider>>>,
    instructions: Vec<TypeInstruction>,
) -> impl Iterator<Item = Result<ExecutionStep, ExecutionError>> {
    gen move {
        for instruction in instructions {
            match instruction {
                // TODO #404: Implement type instructions iteration
                TypeInstruction::ListStart => {
                    interrupt_with_result!(
                        interrupt_provider,
                        ExecutionStep::Pause
                    );
                }
                TypeInstruction::LiteralInteger(integer) => {
                    yield Ok(ExecutionStep::InternalTypeReturn(Some(
                        TypeContainer::Type(Type::structural(integer.0)),
                    )));
                }
                _ => core::todo!("#405 Undescribed by author."),
            }
        }
    }
}

/// Takes a produced value and handles it according to the current scope
fn handle_value(
    context: &mut RuntimeExecutionContext,
    value_container: ValueContainer,
) -> Result<(), ExecutionError> {
    let scope_container = context.scope_stack.get_current_scope_mut();

    let result_value = match &mut scope_container.scope {
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

        Scope::Deref => {
            // set value for slot
            if let ValueContainer::Reference(reference) = value_container {
                Some(reference.value_container())
            } else {
                return Err(ExecutionError::DerefOfNonReference);
            }
        }

        Scope::AssignToReference {
            operator,
            reference,
        } => {
            if (reference.is_none()) {
                // set value for slot
                if let ValueContainer::Reference(new_reference) =
                    value_container
                {
                    reference.replace(new_reference);
                    None
                } else {
                    return Err(ExecutionError::DerefOfNonReference);
                }
            } else {
                let operator = *operator;
                let reference = reference.as_ref().unwrap();
                let lhs = reference.value_container();
                let res = handle_assignment_operation(
                    lhs,
                    value_container,
                    operator,
                )?;
                reference.set_value_container(res)?;
                Some(ValueContainer::Reference(reference.clone()))
            }
        }

        Scope::Apply { args, arg_count } => {
            // collect callee as active value if not set yet and we have args to collect
            if scope_container.active_value.is_none() {
                // directly apply if no args to collect
                if *arg_count == 0 {
                    context.pop_next_scope = true;
                    handle_apply(&value_container, args)?
                }
                // set callee as active value
                else {
                    Some(value_container)
                }
            } else {
                let callee = scope_container.active_value.as_ref().unwrap();
                // callee already exists, collect args
                args.push(value_container);

                // all args collected, apply function
                if args.len() == *arg_count as usize {
                    context.pop_next_scope = true;
                    handle_apply(callee, args)?
                } else {
                    Some(callee.clone())
                }
            }
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
            let result = handle_unary_operation(operator, value_container);
            if let Ok(val) = result {
                Some(val)
            } else {
                // handle error
                return Err(result.unwrap_err());
            }
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

fn handle_apply(
    callee: &ValueContainer,
    args: &[ValueContainer],
) -> Result<Option<ValueContainer>, ExecutionError> {
    // callee is guaranteed to be Some here
    // apply_single if one arg, apply otherwise
    Ok(if args.len() == 1 {
        callee.apply_single(&args[0])?
    } else {
        callee.apply(args)?
    })
}

fn handle_collector(collector: &mut ValueContainer, value: ValueContainer) {
    match collector {
        ValueContainer::Value(Value {
            inner: CoreValue::List(list),
            ..
        }) => {
            // append value to list
            list.push(value);
        }
        ValueContainer::Value(Value {
            inner: CoreValue::Map(map),
            ..
        }) => {
            // TODO #406: Implement map collector for optimized structural maps
            core::panic!("append {:?}", value);
        }
        _ => {
            unreachable!("Unsupported collector for collection scope");
        }
    }
}

fn handle_key_value_pair(
    active_container: &mut ValueContainer,
    key: ValueContainer,
    value: ValueContainer,
) -> Result<(), ExecutionError> {
    // insert key value pair into active map
    match active_container {
        // Map
        ValueContainer::Value(Value {
            inner: CoreValue::Map(map),
            ..
        }) => {
            // make sure key is a string
            map.try_set(key, value)
                .expect("Failed to set key-value pair in map");
        }
        _ => {
            unreachable!(
                "Expected active value that can collect key value pairs, but got: {}",
                active_container
            );
        }
    }

    Ok(())
}

fn handle_unary_reference_operation(
    operator: ReferenceUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    Ok(match operator {
        ReferenceUnaryOperator::CreateRef => {
            ValueContainer::Reference(Reference::from(value_container))
        }
        ReferenceUnaryOperator::CreateRefMut => {
            ValueContainer::Reference(Reference::try_mut_from(value_container)?)
        }
        ReferenceUnaryOperator::Deref => {
            if let ValueContainer::Reference(reference) = value_container {
                reference.value_container()
            } else {
                return Err(ExecutionError::DerefOfNonReference);
            }
        }
    })
}
fn handle_unary_logical_operation(
    operator: LogicalUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    unimplemented!(
        "Logical unary operations are not implemented yet: {operator:?}"
    )
}
fn handle_unary_arithmetic_operation(
    operator: ArithmeticUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        ArithmeticUnaryOperator::Minus => Ok((-value_container)?),
        ArithmeticUnaryOperator::Plus => Ok(value_container),
        _ => unimplemented!(
            "Arithmetic unary operations are not implemented yet: {operator:?}"
        ),
    }
}

fn handle_unary_operation(
    operator: UnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        UnaryOperator::Reference(reference) => {
            handle_unary_reference_operation(reference, value_container)
        }
        UnaryOperator::Logical(logical) => {
            handle_unary_logical_operation(logical, value_container)
        }
        UnaryOperator::Arithmetic(arithmetic) => {
            handle_unary_arithmetic_operation(arithmetic, value_container)
        }
        _ => {
            core::todo!("#102 Unary instruction not implemented: {operator:?}")
        }
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
            // TODO #407: Fix matches, rhs will always be a type, so actual_type() call is wrong
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
        AssignmentOperator::SubtractAssign => Ok((lhs - rhs)?),
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

fn handle_arithmetic_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: ArithmeticOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        ArithmeticOperator::Add => {
            Ok((active_value_container + &value_container)?)
        }
        ArithmeticOperator::Subtract => {
            Ok((active_value_container - &value_container)?)
        }
        // ArithmeticOperator::Multiply => {
        //     Ok((active_value_container * &value_container)?)
        // }
        // ArithmeticOperator::Divide => {
        //     Ok((active_value_container / &value_container)?)
        // }
        _ => {
            core::todo!(
                "#408 Implement arithmetic operation for {:?}",
                operator
            );
        }
    }
}

fn handle_bitwise_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: BitwiseOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    {
        core::todo!("#409 Implement bitwise operation for {:?}", operator);
    }
}

fn handle_logical_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: LogicalOperator,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    {
        core::todo!("#410 Implement logical operation for {:?}", operator);
    }
}

fn handle_binary_operation(
    active_value_container: &ValueContainer,
    value_container: ValueContainer,
    operator: BinaryOperator,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        BinaryOperator::Arithmetic(arith_op) => handle_arithmetic_operation(
            active_value_container,
            value_container,
            arith_op,
        ),
        BinaryOperator::Bitwise(bitwise_op) => handle_bitwise_operation(
            active_value_container,
            value_container,
            bitwise_op,
        ),
        BinaryOperator::Logical(logical_op) => handle_logical_operation(
            active_value_container,
            value_container,
            logical_op,
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::stdlib::assert_matches::assert_matches;
    use crate::stdlib::vec;

    use super::*;
    use crate::compiler::{CompileOptions, compile_script};
    use crate::global::instruction_codes::InstructionCode;
    use crate::logger::init_logger_debug;
    use crate::traits::structural_eq::StructuralEq;
    use crate::{assert_structural_eq, assert_value_eq, datex_list};
    use datex_core::values::core_values::integer::typed_integer::TypedInteger;
    use log::debug;

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
            core::panic!("Execution failed: {err}");
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
        assert!(core::matches!(
            result,
            Err(ExecutionError::InvalidProgram(
                InvalidProgramError::InvalidScopeClose
            ))
        ));
    }

    #[test]
    fn empty_list() {
        let result = execute_datex_script_debug_with_result("[]");
        let list: List = result.to_value().borrow().cast_to_list().unwrap();
        assert_eq!(list.len(), 0);
        assert_eq!(result, Vec::<ValueContainer>::new().into());
        assert_eq!(result, ValueContainer::from(Vec::<ValueContainer>::new()));
    }

    #[test]
    fn list() {
        let result = execute_datex_script_debug_with_result("[1, 2, 3]");
        let list: List = result.to_value().borrow().cast_to_list().unwrap();
        let expected = datex_list![
            Integer::from(1i8),
            Integer::from(2i8),
            Integer::from(3i8)
        ];
        assert_eq!(list.len(), 3);
        assert_eq!(result, expected.into());
        assert_ne!(result, ValueContainer::from(vec![1, 2, 3]));
        assert_structural_eq!(result, ValueContainer::from(vec![1, 2, 3]));
    }

    #[test]
    fn list_with_nested_scope() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("[1, (2 + 3), 4]");
        let expected = datex_list![
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
    #[test]
    fn failing_range() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("11..13");
        assert_eq!(
            result,
            ValueContainer::from(Range::new(
                Integer::from(11).into(),
                Integer::from(13).into()
            ))
        );
    }

    #[test]
    fn typed_integer() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result("-2i16");
        assert_eq!(result, TypedInteger::from(-2i16).into());
        assert_structural_eq!(result, ValueContainer::from(-2_i16));

        let result = execute_datex_script_debug_with_result("2i32");
        assert_eq!(result, TypedInteger::from(2i32).into());
        assert_structural_eq!(result, ValueContainer::from(2_i32));

        let result = execute_datex_script_debug_with_result("-2i64");
        assert_eq!(result, TypedInteger::from(-2i64).into());
        assert_structural_eq!(result, ValueContainer::from(-2_i64));

        let result = execute_datex_script_debug_with_result("2i128");
        assert_eq!(result, TypedInteger::from(2i128).into());
        assert_structural_eq!(result, ValueContainer::from(2_i128));

        let result = execute_datex_script_debug_with_result("2u8");
        assert_eq!(result, TypedInteger::from(2_u8).into());
        assert_structural_eq!(result, ValueContainer::from(2_u8));

        let result = execute_datex_script_debug_with_result("2u16");
        assert_eq!(result, TypedInteger::from(2_u16).into());
        assert_structural_eq!(result, ValueContainer::from(2_u16));

        let result = execute_datex_script_debug_with_result("2u32");
        assert_eq!(result, TypedInteger::from(2_u32).into());
        assert_structural_eq!(result, ValueContainer::from(2_u32));

        let result = execute_datex_script_debug_with_result("2u64");
        assert_eq!(result, TypedInteger::from(2_u64).into());
        assert_structural_eq!(result, ValueContainer::from(2_u64));

        let result = execute_datex_script_debug_with_result("2u128");
        assert_eq!(result, TypedInteger::from(2_u128).into());
        assert_structural_eq!(result, ValueContainer::from(2_u128));

        let result = execute_datex_script_debug_with_result("2big");
        assert_eq!(result, TypedInteger::Big(Integer::from(2)).into());
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
    fn map() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("{x: 1, y: 2, z: 42}");
        let map: CoreValue = result.clone().to_value().borrow().clone().inner;
        let map: Map = map.try_into().unwrap();

        // form and size
        assert_eq!(map.to_string(), "{\"x\": 1, \"y\": 2, \"z\": 42}");
        assert_eq!(map.size(), 3);

        info!("Map: {:?}", map);

        // access by key
        assert_eq!(map.get(&"x".into()), Some(&Integer::from(1i8).into()));
        assert_eq!(map.get(&"y".into()), Some(&Integer::from(2i8).into()));
        assert_eq!(map.get(&"z".into()), Some(&Integer::from(42i8).into()));

        // structural equality checks
        let expected_se: Map = Map::from(vec![
            ("x".to_string(), 1.into()),
            ("y".to_string(), 2.into()),
            ("z".to_string(), 42.into()),
        ]);
        assert_structural_eq!(map, expected_se);

        // strict equality checks
        let expected_strict: Map = Map::from(vec![
            ("x".to_string(), Integer::from(1_u32).into()),
            ("y".to_string(), Integer::from(2_u32).into()),
            ("z".to_string(), Integer::from(42_u32).into()),
        ]);
        debug!("Expected map: {expected_strict}");
        debug!("Map result: {map}");
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
        // FIXME #412: This should be probably disallowed (we can not use x in this scope due to hoisting behavior)
        let result =
            execute_datex_script_debug_with_result("[const x = 42, 2, x]");
        let expected = datex_list![
            Integer::from(42i8),
            Integer::from(2i8),
            Integer::from(42i8)
        ];
        assert_eq!(result, expected.into());
    }

    #[test]
    fn deref() {
        init_logger_debug();
        let result =
            execute_datex_script_debug_with_result("const x = &42; *x");
        assert_eq!(result, ValueContainer::from(Integer::from(42i8)));
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
        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x += 1",
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(43i8)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x += 1; x",
        );

        // FIXME #413 due to addition the resulting value container of the slot
        // is no longer a reference but a value what is incorrect.
        // assert_matches!(result, ValueContainer::Reference(..));
        assert_value_eq!(result, ValueContainer::from(Integer::from(43i8)));
    }

    #[test]
    fn ref_sub_assignment() {
        init_logger_debug();
        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x -= 1",
        );
        assert_value_eq!(result, ValueContainer::from(Integer::from(41i8)));

        let result = execute_datex_script_debug_with_result(
            "const x = &mut 42; *x -= 1; x",
        );

        // FIXME #414 due to addition the resulting value container of the slot
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
        let expected = datex_list![Integer::from(1i8), Integer::from(3i8)];
        assert_eq!(result, expected.into());
    }
}
