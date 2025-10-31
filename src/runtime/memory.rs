use crate::libs::core::{CoreLibPointerId, load_core_lib};
use crate::references::reference::Reference;
use crate::references::type_reference::TypeReference;
use crate::references::value_reference::ValueReference;
use crate::types::error::IllegalTypeError;
use crate::types::type_container::TypeContainer;
use crate::utils::time::Time;
use crate::values::pointer::PointerAddress;
use datex_core::global::protocol_structures::instructions::RawFullPointerAddress;
use datex_core::values::core_values::endpoint::Endpoint;
use core::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::Rc;
// FIXME #105 no-std

#[derive(Debug, Default)]
pub struct Memory {
    local_endpoint: Endpoint,
    local_counter: u64,  // counter for local pointer ids
    last_timestamp: u64, // last timestamp used for a new local pointer id
    pointers: HashMap<PointerAddress, Reference>, // all pointers
}

impl Memory {
    /// Creates a new, Memory instance with the core library loaded.
    pub fn new(endpoint: Endpoint) -> Memory {
        let mut memory = Memory {
            local_endpoint: endpoint,
            local_counter: 0,
            last_timestamp: 0,
            pointers: HashMap::new(),
        };
        // load core library
        load_core_lib(&mut memory);
        memory
    }

    /// Registers a new reference in memory. If the reference has no PointerAddress, a new local one is generated.
    /// If the reference is already registered (has a PointerAddress), the existing address is returned and no new registration is done.
    /// Returns the PointerAddress of the registered reference.
    pub fn register_reference(
        &mut self,
        reference: &Reference,
    ) -> PointerAddress {
        let pointer_address = reference.pointer_address();
        // check if reference is already registered (if it has an address, we assume it is registered)
        if let Some(ref address) = pointer_address
            && self.pointers.contains_key(address)
        {
            return address.clone();
        }
        // auto-generate new local id if no id is set
        let pointer_address = if let Some(address) = pointer_address {
            address
        } else {
            let pointer_address = self.get_new_local_address();
            reference.set_pointer_address(pointer_address.clone());
            pointer_address
        };

        self.pointers
            .insert(pointer_address.clone(), reference.clone());
        pointer_address
    }

    /// Returns a reference stored at the given PointerAddress, if it exists.
    pub fn get_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Option<&Reference> {
        self.pointers.get(pointer_address)
    }

    pub fn get_value_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Option<&Rc<RefCell<ValueReference>>> {
        self.get_reference(pointer_address).and_then(|r| match r {
            Reference::ValueReference(v) => Some(v),
            _ => None,
        })
    }

    pub fn get_type_reference(
        &self,
        pointer_address: &PointerAddress,
    ) -> Option<&Rc<RefCell<TypeReference>>> {
        self.get_reference(pointer_address).and_then(|r| match r {
            Reference::TypeReference(t) => Some(t),
            _ => None,
        })
    }

    /// Helper function to get a core value directly from memory
    pub fn get_core_reference(
        &self,
        pointer_id: CoreLibPointerId,
    ) -> &Reference {
        self.get_reference(&pointer_id.into())
            .expect("core reference not found in memory")
    }

    /// Helper function to get a core type directly from memory if it can be used as a type
    pub fn get_core_type(
        &self,
        pointer_id: CoreLibPointerId,
    ) -> Result<TypeContainer, IllegalTypeError> {
        let reference = self
            .get_reference(&pointer_id.into())
            .ok_or(IllegalTypeError::TypeNotFound)?;
        match &reference {
            Reference::TypeReference(def) => {
                Ok(TypeContainer::TypeReference(def.clone()))
            }
            _ => Err(IllegalTypeError::TypeNotFound),
        }
    }

    /// Helper function to get a core type directly from memory, asserting that is can be used as a type
    /// Panics if the core type is not found or cannot be used as a type.
    pub fn get_core_type_unchecked(
        &self,
        pointer_id: CoreLibPointerId,
    ) -> TypeContainer {
        // FIXME #415: Mark as unchecked
        self.get_core_type(pointer_id)
            .expect("core type not found or cannot be used as a type")
    }

    /// Takes a RawFullPointerAddress and converts it to a PointerAddress::Local or PointerAddress::Remote,
    /// depending on whether the pointer origin id matches the local endpoint.
    pub fn get_pointer_address_from_raw_full_address(
        &self,
        raw_address: RawFullPointerAddress,
    ) -> PointerAddress {
        if raw_address.endpoint == self.local_endpoint {
            PointerAddress::Local(raw_address.id)
        } else {
            // combine raw_address.endpoint and raw_address.id to [u8; 26]
            let writer = Cursor::new(Vec::new());
            let mut bytes = writer.into_inner();
            bytes.extend_from_slice(&raw_address.id);
            PointerAddress::Remote(<[u8; 26]>::try_from(bytes).unwrap())
        }
    }

    /// Creates a new unique local PointerAddress.
    pub fn get_new_local_address(&mut self) -> PointerAddress {
        let timestamp = Time::now();
        // new timestamp, reset counter
        if timestamp != self.last_timestamp {
            self.last_timestamp = timestamp;
            self.local_counter = 0;
        }
        // same timestamp as last time, increment counter to prevent collision
        else {
            self.local_counter += 1;
        }
        self.local_counter += 1;

        // create id: 4 bytes timestamp + 1 byte counter
        let id: [u8; 5] = [
            (timestamp >> 24) as u8,
            (timestamp >> 16) as u8,
            (timestamp >> 8) as u8,
            timestamp as u8,
            (self.local_counter & 0xFF) as u8,
        ];
        PointerAddress::Local(id)
    }
}

impl Reference {
    /// Returns the PointerAddress of this reference, if it has one.
    /// Otherwise, it registers the reference in the given memory and returns the newly assigned PointerAddress.
    pub fn ensure_pointer_address(
        &self,
        memory: &RefCell<Memory>,
    ) -> PointerAddress {
        self.pointer_address()
            .unwrap_or_else(|| memory.borrow_mut().register_reference(self))
    }
}
