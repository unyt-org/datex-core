use crate::{
    global::{
        protocol_structures::injected_values::{
            InjectedValueDeclaration, InjectedValueType,
            SharedInjectedValueType,
        },
        stack_index::StackIndex,
    },
    prelude::*,
    runtime::{
        Runtime,
        cache::shared_values_cache::SharedValuesCache,
        execution::{
            ExecutionError,
            execution_input::ExecutionCallerMetadata,
            execution_loop::{
                ExternalExecutionInterrupt, execution_loop,
                interrupts::InterruptProvider, runtime_value::RuntimeValue,
            },
        },
    },
    shared_values::{
        PointerAddress, SharedContainer,
        base_shared_value_container::observers::TransceiverId,
    },
    values::value_container::ValueContainer,
};
use core::{cell::RefCell, fmt::Debug};
use itertools::{EitherOrBoth, Itertools};

pub struct ExecutionLoopState {
    pub iterator: Box<
        dyn Iterator<Item = Result<ExternalExecutionInterrupt, ExecutionError>>,
    >,
    pub dxb_body: Rc<RefCell<Vec<u8>>>,
    pub(crate) interrupt_provider: InterruptProvider,
}

impl PartialEq for ExecutionLoopState {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.dxb_body, &other.dxb_body)
            && self.interrupt_provider == other.interrupt_provider
    }
}

impl Eq for ExecutionLoopState {}

impl ExecutionLoopState {
    pub fn new(
        dxb_body: Vec<u8>,
        shared_values: Vec<SharedContainer>,
        runtime: Runtime,
        stack: RuntimeExecutionStack,
        caller_metadata: ExecutionCallerMetadata,
    ) -> Self {
        let state = RuntimeExecutionState {
            runtime: runtime.clone(),
            source_id: TransceiverId::from(
                &caller_metadata
                    .endpoint
                    .as_local_if_endpoint(runtime.endpoint()),
            ),
            stack,
            caller_metadata,
            shared_value_cache: SharedValuesCache::new(shared_values),
        };
        // TODO #641: optimize, don't clone the whole DXB body every time here
        let dxb_rc = Rc::new(RefCell::new(dxb_body.to_vec()));
        let interrupt_provider = InterruptProvider::new();
        ExecutionLoopState {
            dxb_body: dxb_rc.clone(),
            iterator: Box::new(execution_loop(
                state,
                dxb_rc,
                interrupt_provider.clone(),
            )),
            interrupt_provider,
        }
    }
}

impl Debug for ExecutionLoopState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExecutionIterator")
            .field("dxb_body_length", &self.dxb_body.borrow().len())
            .finish()
    }
}

#[derive(Debug)]
pub struct RuntimeExecutionState {
    /// Local memory stack for current execution context.
    pub stack: RuntimeExecutionStack,
    pub runtime: Runtime,
    pub source_id: TransceiverId,
    pub caller_metadata: ExecutionCallerMetadata,
    pub shared_value_cache: SharedValuesCache,
}

impl RuntimeExecutionState {
    pub(crate) fn source_id_cloned(&self) -> TransceiverId {
        self.source_id.clone()
    }

