use std::cell::Cell;
use std::fmt;

use crate::global::binary_codes::BinaryCode;
use crate::datex_values::{PrimitiveValue, SlotIdentifier, Type, Value, internal_slot, Quantity, Pointer};
use crate::utils::buffers;
use gen_iter::gen_iter;
use num_bigint::BigUint;


fn extract_slot_identifier(dxb_body:&[u8], index: &mut usize) -> SlotIdentifier {
	let length = buffers::read_u8(dxb_body, index);
	// binary name (2 byte number) TODO: length no longer required
	if length == 0 {
		let index = buffers::read_u16(dxb_body, index);
		return SlotIdentifier {index};
	}
	// string name TODO: deprecated
	else {
		let _name = buffers::read_string_utf8(dxb_body, index, length as usize);
		return SlotIdentifier::default();
	}
}

fn extract_scope(dxb_body:&[u8], index: &mut usize) -> Vec<u8> {
	let size = buffers::read_u32(&dxb_body, index);
	return buffers::read_slice(&dxb_body, index, size as usize);
}

fn extract_type<'a>(dxb_body:&'a [u8], index: &'a mut usize, is_extended: bool) -> Type {
	let namespace_length = buffers::read_u8(dxb_body,index);
	let name_length = buffers::read_u8(dxb_body, index);
	let mut variation_length = 0;
	let mut has_parameters = false; // TODO:get params

	if is_extended {
		variation_length = buffers::read_u8(dxb_body,  index);
		has_parameters = if buffers::read_u8(dxb_body,index) == 0 {false} else {true};
	}

	let namespace = buffers::read_string_utf8(dxb_body, index, namespace_length as usize);
	let name = buffers::read_string_utf8(dxb_body, index, name_length as usize);
	let mut variation: Option<String> = None;

	if is_extended {
		variation = Some(buffers::read_string_utf8(dxb_body, index, variation_length as usize));
	};

	Type { 
		namespace, 
		name, 
		variation
	}
}

