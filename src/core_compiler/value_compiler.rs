use crate::core_compiler::type_compiler::append_type;
use crate::global::instruction_codes::InstructionCode;
use crate::libs::core::{CoreLibPointerId, get_core_lib_type_definition};
use crate::references::reference::ReferenceMutability;
use crate::stdlib::vec::Vec;
use crate::types::definition::TypeDefinition;
use crate::utils::buffers::{
    append_f32, append_f64, append_i8, append_i16, append_u8, append_u32,
    append_u128,
};
use crate::utils::buffers::{
    append_i32, append_i64, append_i128, append_u16, append_u64,
};
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::integer::utils::smallest_fitting_signed;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use binrw::BinWrite;
use binrw::io::Cursor;
use core::prelude::rust_2024::*;

/// Compiles a given value container to a DXB body
pub fn compile_value_container(value_container: &ValueContainer) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(256);
    append_value_container(&mut buffer, value_container);

    buffer
}

pub fn append_value_container(
    buffer: &mut Vec<u8>,
    value_container: &ValueContainer,
) {
    match value_container {
        ValueContainer::Value(value) => append_value(buffer, value),
        ValueContainer::Reference(reference) => {
            // TODO #160: in this case, the ref might also be inserted by pointer id, depending on the compiler settings
            // add CREATE_REF/CREATE_REF_MUT instruction
            if reference.mutability() == ReferenceMutability::Mutable {
                append_instruction_code(
                    buffer,
                    InstructionCode::CREATE_REF_MUT,
                );
            } else {
                append_instruction_code(buffer, InstructionCode::CREATE_REF);
            }
            // insert pointer id + value or only id
            // add pointer to memory if not there yet
            append_value(buffer, &reference.collapse_to_value().borrow())
        }
    }
}

pub fn append_value(buffer: &mut Vec<u8>, value: &Value) {
    // append non-default type information
    if !value.has_default_type() {
        append_type_cast(buffer, &value.actual_type);
    }
    match &value.inner {
        CoreValue::Type(ty) => {
            core::todo!("#439 Type value not supported in CompilationContext");
        }
        CoreValue::Integer(integer) => {
            // NOTE: we might optimize this later, but using INT with big integer encoding 
            // for all integers for now 
            // let integer = integer.to_smallest_fitting();
            // append_encoded_integer(buffer, &integer);
            append_integer(buffer, integer);
        }
        CoreValue::TypedInteger(integer) => {
            append_encoded_integer(buffer, integer);
        }

        CoreValue::Endpoint(endpoint) => append_endpoint(buffer, endpoint),
        CoreValue::Decimal(decimal) => append_decimal(buffer, decimal),
        CoreValue::TypedDecimal(val) => append_encoded_decimal(buffer, val),
        CoreValue::Boolean(val) => append_boolean(buffer, val.0),
        CoreValue::Null => {
            append_instruction_code(buffer, InstructionCode::NULL)
        }
        CoreValue::Text(val) => {
            append_text(buffer, &val.0);
        }
        CoreValue::List(val) => {
            // if list size < 256, use SHORT_LIST
            match val.len() {
                0..=255 => {
                    append_instruction_code(
                        buffer,
                        InstructionCode::SHORT_LIST,
                    );
                    append_u8(buffer, val.len() as u8);
                }
                _ => {
                    append_instruction_code(buffer, InstructionCode::LIST);
                    append_u32(buffer, val.len());
                }
            }

            for item in val {
                append_value_container(buffer, item);
            }
        }
        CoreValue::Map(val) => {
            // if map size < 256, use SHORT_MAP
            match val.size() {
                0..=255 => {
                    append_instruction_code(buffer, InstructionCode::SHORT_MAP);
                    append_u8(buffer, val.size() as u8);
                }
                _ => {
                    append_instruction_code(buffer, InstructionCode::MAP);
                    append_u32(buffer, val.size() as u32); // FIXME: casting from usize to u32 here
                }
            }
            for (key, value) in val {
                append_key_value_pair(
                    buffer,
                    &ValueContainer::from(key),
                    value,
                );
            }
        }
    }
}

pub fn append_type_cast(buffer: &mut Vec<u8>, ty: &TypeDefinition) {
    append_instruction_code(buffer, InstructionCode::TYPED_VALUE);
    // TODO: optimize: avoid cloning
    append_type(buffer, &(ty.clone().into_type(None)));
}