    /// Normalizes a pointer address to ensure it is in the correct form for the current execution context.
    /// For self-owned addresses, it converts them to remote addresses owned by the caller's endpoint.
    /// For remote addresses, it ensures that if the address is local to the current runtime, it is normalized to a @@local address.
    pub fn normalize_pointer_address(
        &self,
        address: &PointerAddress,
    ) -> PointerAddress {
        let local_endpoint = self.runtime.endpoint().clone();
        let owner_endpoint = &self.caller_metadata.endpoint;

        match address {
            // convert self owned to caller owned
            PointerAddress::SelfOwned(address) => PointerAddress::Remote(
                address.remote_for_endpoint(owner_endpoint),
            )
            .normalize_for_local(&local_endpoint),
            // make sure remote with local endpoint is normalized to @@local
            PointerAddress::Remote(address) => {
                address.normalize_for_local(&local_endpoint)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct RuntimeExecutionStack {
    pub values: Vec<Option<ValueContainer>>,
}

impl RuntimeExecutionStack {
    /// Creates a new stack with initial allocated stack values.
    pub fn new(values: Vec<ValueContainer>) -> Self {
        Self {
            values: values.into_iter().map(Some).collect(),
        }
    }

    /// Pushes a value to the stack
    pub(crate) fn push(&mut self, value: ValueContainer) {
        self.values.push(Some(value));
    }

    /// Pushes multiple values to the stack
    pub(crate) fn push_multiple(&mut self, values: Vec<ValueContainer>) {
        self.values.extend(values.into_iter().map(Some));
    }

    /// Returns the current index for the next value to be pushed to the stack, which is the length of the current stack values.
    pub(crate) fn current_index(&self) -> StackIndex {
        StackIndex(self.values.len() as u32)
    }

    /// Frees the stack to the given index, removing all values above that index. If the index is out of bounds, it panics.
    pub(crate) fn truncate(&mut self, index: StackIndex) {
        if index.0 as usize > self.values.len() {
            panic!(
                "Cannot free stack to index {:?} as it is out of bounds (current stack size: {})",
                index,
                self.values.len()
            );
        }
        self.values.truncate(index.0 as usize);
    }

    /// Takes a stack value by its index and returns its value.
    /// If the stack value is not allocated or the index is out of bounds, it returns an error.
    pub(crate) fn take_stack_value(
        &mut self,
        index: StackIndex,
    ) -> Result<ValueContainer, ExecutionError> {
        if let Some(stack_value) = self.values.get_mut(index.0 as usize) {
            stack_value
                .take()
                .ok_or_else(|| ExecutionError::StackValueNotAllocated(index))
        } else {
            Err(ExecutionError::StackOutOfBoundsAccess(index))
        }
    }

    /// Sets the value of a stack index, returning the previous value if it existed.
    /// If the stack value is not allocated, it returns an error.
    pub(crate) fn set_stack_value(
        &mut self,
        index: StackIndex,
        value: ValueContainer,
    ) -> Result<Option<ValueContainer>, ExecutionError> {
        if let Some(stack_value) = self.values.get_mut(index.0 as usize) {
            Ok(stack_value.replace(value))
        } else {
            Err(ExecutionError::StackOutOfBoundsAccess(index))
        }
    }

    /// Retrieves a reference to the value of a stack value by its address.
    /// If the stack value is not allocated, it returns an error.
    pub(crate) fn get_stack_value(
        &self,
        index: StackIndex,
    ) -> Result<&ValueContainer, ExecutionError> {
        if let Some(stack_value) = self.values.get(index.0 as usize) {
            stack_value
                .as_ref()
                .ok_or_else(|| ExecutionError::StackValueNotAllocated(index))
        } else {
            Err(ExecutionError::StackOutOfBoundsAccess(index))
        }
    }

    /// Retrieves a mutable reference to the stack value by its index.
    /// If the stack value is not allocated, it returns an error.
    pub(crate) fn get_stack_value_mut(
        &mut self,
        index: StackIndex,
    ) -> Result<&mut ValueContainer, ExecutionError> {
        if let Some(stack_value) = self.values.get_mut(index.0 as usize) {
            stack_value
                .as_mut()
                .ok_or_else(|| ExecutionError::StackValueNotAllocated(index))
        } else {
            Err(ExecutionError::StackOutOfBoundsAccess(index))
        }
    }

    /// Resolves a list of injected values to actual values on the stack
    pub fn resolve_injected_values(
        &mut self,
        injected_values: &[InjectedValueDeclaration],
    ) -> Result<Vec<ValueContainer>, ExecutionError> {
        let mut moved: Vec<Option<_>> = vec![None; injected_values.len()];

        // perform all mutable operations (removing moved shared values)
        for (i, InjectedValueDeclaration { index, ty }) in
            injected_values.iter().enumerate()
        {
            if matches!(
                ty,
                InjectedValueType::Shared(SharedInjectedValueType::Move)
            ) {
                moved[i] = Some(self.take_stack_value(*index)?);
            }
        }

        // collect all values
        let mut resolved_values = Vec::with_capacity(injected_values.len());
        for (i, InjectedValueDeclaration { index, ty }) in
            injected_values.iter().enumerate()
        {
            resolved_values.push(match ty {
                InjectedValueType::Shared(SharedInjectedValueType::Move) => {
                    match moved[i].take().unwrap() {
                        shared @ ValueContainer::Shared(_) => shared,
                        local @ ValueContainer::Local(_) => {
                            local
                            // FIXME: should return this error, but since compiler currently also marks local values as shared, this is disabled for now
                            // return Err(ExecutionError::ExpectedSharedValue);
                        }
                    }
                }
                _ => self.get_stack_value(*index)?.clone(), // TODO: avoid clone?
            });
        }

        Ok(resolved_values)
    }

    /// Resolves a list of runtime values to actual values on the stack,
    /// returning both the resolved values and their corresponding stack indices (for values stored on the stack)
    pub fn take_runtime_values_with_stack_indices(
        &mut self,
        values: Vec<RuntimeValue>,
    ) -> Result<(Vec<ValueContainer>, Vec<Option<StackIndex>>), ExecutionError>
    {
        let mut stack_indices = Vec::new();
        let mut resolved_values = Vec::new();

        for value in values {
            match value {
                RuntimeValue::StackValue(index) => {
                    stack_indices.push(Some(index));
                    resolved_values.push(self.take_stack_value(index)?);
                }
                RuntimeValue::ValueContainer(value) => {
                    stack_indices.push(None);
                    resolved_values.push(value);
                }
            }
        }

        Ok((resolved_values, stack_indices))
    }

    /// Restores values to the stack at the given indices, if both the index and value are available.
    pub fn restore_stack_values(
        &mut self,
        values: Vec<ValueContainer>,
        stack_indices: Vec<Option<StackIndex>>,
    ) -> Result<(), ExecutionError> {
        for x in stack_indices.into_iter().flatten().zip_longest(values) {
            match x {
                EitherOrBoth::Both(previous, next) => {
                    // If a stack index is available, and the value for the reserved stack index is available, restore the value to the stack at the given index.
                    self.set_stack_value(previous, next)?;
                }
                _ => {
                    // error if the number of stack indices does not match the number of values
                    return Err(ExecutionError::StackRestoreMismatch);
                }
            }
        }
        Ok(())
    }
}
