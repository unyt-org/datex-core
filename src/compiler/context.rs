use crate::collections::HashMap;
use crate::core_compiler::value_compiler::append_instruction_code;
use crate::core_compiler::value_compiler::append_value_container;
use crate::global::instruction_codes::InstructionCode;
use crate::runtime::execution::context::ExecutionMode;
use crate::utils::buffers::append_u32;
use crate::values::value_container::ValueContainer;
use core::cmp::PartialEq;
use itertools::Itertools;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub struct VirtualSlot {
    pub level: u8, // parent scope level if exists, otherwise 0
    // local slot address of scope with level
    pub virtual_address: u32,
}

impl VirtualSlot {
    pub fn local(virtual_address: u32) -> Self {
        VirtualSlot {
            level: 0,
            virtual_address,
        }
    }
    pub fn is_external(&self) -> bool {
        self.level > 0
    }

    pub fn external(level: u8, virtual_address: u32) -> Self {
        VirtualSlot {
            level,
            virtual_address,
        }
    }

    pub fn downgrade(&self) -> Self {
        VirtualSlot {
            level: self.level + 1,
            virtual_address: self.virtual_address,
        }
    }

    pub fn upgrade(&self) -> Self {
        if self.level > 0 {
            VirtualSlot {
                level: self.level - 1,
                virtual_address: self.virtual_address,
            }
        } else {
            core::panic!("Cannot upgrade a local slot");
        }
    }
}

/// compilation context, created for each compiler call, even if compiling a script for the same scope
pub struct CompilationContext {
    pub inserted_value_index: usize,
    pub buffer: Vec<u8>,
    pub inserted_values: Vec<ValueContainer>,
    /// this flag is set to true if any non-static value is encountered
    pub has_non_static_value: bool,
    pub execution_mode: ExecutionMode,

    // mapping for temporary scope slot resolution
    slot_indices: HashMap<VirtualSlot, Vec<u32>>,
}

impl CompilationContext {
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

    pub fn new(
        buffer: Vec<u8>,
        inserted_values: Vec<ValueContainer>,
        execution_mode: ExecutionMode,
    ) -> Self {
        CompilationContext {
            inserted_value_index: 0,
            buffer,
            inserted_values,
            has_non_static_value: false,
            slot_indices: HashMap::new(),
            execution_mode,
        }
    }

    pub fn buffer_index(&self) -> usize {
        self.buffer.len()
    }

    fn insert_value_container(&mut self, value_container: &ValueContainer) {
        append_value_container(&mut self.buffer, value_container);
    }

    pub fn external_slots(&self) -> Vec<VirtualSlot> {
        self.slot_indices
            .iter()
            .filter(|(slot, _)| slot.is_external())
            .sorted_by(|a, b| a.0.virtual_address.cmp(&b.0.virtual_address))
            .map(|(slot, _)| *slot)
            .collect()
    }

    /// Gets all slots for either local or external slots depending on the value of external
    pub fn get_slot_byte_indices(
        &self,
        match_externals: bool,
    ) -> Vec<Vec<u32>> {
        self.slot_indices
            .iter()
            .filter(|(slot, _)| slot.is_external() == match_externals)
            .sorted_by(|a, b| a.0.virtual_address.cmp(&b.0.virtual_address))
            .map(|(_, indices)| indices.clone())
            .collect()
    }

    pub fn remap_virtual_slots(&mut self) {
        let mut slot_address = 0;

        // parent slots
        for byte_indices in self.get_slot_byte_indices(true) {
            for byte_index in byte_indices {
                self.set_u32_at_index(slot_address, byte_index as usize);
            }
            slot_address += 1;
        }

        // local slots
        for byte_indices in self.get_slot_byte_indices(false) {
            for byte_index in byte_indices {
                self.set_u32_at_index(slot_address, byte_index as usize);
            }
            slot_address += 1;
        }
    }

    // This method writes a placeholder value for the slot
    // since the slot address is not known yet and just temporary.
    pub fn insert_virtual_slot_address(&mut self, virtual_slot: VirtualSlot) {
        let buffer_index = self.buffer_index() as u32;
        if let Some(indices) = self.slot_indices.get_mut(&virtual_slot) {
            indices.push(buffer_index);
        } else {
            self.slot_indices.insert(virtual_slot, vec![buffer_index]);
        }
        append_u32(&mut self.buffer, 0); // placeholder for the slot address
    }

    pub fn set_u32_at_index(&mut self, u32: u32, index: usize) {
        self.buffer[index..index + CompilationContext::INT_32_BYTES as usize]
            .copy_from_slice(&u32.to_le_bytes());
    }

    pub fn mark_has_non_static_value(&mut self) {
        self.has_non_static_value = true;
    }

    pub fn append_instruction_code(&mut self, code: InstructionCode) {
        append_instruction_code(&mut self.buffer, code);
    }
}