pub fn append_text(buffer: &mut Vec<u8>, string: &str) {
    let bytes = string.as_bytes();
    let len = bytes.len();

    if len < 256 {
        append_instruction_code(buffer, InstructionCode::SHORT_TEXT);
        append_u8(buffer, len as u8);
    } else {
        append_instruction_code(buffer, InstructionCode::TEXT);
        append_u32(buffer, len as u32);
    }

    buffer.extend_from_slice(bytes);
}

pub fn append_boolean(buffer: &mut Vec<u8>, boolean: bool) {
    if boolean {
        append_instruction_code(buffer, InstructionCode::TRUE);
    } else {
        append_instruction_code(buffer, InstructionCode::FALSE);
    }
}

pub fn append_decimal(buffer: &mut Vec<u8>, decimal: &Decimal) {
    append_instruction_code(buffer, InstructionCode::DECIMAL);
    append_big_decimal(buffer, decimal);
}

pub fn append_big_decimal(buffer: &mut Vec<u8>, decimal: &Decimal) {
    // big_decimal binrw write into buffer
    let original_length = buffer.len();
    let mut buffer_writer = Cursor::new(&mut *buffer);
    // set writer position to end
    buffer_writer.set_position(original_length as u64);
    decimal
        .write_le(&mut buffer_writer)
        .expect("Failed to write big decimal");
}

pub fn append_endpoint(buffer: &mut Vec<u8>, endpoint: &Endpoint) {
    append_instruction_code(buffer, InstructionCode::ENDPOINT);
    buffer.extend_from_slice(&endpoint.to_binary());
}

/// Appends a typed integer with explicit type casts
pub fn append_typed_integer(buffer: &mut Vec<u8>, integer: &TypedInteger) {
    append_type_cast(
        buffer,
        &get_core_lib_type_definition(CoreLibPointerId::from(integer)),
    );
    append_encoded_integer(buffer, &integer);
}

/// Appends a default, unsized integer
pub fn append_integer(buffer: &mut Vec<u8>, integer: &Integer) {
    append_instruction_code(buffer, InstructionCode::INT);
    append_big_integer(buffer, integer);
}

/// Appends an encoded integer without explicit type casts
pub fn append_encoded_integer(buffer: &mut Vec<u8>, integer: &TypedInteger) {
    match integer {
        TypedInteger::I8(val) => {
            append_instruction_code(buffer, InstructionCode::INT_8);
            append_i8(buffer, *val);
        }
        TypedInteger::I16(val) => {
            append_instruction_code(buffer, InstructionCode::INT_16);
            append_i16(buffer, *val);
        }
        TypedInteger::I32(val) => {
            append_instruction_code(buffer, InstructionCode::INT_32);
            append_i32(buffer, *val);
        }
        TypedInteger::I64(val) => {
            append_instruction_code(buffer, InstructionCode::INT_64);
            append_i64(buffer, *val);
        }
        TypedInteger::I128(val) => {
            append_instruction_code(buffer, InstructionCode::INT_128);
            append_i128(buffer, *val);
        }
        TypedInteger::U8(val) => {
            append_instruction_code(buffer, InstructionCode::UINT_8);
            append_u8(buffer, *val);
        }
        TypedInteger::U16(val) => {
            append_instruction_code(buffer, InstructionCode::UINT_16);
            append_u16(buffer, *val);
        }
        TypedInteger::U32(val) => {
            append_instruction_code(buffer, InstructionCode::UINT_32);
            append_u32(buffer, *val);
        }
        TypedInteger::U64(val) => {
            append_instruction_code(buffer, InstructionCode::UINT_64);
            append_u64(buffer, *val);
        }
        TypedInteger::U128(val) => {
            append_instruction_code(buffer, InstructionCode::UINT_128);
            append_u128(buffer, *val);
        }
        TypedInteger::Big(val) => {
            append_instruction_code(buffer, InstructionCode::INT_BIG);
            append_big_integer(buffer, val);
        }
    }
}

