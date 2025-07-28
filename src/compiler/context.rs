use crate::global::binary_codes::InstructionCode;
use crate::utils::buffers::{
    append_f32, append_f64, append_i8, append_i16, append_i32, append_i64,
    append_i128, append_u8, append_u32, append_u128,
};
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::integer::utils::smallest_fitting_signed;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use binrw::BinWrite;
use itertools::Itertools;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;

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

pub struct Context<'a> {
    pub index: Cell<usize>,
    pub inserted_value_index: Cell<usize>,
    pub buffer: RefCell<Vec<u8>>,
    pub inserted_values: RefCell<&'a [&'a ValueContainer]>,
    /// this flag is set to true if any non-static value is encountered
    pub has_non_static_value: RefCell<bool>,

    // mapping for temporary scope slot resolution
    slot_indices: RefCell<HashMap<VirtualSlot, Vec<u32>>>,
}

impl<'a> Context<'a> {
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
        inserted_values: &'a [&'a ValueContainer],
    ) -> Self {
        Context {
            index: Cell::new(0),
            inserted_value_index: Cell::new(0),
            buffer,
            inserted_values: RefCell::new(inserted_values),
            has_non_static_value: RefCell::new(false),
            slot_indices: RefCell::new(HashMap::new()),
        }
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
            indices.push(self.index.get() as u32);
        } else {
            slot_indices.insert(virtual_slot, vec![self.index.get() as u32]);
        }
        self.append_u32(0); // placeholder for the slot address
    }

    pub fn insert_value_container(&self, value_container: &ValueContainer) {
        self.mark_has_non_static_value();
        match value_container {
            ValueContainer::Value(value) => self.insert_value(value),
            ValueContainer::Reference(reference) => {
                // TODO: in this case, the ref might also be inserted by pointer id, depending on the compiler settings
                // add CREATE_REF instruction
                self.append_binary_code(InstructionCode::CREATE_REF);
                self.insert_value(
                    &reference.borrow().current_resolved_value().borrow(),
                )
            }
        }
    }

    pub fn insert_value(&self, value: &Value) {
        match &value.inner {
            CoreValue::TypedInteger(val) | CoreValue::Integer(Integer(val)) => {
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
            CoreValue::Null => self.append_binary_code(InstructionCode::NULL),
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
                        && let CoreValue::Integer(Integer(integer)) = key.inner
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
    pub fn insert_boolean(&self, boolean: bool) {
        if boolean {
            self.append_binary_code(InstructionCode::TRUE);
        } else {
            self.append_binary_code(InstructionCode::FALSE);
        }
    }

    pub fn insert_text(&self, string: &str) {
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

    pub fn insert_key_value_pair(
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

    pub fn insert_key_string(&self, key_string: &str) {
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

    pub fn insert_typed_decimal(&self, decimal: &TypedDecimal) {
        fn insert_f32_or_f64(scope: &Context, decimal: &TypedDecimal) {
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

    pub fn insert_float32(&self, float32: f32) {
        self.append_binary_code(InstructionCode::DECIMAL_F32);
        self.append_f32(float32);
    }
    pub fn insert_float64(&self, float64: f64) {
        self.append_binary_code(InstructionCode::DECIMAL_F64);
        self.append_f64(float64);
    }

    pub fn insert_endpoint(&self, endpoint: &Endpoint) {
        self.append_binary_code(InstructionCode::ENDPOINT);
        self.append_buffer(&endpoint.to_binary());
    }

    pub fn insert_decimal(&self, decimal: &Decimal) {
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

    pub fn insert_float_as_i16(&self, int: i16) {
        self.append_binary_code(InstructionCode::DECIMAL_AS_INT_16);
        self.append_i16(int);
    }
    pub fn insert_float_as_i32(&self, int: i32) {
        self.append_binary_code(InstructionCode::DECIMAL_AS_INT_32);
        self.append_i32(int);
    }

    pub fn insert_int(&self, int: i64) {
        if (Context::MIN_INT_8..=Context::MAX_INT_8).contains(&int) {
            self.insert_i8(int as i8)
        } else if (Context::MIN_INT_16..=Context::MAX_INT_16).contains(&int) {
            self.insert_i16(int as i16)
        } else if (Context::MIN_INT_32..=Context::MAX_INT_32).contains(&int) {
            self.insert_i32(int as i32)
        } else {
            self.insert_i64(int)
        }
    }

    pub fn insert_i8(&self, int8: i8) {
        self.append_binary_code(InstructionCode::INT_8);
        self.append_i8(int8);
    }

    pub fn insert_i16(&self, int16: i16) {
        self.append_binary_code(InstructionCode::INT_16);
        self.append_i16(int16);
    }
    pub fn insert_i32(&self, int32: i32) {
        self.append_binary_code(InstructionCode::INT_32);
        self.append_i32(int32);
    }
    pub fn insert_i64(&self, int64: i64) {
        self.append_binary_code(InstructionCode::INT_64);
        self.append_i64(int64);
    }
    pub fn insert_i128(&self, int128: i128) {
        self.append_binary_code(InstructionCode::INT_128);
        self.append_i128(int128);
    }
    pub fn insert_u8(&self, uint8: u8) {
        self.append_binary_code(InstructionCode::INT_16);
        self.append_i16(uint8 as i16);
    }
    pub fn insert_u16(&self, uint16: u16) {
        self.append_binary_code(InstructionCode::INT_32);
        self.append_i32(uint16 as i32);
    }
    pub fn insert_u32(&self, uint32: u32) {
        self.append_binary_code(InstructionCode::INT_64);
        self.append_i64(uint32 as i64);
    }
    pub fn insert_u64(&self, uint64: u64) {
        self.append_binary_code(InstructionCode::INT_128);
        self.append_i128(uint64 as i128);
    }
    pub fn insert_u128(&self, uint128: u128) {
        self.append_binary_code(InstructionCode::UINT_128);
        self.append_i128(uint128 as i128);
    }
    pub fn append_u8(&self, u8: u8) {
        append_u8(self.buffer.borrow_mut().as_mut(), u8);
        self.index.update(|x| x + Context::INT_8_BYTES as usize);
    }
    pub fn append_u32(&self, u32: u32) {
        append_u32(self.buffer.borrow_mut().as_mut(), u32);
        self.index.update(|x| x + Context::INT_32_BYTES as usize);
    }
    pub fn set_u32_at_index(&self, u32: u32, index: usize) {
        let mut buffer = self.buffer.borrow_mut();
        buffer[index..index + Context::INT_32_BYTES as usize]
            .copy_from_slice(&u32.to_le_bytes());
    }
    pub fn append_i8(&self, i8: i8) {
        append_i8(self.buffer.borrow_mut().as_mut(), i8);
        self.index.update(|x| x + Context::INT_8_BYTES as usize);
    }
    pub fn append_i16(&self, i16: i16) {
        append_i16(self.buffer.borrow_mut().as_mut(), i16);
        self.index.update(|x| x + Context::INT_16_BYTES as usize);
    }
    pub fn append_i32(&self, i32: i32) {
        append_i32(self.buffer.borrow_mut().as_mut(), i32);
        self.index.update(|x| x + Context::INT_32_BYTES as usize);
    }
    pub fn append_i64(&self, i64: i64) {
        append_i64(self.buffer.borrow_mut().as_mut(), i64);
        self.index.update(|x| x + Context::INT_64_BYTES as usize);
    }
    pub fn append_i128(&self, i128: i128) {
        append_i128(self.buffer.borrow_mut().as_mut(), i128);
        self.index.update(|x| x + Context::INT_128_BYTES as usize);
    }

    pub fn append_u128(&self, u128: u128) {
        append_u128(self.buffer.borrow_mut().as_mut(), u128);
        self.index.update(|x| x + Context::INT_128_BYTES as usize);
    }

    pub fn append_f32(&self, f32: f32) {
        append_f32(self.buffer.borrow_mut().as_mut(), f32);
        self.index.update(|x| x + Context::FLOAT_32_BYTES as usize);
    }
    pub fn append_f64(&self, f64: f64) {
        append_f64(self.buffer.borrow_mut().as_mut(), f64);
        self.index.update(|x| x + Context::FLOAT_64_BYTES as usize);
    }
    pub fn append_string_utf8(&self, string: &str) {
        let bytes = string.as_bytes();
        (*self.buffer.borrow_mut()).extend_from_slice(bytes);
        self.index.update(|x| x + bytes.len());
    }
    pub fn append_buffer(&self, buffer: &[u8]) {
        (*self.buffer.borrow_mut()).extend_from_slice(buffer);
        self.index.update(|x| x + buffer.len());
    }

    pub fn mark_has_non_static_value(&self) {
        self.has_non_static_value.replace(true);
    }

    pub fn append_binary_code(&self, binary_code: InstructionCode) {
        self.append_u8(binary_code as u8);
    }
}