pub fn iterate_instructions<'a>(dxb_body:&'a[u8], mut _index: &'a Cell<usize>) -> impl Iterator<Item = Instruction>  + 'a {

	return gen_iter!(move {

		let max = dxb_body.len();

		// iterate over bytes
		while _index.get() < max {
			let index = &mut _index.get();
			let token = buffers::read_u8(dxb_body, index);
			_index.set(*index);

			// integers
			if token == BinaryCode::INT_8 as u8 {
				let value = buffers::read_i8(&dxb_body, index);
				_index.set(*index); // TODO better way
				yield Instruction {code:BinaryCode::INT_8, slot: None, primitive_value: Some(PrimitiveValue::INT_64(value as i64)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INT_16 as u8 {
				let value = buffers::read_i16(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INT_16, slot: None, primitive_value: Some(PrimitiveValue::INT_64(value as i64)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INT_32 as u8 {
				let value = buffers::read_i32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INT_32, slot: None, primitive_value: Some(PrimitiveValue::INT_64(value as i64)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INT_64 as u8 {
				let value = buffers::read_i64(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INT_64, slot: None, primitive_value: Some(PrimitiveValue::INT_64(value)), value:None, subscope_continue:false}
			}

			// decimals
			else if token == BinaryCode::FLOAT_64 as u8 {
				let value = buffers::read_f64(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::FLOAT_64, slot: None, primitive_value: Some(PrimitiveValue::FLOAT_64(value)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::FLOAT_AS_INT as u8 {
				let value = buffers::read_i32(&dxb_body, index) as f64;
				_index.set(*index);
				yield Instruction {code:BinaryCode::FLOAT_64, slot: None, primitive_value: Some(PrimitiveValue::FLOAT_64(value)), value:None, subscope_continue:false}
			}

			// text
			else if token == BinaryCode::TEXT as u8 {
				let size = buffers::read_u32(&dxb_body, index);
				let value = buffers::read_string_utf8(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::TEXT, slot: None, primitive_value: Some(PrimitiveValue::TEXT(value)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::SHORT_TEXT as u8 {
				let size = buffers::read_u8(&dxb_body, index);
				let value = buffers::read_string_utf8(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::SHORT_TEXT, slot: None, primitive_value: Some(PrimitiveValue::TEXT(value)), value:None, subscope_continue:false}
			}

			// buffer
			else if token == BinaryCode::BUFFER as u8 {
				let size = buffers::read_u32(&dxb_body, index);
				let value = buffers::read_slice(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::BUFFER, slot: None, primitive_value: Some(PrimitiveValue::BUFFER(value)), value:None, subscope_continue:false}
			}


			// quantity
			else if token == BinaryCode::QUANTITY as u8 {
				let sign = if buffers::read_u8(&dxb_body, index) == 0 {false} else {true};

				let num_size = buffers::read_u16(&dxb_body, index);
				let den_size = buffers::read_u16(&dxb_body, index);
				let num_buffer = buffers::read_slice(&dxb_body, index, num_size as usize);
				let den_buffer = buffers::read_slice(&dxb_body, index, den_size as usize);

				let factor_count = buffers::read_u8(&dxb_body, index);
				let mut unit:Vec<(u8, i8)> = Vec::new();
				for _i in 0..factor_count {
					let code = buffers::read_u8(&dxb_body, index);
					let exponent = buffers::read_i8(&dxb_body, index);
					unit.push((code, exponent));
				}

				let quantity = Quantity {
					sign, 
					unit,
					numerator:BigUint::from_bytes_le(&num_buffer), 
					denominator:BigUint::from_bytes_le(&den_buffer),
				};
				_index.set(*index);
				yield Instruction {code:BinaryCode::QUANTITY, slot: None, primitive_value: Some(PrimitiveValue::QUANTITY(quantity)), value:None, subscope_continue:false}
			}


			// constant primitives
			else if token == BinaryCode::TRUE as u8 {
				_index.set(*index);
				yield Instruction {code:BinaryCode::TRUE, slot: None, primitive_value: Some(PrimitiveValue::BOOLEAN(true)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::FALSE as u8 {
				_index.set(*index);
				yield Instruction {code:BinaryCode::FALSE, slot: None, primitive_value: Some(PrimitiveValue::BOOLEAN(false)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::NULL as u8 {
				_index.set(*index);
				yield Instruction {code:BinaryCode::NULL, slot: None, primitive_value: Some(PrimitiveValue::NULL), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VOID as u8 {
				_index.set(*index);
				yield Instruction {code:BinaryCode::VOID, slot: None, primitive_value: Some(PrimitiveValue::VOID), value:None, subscope_continue:false}
			}


			// slot instructions
			else if token == BinaryCode::SET_INTERNAL_VAR_REFERENCE as u8 {
				let slot = extract_slot_identifier(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::SET_INTERNAL_VAR_REFERENCE, slot: Some(slot), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::SET_INTERNAL_VAR as u8 {
				let slot = extract_slot_identifier(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::SET_INTERNAL_VAR, slot: Some(slot), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INIT_INTERNAL_VAR as u8 {
				let slot = extract_slot_identifier(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INIT_INTERNAL_VAR, slot: Some(slot), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INTERNAL_VAR as u8 {
				let slot = extract_slot_identifier(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(slot), primitive_value: None, value:None, subscope_continue:false}
			}


			// internal slots
			else if token == BinaryCode::VAR_PUBLIC as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::PUBLIC), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_THIS as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::THIS), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_IT as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::IT), primitive_value: None, value:None, subscope_continue:false}
			}

			// pointer
			else if token == BinaryCode::POINTER as u8 {
				let id = buffers::read_slice(&dxb_body, index, Pointer::MAX_POINTER_ID_SIZE);
				_index.set(*index);
				let pointer = Pointer::from_id(id);
				yield Instruction {code:BinaryCode::POINTER, slot: None, primitive_value: None, value:Some(Box::new(pointer)), subscope_continue:false}
			}

			// actions
			else if token == BinaryCode::CHILD_ACTION as u8 {
				let code = buffers::read_u8(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::CHILD_ACTION, slot: None, primitive_value: Some(PrimitiveValue::UINT_8(code)), value:None, subscope_continue:false}
			}

			// objects, tuples, arrays
			else if token == BinaryCode::ELEMENT_WITH_KEY as u8 {
				let size = buffers::read_u8(&dxb_body, index);
				let key = buffers::read_string_utf8(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::ELEMENT_WITH_KEY, slot: None, primitive_value: Some(PrimitiveValue::TEXT(key)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::ELEMENT_WITH_INT_KEY as u8 {
				let key = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::ELEMENT_WITH_INT_KEY, slot: None, primitive_value: Some(PrimitiveValue::UINT_32(key)), value:None, subscope_continue:false}
			}

			// types
			else if token == BinaryCode::TYPE as u8 {
				let dx_type = extract_type(&dxb_body, index, false);
				_index.set(*index);
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(dx_type)), subscope_continue:false}
			}
			else if token == BinaryCode::EXTENDED_TYPE as u8 {
				let dx_type = extract_type(&dxb_body, index, true);
				_index.set(*index);
				yield Instruction {code:BinaryCode::EXTENDED_TYPE, slot: None, primitive_value: None, value:Some(Box::new(dx_type)), subscope_continue:false}
			}

			// std types
			else if token == BinaryCode::STD_TYPE_SET as u8 {
				// TODO : Some(Box::new(std_types::SET))
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"Set".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_MAP as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"Map".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_TEXT as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"text".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_ARRAY as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"Array".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_OBJECT as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"Object".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_INT as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"integer".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_FLOAT as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"decimal".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_TIME as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"time".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_NULL as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"null".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_VOID as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"void".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_UNIT as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"quantity".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_BUFFER as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"buffer".to_string(), variation:None})), subscope_continue:false}
			}
			else if token == BinaryCode::STD_TYPE_FUNCTION as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"Function".to_string(), variation:None})), subscope_continue:false}
			}

			// commands
			else if token == BinaryCode::COPY as u8 {
				yield Instruction {code:BinaryCode::COPY, slot: None, primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::CLONE as u8 {
				yield Instruction {code:BinaryCode::CLONE, slot: None, primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::CREATE_POINTER as u8 {
				yield Instruction {code:BinaryCode::CREATE_POINTER, slot: None, primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::RUN as u8 {
				yield Instruction {code:BinaryCode::RUN, slot: None, primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::AWAIT as u8 {
				yield Instruction {code:BinaryCode::AWAIT, slot: None, primitive_value: None, value:None, subscope_continue:false}
			}

			// subscopes
			else if token == BinaryCode::ARRAY_START as u8 {
				yield Instruction {code:BinaryCode::ARRAY_START, slot: None, primitive_value: None, value:None, subscope_continue: true}
			}
			else if token == BinaryCode::ARRAY_END as u8 {
				yield Instruction {code:BinaryCode::ARRAY_END, slot: None, primitive_value: None, value:None, subscope_continue:false};
				break;
			}

			else if token == BinaryCode::OBJECT_START as u8 {
				yield Instruction {code:BinaryCode::OBJECT_START, slot: None, primitive_value: None, value:None, subscope_continue: true}
			}
			else if token == BinaryCode::OBJECT_END as u8 {
				yield Instruction {code:BinaryCode::OBJECT_END, slot: None, primitive_value: None, value:None, subscope_continue:false};
				break;
			}

			else if token == BinaryCode::TUPLE_START as u8 {
				yield Instruction {code:BinaryCode::TUPLE_START, slot: None, primitive_value: None, value:None, subscope_continue: true}
			}
			else if token == BinaryCode::TUPLE_END as u8 {
				yield Instruction {code:BinaryCode::TUPLE_END, slot: None, primitive_value: None, value:None, subscope_continue:false};
				break;
			}

			else if token == BinaryCode::SUBSCOPE_START as u8 {
				yield Instruction {code:BinaryCode::SUBSCOPE_START, slot: None, primitive_value: None, value:None, subscope_continue: true}
			}
			else if token == BinaryCode::SUBSCOPE_END as u8 {
				yield Instruction {code:BinaryCode::SUBSCOPE_END, slot: None, primitive_value: None, value:None, subscope_continue:false};
				break;
			}

			// scope (used in combination with do, function, run, ...)

			else if token == BinaryCode::SCOPE_BLOCK_START as u8 {
				let scope = extract_scope(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::SCOPE_BLOCK_START, slot: None, primitive_value: Some(PrimitiveValue::BUFFER(scope)), value:None, subscope_continue: false}
			}




			// default
			else {
				yield Instruction {code:BinaryCode::try_from(token).expect("enum conversion error"), slot: None, primitive_value:None, value:None, subscope_continue:false}
			}

		}

	});
}


pub struct Instruction {
	pub code: BinaryCode,
	pub slot: Option<SlotIdentifier>,
	pub primitive_value: Option<PrimitiveValue>,
	pub value: Option<Box<dyn Value>>,
	pub subscope_continue: bool
}

impl Instruction {
    pub fn to_string(&self) -> String {
		if self.primitive_value.is_some() && self.value.is_some() {return format!("{} [{:X}] {} {}", self.code, self.code as u8, self.primitive_value.as_ref().unwrap(), self.value.as_ref().unwrap())}
		else if self.primitive_value.is_some() {return format!("{} [{:X}] {}", self.code, self.code as u8, self.primitive_value.as_ref().unwrap())}
		else if self.value.is_some() {return format!("{} [{:X}] {}", self.code, self.code as u8, self.value.as_ref().unwrap())}
		else {return format!("{} [{:X}]", self.code, self.code as u8,)}
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Instruction::to_string(self))
    }
}
