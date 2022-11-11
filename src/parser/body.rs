use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

use crate::{Logger, datex_values};
use crate::global::binary_codes::BinaryCode;
use crate::datex_values::{PrimitiveValue, SlotIdentifier};
use crate::utils::buffers;


fn extract_slot_identifier(dxb_body:&[u8], index: &mut usize) -> SlotIdentifier {
	let length = buffers::read_u8(dxb_body, index);
	// binary name (2 byte number)
	if length == 0 {
		let index = buffers::read_u16(dxb_body, index);
		return SlotIdentifier::ID(index)
	}
	// string name
	else {
		let name = buffers::read_string_utf8(dxb_body, index, length as usize);
		return SlotIdentifier::NAME(name)
	}
}


pub fn parse_loop(dxb_body:&[u8]) -> GeneratorIteratorAdapter<impl Generator<Yield = Instruction, Return = ()> + '_> {

	return GeneratorIteratorAdapter::new(|| {
		let logger:Logger = Logger::new("DATEX WASM Parser");

		let mut index = 0;
		let max = dxb_body.len();

		// iterate over bytes
		while index < max {
			let token = buffers::read_u8(dxb_body, &mut index);

			logger.info(&format!("token: {token}"));

			// integers
			if token == BinaryCode::INT_8 as u8 {
				let value = buffers::read_i8(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::INT_8, slot: None, primitive_value: Option::Some(PrimitiveValue::INT_8(value))}
			}
			else if token == BinaryCode::INT_16 as u8 {
				let value = buffers::read_i16(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::INT_16, slot: None, primitive_value: Option::Some(PrimitiveValue::INT_16(value))}
			}
			else if token == BinaryCode::INT_32 as u8 {
				let value = buffers::read_i32(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::INT_32, slot: None, primitive_value: Option::Some(PrimitiveValue::INT_32(value))}
			}
			else if token == BinaryCode::INT_64 as u8 {
				let value = buffers::read_i64(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::INT_64, slot: None, primitive_value: Option::Some(PrimitiveValue::INT_64(value))}
			}

			// decimals
			else if token == BinaryCode::FLOAT_64 as u8 {
				let value = buffers::read_f64(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::FLOAT_64, slot: None, primitive_value: Option::Some(PrimitiveValue::FLOAT_64(value))}
			}

			// text
			else if token == BinaryCode::TEXT as u8 {
				let size:usize= buffers::read_u32(dxb_body, &mut index).try_into().unwrap();
				let value = buffers::read_string_utf8(dxb_body, &mut index, size);
				yield Instruction {code:BinaryCode::TEXT, slot: None, primitive_value: Option::Some(PrimitiveValue::TEXT(value))}
			}
			else if token == BinaryCode::SHORT_TEXT as u8 {
				let size:usize= buffers::read_u8(dxb_body, &mut index).try_into().unwrap();
				let value = buffers::read_string_utf8(dxb_body, &mut index, size);
				yield Instruction {code:BinaryCode::SHORT_TEXT, slot: None, primitive_value: Option::Some(PrimitiveValue::TEXT(value))}
			}

			// buffer
			else if token == BinaryCode::BUFFER as u8 {
				let size:usize= buffers::read_u32(dxb_body, &mut index).try_into().unwrap();
				let value = buffers::read_slice(dxb_body, &mut index, size);
				yield Instruction {code:BinaryCode::BUFFER, slot: None, primitive_value: Option::Some(PrimitiveValue::BUFFER(value))}
			}

			// constant primitives
			else if token == BinaryCode::TRUE as u8 {
				yield Instruction {code:BinaryCode::TRUE, slot: None, primitive_value: Option::Some(PrimitiveValue::BOOLEAN(true))}
			}
			else if token == BinaryCode::FALSE as u8 {
				yield Instruction {code:BinaryCode::FALSE, slot: None, primitive_value: Option::Some(PrimitiveValue::BOOLEAN(false))}
			}
			else if token == BinaryCode::NULL as u8 {
				yield Instruction {code:BinaryCode::NULL, slot: None, primitive_value: Option::Some(PrimitiveValue::NULL)}
			}
			else if token == BinaryCode::VOID as u8 {
				yield Instruction {code:BinaryCode::VOID, slot: None, primitive_value: Option::Some(PrimitiveValue::VOID)}
			}


			// slot instructions
			else if token == BinaryCode::SET_INTERNAL_VAR_REFERENCE as u8 {
				let slot = extract_slot_identifier(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::SET_INTERNAL_VAR_REFERENCE, slot: Option::Some(slot), primitive_value: None}
			}
			else if token == BinaryCode::SET_INTERNAL_VAR as u8 {
				let slot = extract_slot_identifier(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::SET_INTERNAL_VAR, slot: Option::Some(slot), primitive_value: None}
			}
			else if token == BinaryCode::INIT_INTERNAL_VAR as u8 {
				let slot = extract_slot_identifier(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::INIT_INTERNAL_VAR, slot: Option::Some(slot), primitive_value: None}
			}
			else if token == BinaryCode::INTERNAL_VAR as u8 {
				let slot = extract_slot_identifier(dxb_body, &mut index);
				yield Instruction {code:BinaryCode::INTERNAL_VAR, slot: Option::Some(slot), primitive_value: None}
			}


			// commands
			else if token == BinaryCode::COPY as u8 {
				yield Instruction {code:BinaryCode::COPY, slot: None, primitive_value: None}
			}
			else if token == BinaryCode::CLONE as u8 {
				yield Instruction {code:BinaryCode::CLONE, slot: None, primitive_value: None}
			}
			else if token == BinaryCode::CREATE_POINTER as u8 {
				yield Instruction {code:BinaryCode::CREATE_POINTER, slot: None, primitive_value: None}
			}
			else if token == BinaryCode::RUN as u8 {
				yield Instruction {code:BinaryCode::RUN, slot: None, primitive_value: None}
			}
			else if token == BinaryCode::AWAIT as u8 {
				yield Instruction {code:BinaryCode::AWAIT, slot: None, primitive_value: None}
			}

			else {
				yield Instruction {code:BinaryCode::try_from(token).expect("enum conversion error"), slot: None, primitive_value:None}
			}

		}

	});
}


pub struct Instruction {
	pub code: BinaryCode,
	pub slot: Option<SlotIdentifier>,
	pub primitive_value: Option<PrimitiveValue>
}



pub struct GeneratorIteratorAdapter<G>(Pin<Box<G>>);

impl<G> GeneratorIteratorAdapter<G>
where
    G: Generator<Return = ()>,
{
    fn new(gen: G) -> Self {
        Self(Box::pin(gen))
    }
}

impl<G> Iterator for GeneratorIteratorAdapter<G>
where
    G: Generator<Return = ()>,
{
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.as_mut().resume(()) {
            GeneratorState::Yielded(x) => Some(x),
            GeneratorState::Complete(_) => None,
        }
    }
}