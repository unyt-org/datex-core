use crate::global::instruction_codes::InstructionCode;
use crate::utils::buffers::{
    append_f32, append_f64, append_i8, append_i16, append_i32, append_i64,
    append_i128, append_u8, append_u32, append_u128,
};
use crate::values::core_values::integer::Integer;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use itertools::Itertools;
use core::cell::{Cell, RefCell};
use core::cmp::PartialEq;
use datex_core::core_compiler::value_compiler::append_instruction_code;
use crate::core_compiler::value_compiler::append_value_container;
use crate::stdlib::collections::HashMap;

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
            panic!("Cannot upgrade a local slot");
        }
    }
}

/// compilation context, created for each compiler call, even if compiling a script for the same scope
pub struct CompilationContext {
    pub inserted_value_index: Cell<usize>,
    pub buffer: RefCell<Vec<u8>>,
    // FIXME #485: use lifetimes and references here
    pub inserted_values: RefCell<Vec<ValueContainer>>,
    /// this flag is set to true if any non-static value is encountered
    pub has_non_static_value: RefCell<bool>,

    /// Set to true if no further source text is expected to be compiled.
    /// Example: for a REPL, this is set to false
    pub is_end_of_source_text: bool,

    // mapping for temporary scope slot resolution
    slot_indices: RefCell<HashMap<VirtualSlot, Vec<u32>>>,
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
        buffer: RefCell<Vec<u8>>,
        inserted_values: Vec<ValueContainer>,
        is_end_of_source_text: bool,
    ) -> Self {
        CompilationContext {
            inserted_value_index: Cell::new(0),
            buffer,
            inserted_values: RefCell::new(inserted_values),
            has_non_static_value: RefCell::new(false),
            slot_indices: RefCell::new(HashMap::new()),
            is_end_of_source_text,
        }
    }

    pub fn index(&self) -> usize {
        self.buffer.borrow().len()
    }

    fn insert_value_container(&self, value_container: &ValueContainer) {
        append_value_container(self.buffer.borrow_mut().as_mut(), value_container);
    }

    pub fn external_slots(&self) -> Vec<VirtualSlot> {
        self.slot_indices
            .borrow()
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
            .borrow()
            .iter()
            .filter(|(slot, _)| slot.is_external() == match_externals)
            .sorted_by(|a, b| a.0.virtual_address.cmp(&b.0.virtual_address))
            .map(|(_, indices)| indices.clone())
            .collect()
    }

    pub fn remap_virtual_slots(&self) {
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
    pub fn insert_virtual_slot_address(&self, virtual_slot: VirtualSlot) {
        let mut slot_indices = self.slot_indices.borrow_mut();
        if let Some(indices) = slot_indices.get_mut(&virtual_slot) {
            indices.push(self.index() as u32);
        } else {
            slot_indices.insert(virtual_slot, vec![self.index() as u32]);
        }
        append_u32(self.buffer.borrow_mut().as_mut(), 0); // placeholder for the slot address
    }

    // TODO #440: we should probably not compile unions with nested binary operations, but rather have a separate instruction for n-ary unions
    // pub fn insert_union(&self, union: &Union) {
    //     // insert values as nested UNION binary operations

    //     self.append_binary_code(InstructionCode::UNION);
    //     // insert first value
    //     self.insert_value_container(&union.options[0]);

    //     // insert rest of values recursively
    //     self.insert_union_options(union.options[1..].to_vec());
    // }

    fn insert_union_options(&self, options: Vec<ValueContainer>) {
        // directly insert value if only one option left
        if options.len() == 1 {
            self.insert_value_container(&options[0]);
        } else {
            self.append_instruction_code(InstructionCode::SCOPE_START);
            self.append_instruction_code(InstructionCode::UNION);
            // insert first value
            self.insert_value_container(&options[0]);
            // insert rest of values recursively
            self.insert_union_options(options[1..].to_vec());
            self.append_instruction_code(InstructionCode::SCOPE_END);
        }
    }
    pub fn set_u32_at_index(&self, u32: u32, index: usize) {
        let mut buffer = self.buffer.borrow_mut();
        buffer[index..index + CompilationContext::INT_32_BYTES as usize]
            .copy_from_slice(&u32.to_le_bytes());
    }

    pub fn mark_has_non_static_value(&self) {
        self.has_non_static_value.replace(true);
    }

    pub fn append_instruction_code(&self, code: InstructionCode) {
        append_instruction_code(self.buffer.borrow_mut().as_mut(), code);
    }
}
