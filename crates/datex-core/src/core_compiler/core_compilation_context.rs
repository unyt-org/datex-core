use core::cell::RefCell;
use crate::{
    core_compiler::{
        preamble::append_injected_values_preamble,
        shared_value_tracking::SharedValueTracking,
    },
    prelude::*,
    runtime::pointer_availability_lookup::PointerAvailabilityLookup,
    shared_values::SharedContainer,
    values::{
        core_values::endpoint::Endpoint,
    },
};
use binrw::io::Cursor;
use crate::core_compiler::to_instructions::ToInstructions;
use crate::core_compiler::value_visitor::{compile_instructions_to_dxb, ValueVisitor};

pub type ByteCursor = Cursor<Vec<u8>>;

#[derive(Debug)]
pub struct DXBWithSharedValues {
    pub dxb: Vec<u8>,
    /// Shared values that can be accessed inside the execution
    pub shared_values: Vec<SharedContainer>,
}
impl DXBWithSharedValues {
    /// Create a new [DXBWithSharedValues] with the provided DXB bytecode and shared values.
    pub fn new(dxb: Vec<u8>, shared_values: Vec<SharedContainer>) -> Self {
        DXBWithSharedValues { dxb, shared_values }
    }

    /// Create a new [DXBWithSharedValues] with the provided DXB bytecode and shared values.
    /// Tries to upgrade referenced shared containers to owned containers if possible.
    ///
    /// # Safety
    /// The caller must ensure for all shared_values that no other [OwnedSharedContainer] for the same
    /// inner value exists if move_indicator of the [ReferencedSharedContainer] is set.
    pub unsafe fn new_with_upgraded_owned_containers(
        dxb: Vec<u8>,
        shared_values: Vec<SharedContainer>,
    ) -> Self {
        // Force convert referenced containers with move_indicator flag
        // to OwnedSharedContainer.
        let shared_values = shared_values
            .into_iter()
            .map(|shared_container| {
                unsafe { shared_container.try_upgrade_to_owned() }
                    .map(SharedContainer::Owned)
                    .unwrap_or_else(SharedContainer::Referenced)
            })
            .collect();
        DXBWithSharedValues { dxb, shared_values }
    }

    pub fn into_dxb(self) -> Vec<u8> {
        self.dxb
    }
}

#[derive(Debug, Clone)]
pub struct CompileInput<'a> {
    pub pointer_lookup: &'a PointerAvailabilityLookup,
    pub receivers: &'a [Endpoint],
}
impl<'a> CompileInput<'a> {
    pub fn new(
        pointer_lookup: &'a PointerAvailabilityLookup,
        receivers: &'a [Endpoint],
    ) -> Self {
        CompileInput {
            pointer_lookup,
            receivers,
        }
    }
}

/// # Safety
/// This function is unsafe because it creates a CompileInput with a static lifetime, which may lead to dangling references if used incorrectly. It should only be used in tests where the leaked memory is acceptable
pub(crate) unsafe fn default_compile_input<'a>() -> CompileInput<'a> {
    CompileInput {
        pointer_lookup: Box::leak(Box::new(
            PointerAvailabilityLookup::default(),
        )),
        receivers: Box::leak(Box::new([])),
    }
}

#[derive(Debug)]
pub struct CoreCompilationContext<'a> {
    pub shared_value_tracking: RefCell<SharedValueTracking<'a>>,
    pub input: CompileInput<'a>,
}

/// # Safety
/// This function is unsafe because it creates a CoreCompilationContext
pub(crate) unsafe fn default_core_compilation_context<'a>()
-> CoreCompilationContext<'a> {
    CoreCompilationContext::new(unsafe { default_compile_input() })
}

impl<'a> CoreCompilationContext<'a> {
    /// Create a new core compilation context with an initial byte input buffer
    pub fn new(
        input: CompileInput<'a>,
    ) -> CoreCompilationContext<'a> {
        CoreCompilationContext {
            shared_value_tracking: RefCell::new(SharedValueTracking::new(
                input.pointer_lookup,
                input.receivers,
            )),
            input,
        }
    }

    /// Create a new core compilation context for the provided receiver endpoints
    pub fn new_for_endpoints(
        pointer_lookup: &'a PointerAvailabilityLookup,
        receivers: &'a [Endpoint],
    ) -> CoreCompilationContext<'a> {
        let input = CompileInput::new(pointer_lookup, receivers);
        CoreCompilationContext::new(input)
    }

    /// Compiles a [ToInstructions] value with shared value tracking and appends a preamble with the injected shared values.
    /// Returns the final byte buffer and the list of shared values that were moved or referenced during compilation
    pub fn compile_to_dxb_with_shared_values(mut self, input: impl ToInstructions) -> DXBWithSharedValues {
        let compiled_body = compile_instructions_to_dxb(self.shared_value_tracking.get_mut(), input);

        self.append_preamble_to_dxb(compiled_body)
    }
    
    /// Appends a preamble with the injected shared values to the provided, already compiled body and returns a [DXBWithSharedValues] 
    /// containing the combined buffer and the list of shared values that were moved or referenced during compilation.
    /// Note: This function should only be called with a body that results from a compilation that used the same shared value tracking as this context, otherwise the shared values may not match the body.
    pub fn append_preamble_to_dxb(
        self,
        compiled_body: Vec<u8>
    ) -> DXBWithSharedValues {
        let tracked_values = self.shared_value_tracking.into_inner().into_tracked_values();
        let (combined_buffer, top_level_values) =
            append_injected_values_preamble(tracked_values, compiled_body);
        // SAFETY: it is assumed that the tracked values from the shared value tracking were
        // moved inside the compilation and are no longer accessible from the outside
        unsafe {
            DXBWithSharedValues::new_with_upgraded_owned_containers(
                combined_buffer,
                top_level_values,
            )
        }
    }
}
