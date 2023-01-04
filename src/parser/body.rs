use std::cell::Cell;
use std::fmt;

use crate::global::binary_codes::BinaryCode;
use crate::datex_values::{PrimitiveValue, SlotIdentifier, Type, Value, internal_slot, Quantity, Time, Pointer, Endpoint, Url};
use crate::utils::buffers;
use gen_iter::gen_iter;
use num_bigint::BigUint;


fn extract_slot_identifier(dxb_body:&[u8], index: &mut usize) -> SlotIdentifier {
	let length = buffers::read_u8(dxb_body, index);
	// binary name (2 byte number) TODO: length no longer required
	if length == 0 {
		let index = buffers::read_u16(dxb_body, index);
		return SlotIdentifier::new(index);
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
	let mut _has_parameters = false; // TODO:get params

	if is_extended {
		variation_length = buffers::read_u8(dxb_body,  index);
		_has_parameters = if buffers::read_u8(dxb_body,index) == 0 {false} else {true};
	}

	let namespace = buffers::read_string_utf8(dxb_body, index, namespace_length as usize);
	let name = buffers::read_string_utf8(dxb_body, index, name_length as usize);
	let mut variation: Option<String> = None;

	if is_extended && variation_length!=0 {
		variation = Some(buffers::read_string_utf8(dxb_body, index, variation_length as usize));
	};

	Type { 
		namespace, 
		name, 
		variation
	}
}

fn extract_endpoint(dxb_body:&[u8], index: &mut usize, endpoint_type:BinaryCode) -> Endpoint {

	match endpoint_type {
		BinaryCode::ENDPOINT => (),
		BinaryCode::ENDPOINT_WILDCARD => (),
		BinaryCode::PERSON_ALIAS => (),
		BinaryCode::PERSON_ALIAS_WILDCARD => (),
		BinaryCode::INSTITUTION_ALIAS => (),
		BinaryCode::INSTITUTION_ALIAS_WILDCARD => (),
		_ => panic!("invalid endpoint extraction") // should never happen
	}

	let name_is_binary = endpoint_type == BinaryCode::ENDPOINT || endpoint_type == BinaryCode::ENDPOINT_WILDCARD;

	let mut instance:&str = "";

	let name_length = buffers::read_u8(dxb_body, index); // get name length
	let subspace_number = buffers::read_u8(dxb_body, index); // get subspace number
	let mut instance_length = buffers::read_u8(dxb_body, index); // get instance length

	if instance_length == 0 {instance = "*"}
	else if instance_length == 255 {instance_length = 0};

	// get name
	let mut name:String = "".to_string();
	let mut name_binary:Vec<u8> = vec![];
	if name_is_binary {
		name_binary = buffers::read_slice(dxb_body, index, name_length as usize);
	}
	else {
		name = buffers::read_string_utf8(dxb_body, index, name_length as usize);
	}  

	let mut subspaces:Vec<String> = vec![];

	for _n in 0..subspace_number {
		let length = buffers::read_u8(dxb_body, index);
		if length == 0 {
			subspaces.push("*".to_string());
		}
		else {
			let subspace_name = buffers::read_string_utf8(dxb_body, index, length as usize);
			subspaces.push(subspace_name.to_string());
		}
	}
	
	if instance_length !=0 && instance.len()==0 {
		instance = &buffers::read_string_utf8(dxb_body, index, instance_length as usize);  // get instance
	}
	
	// TODO: new instance format, without length
	return if endpoint_type == BinaryCode::PERSON_ALIAS {
		Endpoint::new_person_alias(&name, Endpoint::ANY_INSTANCE, Some(subspaces))
	}
	else if endpoint_type == BinaryCode::INSTITUTION_ALIAS {
		Endpoint::new_institution_alias(&name, Endpoint::ANY_INSTANCE, Some(subspaces))
	}
	else if endpoint_type == BinaryCode::ENDPOINT {
		Endpoint::new(&name_binary, Endpoint::ANY_INSTANCE, Some(subspaces))
	}

	// should never get here
	else {
		Endpoint::new(&name_binary, Endpoint::ANY_INSTANCE, Some(subspaces))
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
				yield Instruction {code:BinaryCode::INT_8, slot: None, primitive_value: Some(PrimitiveValue::Int64(value as i64)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INT_16 as u8 {
				let value = buffers::read_i16(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INT_16, slot: None, primitive_value: Some(PrimitiveValue::Int64(value as i64)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INT_32 as u8 {
				let value = buffers::read_i32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INT_32, slot: None, primitive_value: Some(PrimitiveValue::Int64(value as i64)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INT_64 as u8 {
				let value = buffers::read_i64(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INT_64, slot: None, primitive_value: Some(PrimitiveValue::Int64(value)), value:None, subscope_continue:false}
			}

			// decimals
			else if token == BinaryCode::FLOAT_64 as u8 {
				let value = buffers::read_f64(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::FLOAT_64, slot: None, primitive_value: Some(PrimitiveValue::Float64(value)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::FLOAT_AS_INT as u8 {
				let value = buffers::read_i32(&dxb_body, index) as f64;
				_index.set(*index);
				yield Instruction {code:BinaryCode::FLOAT_64, slot: None, primitive_value: Some(PrimitiveValue::Float64(value)), value:None, subscope_continue:false}
			}

			// text
			else if token == BinaryCode::TEXT as u8 {
				let size = buffers::read_u32(&dxb_body, index);
				let value = buffers::read_string_utf8(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::TEXT, slot: None, primitive_value: Some(PrimitiveValue::Text(value)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::SHORT_TEXT as u8 {
				let size = buffers::read_u8(&dxb_body, index);
				let value = buffers::read_string_utf8(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::SHORT_TEXT, slot: None, primitive_value: Some(PrimitiveValue::Text(value)), value:None, subscope_continue:false}
			}

			// buffer
			else if token == BinaryCode::BUFFER as u8 {
				let size = buffers::read_u32(&dxb_body, index);
				let value = buffers::read_slice(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::BUFFER, slot: None, primitive_value: Some(PrimitiveValue::Buffer(value)), value:None, subscope_continue:false}
			}
			
			// time
			else if token == BinaryCode::TIME as u8 {
				let ms = buffers::read_u64(&dxb_body, index);
				let time = Time::from_milliseconds(ms);
				_index.set(*index);
				yield Instruction {code:BinaryCode::TIME, slot: None, primitive_value: Some(PrimitiveValue::Time(time)), value:None, subscope_continue:false}
			}

			// url
			else if token == BinaryCode::URL as u8 {
				let length = buffers::read_u32(&dxb_body, index);
				let url_string = buffers::read_string_utf8(&dxb_body, index, length as usize);
				let url = Url::new(url_string);
				_index.set(*index);
				yield Instruction {code:BinaryCode::URL, slot: None, primitive_value: Some(PrimitiveValue::Url(url)), value:None, subscope_continue:false}
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
				yield Instruction {code:BinaryCode::QUANTITY, slot: None, primitive_value: Some(PrimitiveValue::Quantity(quantity)), value:None, subscope_continue:false}
			}

			// logical
			else if token == BinaryCode::CONJUNCTION as u8 {
				let count = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::CONJUNCTION, slot:None, primitive_value: Some(PrimitiveValue::UInt32(count)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::DISJUNCTION as u8 {
				let count = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::DISJUNCTION, slot:None, primitive_value: Some(PrimitiveValue::UInt32(count)), value:None, subscope_continue:false}
			}

			// constant primitives
			else if token == BinaryCode::TRUE as u8 {
				yield Instruction {code:BinaryCode::TRUE, slot: None, primitive_value: Some(PrimitiveValue::Boolean(true)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::FALSE as u8 {
				yield Instruction {code:BinaryCode::FALSE, slot: None, primitive_value: Some(PrimitiveValue::Boolean(false)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::NULL as u8 {
				yield Instruction {code:BinaryCode::NULL, slot: None, primitive_value: Some(PrimitiveValue::Null), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VOID as u8 {
				yield Instruction {code:BinaryCode::VOID, slot: None, primitive_value: Some(PrimitiveValue::Void), value:None, subscope_continue:false}
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
				let jmp_index = buffers::read_u32(dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INIT_INTERNAL_VAR, slot: Some(slot), primitive_value: Some(PrimitiveValue::UInt32(jmp_index)), value:None, subscope_continue:false}
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
			else if token == BinaryCode::VAR_SENDER as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::FROM), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_CURRENT as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::ENDPOINT), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_LOCATION as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::LOCATION), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_META as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::META), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_ENV as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::ENV), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_RESULT as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::RESULT), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::SET_VAR_RESULT as u8 {
				yield Instruction {code:BinaryCode::SET_INTERNAL_VAR, slot: Some(internal_slot::RESULT), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::VAR_SUB_RESULT as u8 {
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Some(internal_slot::SUB_RESULT), primitive_value: None, value:None, subscope_continue:false}
			}
			else if token == BinaryCode::SET_VAR_SUB_RESULT as u8 {
				yield Instruction {code:BinaryCode::SET_INTERNAL_VAR, slot: Some(internal_slot::SUB_RESULT), primitive_value: None, value:None, subscope_continue:false}
			}

			// jmp instructions
			else if token == BinaryCode::JMP as u8 {
				let jmp_index = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::JMP, slot:None, primitive_value: Some(PrimitiveValue::UInt32(jmp_index)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::JFA as u8 {
				let jmp_index = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::JFA, slot:None, primitive_value: Some(PrimitiveValue::UInt32(jmp_index)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::JTR as u8 {
				let jmp_index = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::JTR, slot:None, primitive_value: Some(PrimitiveValue::UInt32(jmp_index)), value:None, subscope_continue:false}
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
				yield Instruction {code:BinaryCode::CHILD_ACTION, slot: None, primitive_value: Some(PrimitiveValue::Uint8(code)), value:None, subscope_continue:false}
			}

			// objects, tuples, arrays
			else if token == BinaryCode::ELEMENT_WITH_KEY as u8 {
				let size = buffers::read_u8(&dxb_body, index);
				let key = buffers::read_string_utf8(&dxb_body, index, size as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::ELEMENT_WITH_KEY, slot: None, primitive_value: Some(PrimitiveValue::Text(key)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::ELEMENT_WITH_INT_KEY as u8 {
				let key = buffers::read_u32(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::ELEMENT_WITH_INT_KEY, slot: None, primitive_value: Some(PrimitiveValue::UInt32(key)), value:None, subscope_continue:false}
			}
			else if token == BinaryCode::INTERNAL_OBJECT_SLOT as u8 {
				let key = buffers::read_u16(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INTERNAL_OBJECT_SLOT, slot: None, primitive_value: Some(PrimitiveValue::UInt16(key)), value:None, subscope_continue:false}
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

			// resolve relative path
			else if token == BinaryCode::RESOLVE_RELATIVE_PATH as u8 {
				let length = buffers::read_u32(&dxb_body, index);
				let path = buffers::read_string_utf8(&dxb_body, index, length as usize);
				_index.set(*index);
				yield Instruction {code:BinaryCode::RESOLVE_RELATIVE_PATH, slot: None, primitive_value: Some(PrimitiveValue::Text(path)), value:None, subscope_continue:false}
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
				else if token == BinaryCode::STD_TYPE_ITERATOR as u8 {
				yield Instruction {code:BinaryCode::TYPE, slot: None, primitive_value: None, value:Some(Box::new(Type {namespace:"".to_string(), name:"Iterator".to_string(), variation:None})), subscope_continue:false}
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


			// endpoints
			else if token == BinaryCode::PERSON_ALIAS as u8 {
				let endpoint = extract_endpoint(&dxb_body, index, BinaryCode::PERSON_ALIAS);
				_index.set(*index);
				yield Instruction {code:BinaryCode::PERSON_ALIAS, slot: None, primitive_value: Some(PrimitiveValue::Endpoint(endpoint)), value:None, subscope_continue: false}
			}
			else if token == BinaryCode::INSTITUTION_ALIAS as u8 {
				let endpoint = extract_endpoint(&dxb_body, index, BinaryCode::INSTITUTION_ALIAS);
				_index.set(*index);
				yield Instruction {code:BinaryCode::INSTITUTION_ALIAS, slot: None, primitive_value: Some(PrimitiveValue::Endpoint(endpoint)), value:None, subscope_continue: false}
			}
			else if token == BinaryCode::ENDPOINT as u8 {
				let endpoint = extract_endpoint(&dxb_body, index, BinaryCode::ENDPOINT);
				_index.set(*index);
				yield Instruction {code:BinaryCode::ENDPOINT, slot: None, primitive_value: Some(PrimitiveValue::Endpoint(endpoint)), value:None, subscope_continue: false}
			}

			// scope (used in combination with do, function, run, ...)

			else if token == BinaryCode::SCOPE_BLOCK_START as u8 {
				let scope = extract_scope(&dxb_body, index);
				_index.set(*index);
				yield Instruction {code:BinaryCode::SCOPE_BLOCK_START, slot: None, primitive_value: Some(PrimitiveValue::Buffer(scope)), value:None, subscope_continue: false}
			}




			// default
			else {
				yield Instruction {code:BinaryCode::try_from(token).expect("Could not parse DXB, invalid instruction"), slot: None, primitive_value:None, value:None, subscope_continue:false}
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
