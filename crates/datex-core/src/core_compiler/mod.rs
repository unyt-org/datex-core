//! This module contains the core compiler logic for DATEX, including [value_compiler] and [type_compiler] needed for compilation.
pub mod core_compilation_context;
pub mod injected_values;
mod preamble;
pub mod shared_value_tracking;
pub mod traits;
pub mod type_compiler;
pub mod update_compiler;
pub mod value_compiler;
pub mod instruction_input;

use crate::{
    core_compiler::{
        core_compilation_context::{
            CoreCompilationContext, DXBWithSharedValues,
        },
    },
    prelude::*,
    runtime::pointer_availability_lookup::PointerAvailabilityLookup,
    values::{
        core_values::endpoint::Endpoint,
    },
};
pub use traits::*;
use crate::core_compiler::instruction_input::InstructionInput;

/// Compiles a [DXBWithSharedValues] from the provided [PointerAvailabilityLookup], [Endpoint]s, and [InstructionInput]s.
pub fn core_compile(
    pointer_availability_lookup: &PointerAvailabilityLookup,
    endpoints: &[Endpoint],
    instructions_input: Vec<InstructionInput>,
) -> DXBWithSharedValues {
    CoreCompilationContext::new_for_endpoints(
        pointer_availability_lookup,
        endpoints,
    ).compile_to_dxb_with_shared_values(instructions_input)
}