use std::collections::HashMap;

use crate::datex_values::Pointer;

pub struct Memory {
	pointers: HashMap<[i8; 26], Pointer>, // all pointers
}

impl Memory {
	pub fn new() -> Memory{
		Memory {
			pointers: HashMap::new()
		}
	}

	pub fn get_pointer_by_id(&mut self, address: [i8; 26]) -> Option<&mut Pointer> {
		self.pointers.get_mut(&address)
	}

	pub fn store_pointer(&mut self, address: [i8; 26], pointer: Pointer) {
		self.pointers.insert(address, pointer);
	}
}