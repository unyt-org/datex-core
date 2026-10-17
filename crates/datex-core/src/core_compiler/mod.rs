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
use crate::core_compiler::to_instructions::{ToInstructions, yield_all_collected};
use crate::core_compiler::value_visitor::compile_instruction_iterator_to_dxb;

/// Compiles a [DXBWithSharedValues] from the provided [PointerAvailabilityLookup], [Endpoint]s, and [InstructionInput]s.
pub fn core_compile(
    pointer_availability_lookup: &PointerAvailabilityLookup,
    endpoints: &[Endpoint],
    instructions_input: Vec<InstructionInput>,
) -> DXBWithSharedValues {
    let mut ctx = CoreCompilationContext::new_for_endpoints(
        pointer_availability_lookup,
        endpoints,
    );
    
    let dxb_gen = gen {
        for input in instructions_input {
            match input {
                InstructionInput::Instruction(instruction) => yield instruction,
                InstructionInput::ValueContainer(value_container) => yield_all_collected!(value_container.to_instructions(ctx.shared_value_tracking.get_mut())),
            }
        }
    };

    let dxb = compile_instruction_iterator_to_dxb(dxb_gen);
    
    ctx.append_preamble_to_dxb(dxb)
}