pub fn append_encoded_decimal(buffer: &mut Vec<u8>, decimal: &TypedDecimal) {
    fn append_f32_or_f64(buffer: &mut Vec<u8>, decimal: &TypedDecimal) {
        match decimal {
            TypedDecimal::F32(val) => {
                append_float32(buffer, val.into_inner());
            }
            TypedDecimal::F64(val) => {
                append_float64(buffer, val.into_inner());
            }
            TypedDecimal::Decimal(val) => {
                append_instruction_code(buffer, InstructionCode::DECIMAL_BIG);
                append_big_decimal(buffer, val);
            }
        }
    }

    append_f32_or_f64(buffer, decimal);
    
    // TODO: maybe use this in the future, but type casts are necessary to decide which actual type is represented
    // match decimal.as_integer() {
    //     Some(int) => {
    //         let smallest = smallest_fitting_signed(int as i128);
    //         match smallest {
    //             TypedInteger::I8(val) => {
    //                 append_float_as_i16(buffer, val as i16);
    //             }
    //             TypedInteger::I16(val) => {
    //                 append_float_as_i16(buffer, val);
    //             }
    //             TypedInteger::I32(val) => {
    //                 append_float_as_i32(buffer, val);
    //             }
    //             _ => append_f32_or_f64(buffer, decimal),
    //         }
    //     }
    //     None => append_f32_or_f64(buffer, decimal),
    // }
}

pub fn append_float32(buffer: &mut Vec<u8>, float32: f32) {
    append_instruction_code(buffer, InstructionCode::DECIMAL_F32);
    append_f32(buffer, float32);
}
pub fn append_float64(buffer: &mut Vec<u8>, float64: f64) {
    append_instruction_code(buffer, InstructionCode::DECIMAL_F64);
    append_f64(buffer, float64);
}

pub fn append_big_integer(buffer: &mut Vec<u8>, integer: &Integer) {
    // use BinWrite to write the integer to the buffer
    // big_integer binrw write into buffer
    let original_length = buffer.len();
    let mut buffer_writer = Cursor::new(&mut *buffer);
    // set writer position to end
    buffer_writer.set_position(original_length as u64);
    integer
        .write_le(&mut buffer_writer)
        .expect("Failed to write big integer");
}

pub fn append_typed_decimal(buffer: &mut Vec<u8>, decimal: &TypedDecimal) {
    append_type_cast(
        buffer,
        &get_core_lib_type_definition(CoreLibPointerId::from(decimal)),
    );
    append_encoded_decimal(buffer, decimal);
}

pub fn append_float_as_i16(buffer: &mut Vec<u8>, int: i16) {
    append_instruction_code(buffer, InstructionCode::DECIMAL_AS_INT_16);
    append_i16(buffer, int);
}
pub fn append_float_as_i32(buffer: &mut Vec<u8>, int: i32) {
    append_instruction_code(buffer, InstructionCode::DECIMAL_AS_INT_32);
    append_i32(buffer, int);
}

pub fn append_get_ref(buffer: &mut Vec<u8>, address: &PointerAddress) {
    match address {
        PointerAddress::Internal(id) => {
            append_instruction_code(buffer, InstructionCode::GET_INTERNAL_REF);
            buffer.extend_from_slice(id);
        }
        PointerAddress::Local(id) => {
            append_instruction_code(buffer, InstructionCode::GET_LOCAL_REF);
            buffer.extend_from_slice(id);
        }
        PointerAddress::Remote(id) => {
            append_instruction_code(buffer, InstructionCode::GET_REF);
            buffer.extend_from_slice(id);
        }
    }
}

pub fn append_key_value_pair(
    buffer: &mut Vec<u8>,
    key: &ValueContainer,
    value: &ValueContainer,
) {
    // insert key
    match key {
        // if text, append_key_string, else dynamic
        ValueContainer::Value(Value {
            inner: CoreValue::Text(text),
            ..
        }) => {
            append_key_string(buffer, &text.0);
        }
        _ => {
            append_instruction_code(buffer, InstructionCode::KEY_VALUE_DYNAMIC);
            append_value_container(buffer, key);
        }
    }
    // insert value
    append_value_container(buffer, value);
}

pub fn append_key_string(buffer: &mut Vec<u8>, key_string: &str) {
    let bytes = key_string.as_bytes();
    let len = bytes.len();

    if len < 256 {
        append_instruction_code(buffer, InstructionCode::KEY_VALUE_SHORT_TEXT);
        append_u8(buffer, len as u8);
        buffer.extend_from_slice(bytes);
    } else {
        append_instruction_code(buffer, InstructionCode::KEY_VALUE_DYNAMIC);
        append_text(buffer, key_string);
    }
}

pub fn append_instruction_code(buffer: &mut Vec<u8>, code: InstructionCode) {
    append_u8(buffer, code as u8);
}
