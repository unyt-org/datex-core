use std::collections::HashMap;
use crate::values::reference::Reference;
// FIXME no-std

pub struct Memory {
    pointers: HashMap<[u8; 26], Reference>, // all pointers
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            pointers: HashMap::new(),
        }
    }

    pub fn get_pointer_by_id(
        &mut self,
        address: [u8; 26],
    ) -> Option<&mut Reference> {
        self.pointers.get_mut(&address)
    }

    pub fn get_pointer_by_id_vec(
        &mut self,
        address: Vec<u8>,
    ) -> Option<&mut Reference> {
        let mut address_array: [u8; 26] = [0; 26];
        for i in 0..26 {
            address_array[i] = address[i];
        }
        self.get_pointer_by_id(address_array)
    }

    pub fn get_pointer_ids(&self) -> Vec<[u8; 26]> {
        let mut ids: Vec<[u8; 26]> = Vec::new();
        for id in self.pointers.keys() {
            ids.push(*id);
        }
        ids
    }

    pub fn store_pointer(&mut self, address: [u8; 26], reference: Reference) {
        self.pointers.insert(address, reference);
    }
}
