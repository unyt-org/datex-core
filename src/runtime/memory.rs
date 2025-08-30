use std::collections::HashMap;
use std::io::Cursor;
use binrw::BinWrite;
use chumsky::primitive::End;
use log::info;
use datex_core::global::protocol_structures::instructions::RawFullPointerAddress;
use datex_core::runtime::global_context::get_global_context;
use datex_core::values::core_values::endpoint::Endpoint;
use crate::global::protocol_structures::instructions::RawInternalPointerAddress;
use crate::libs::core::load_core_lib;
use crate::values::pointer::PointerAddress;
use crate::values::reference::Reference;
// FIXME #105 no-std

#[derive(Debug)]
pub struct Memory {
    local_endpoint: Endpoint,
    local_counter: u64, // counter for local pointer ids
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


    pub fn register_reference(&mut self, reference: Reference) {
        // auto-generate new local id if no id is set
        let pointer_id = reference.data.borrow().pointer_id().clone()
            .unwrap_or_else(|| self.get_new_local_address());
        self.pointers.insert(pointer_id, reference);
    }

    pub fn get_reference(&self, pointer_address: &PointerAddress) -> Option<&Reference> {
        self.pointers.get(pointer_address)
    }

    /// Takes a RawFullPointerAddress and converts it to a PointerAddress::Local or PointerAddress::Remote,
    /// depending on whether the pointer origin id matches the local endpoint.
    pub fn get_pointer_address_from_raw_full_address(&self, raw_address: RawFullPointerAddress) -> PointerAddress {
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
        let timestamp = get_global_context().time.lock().unwrap().now(); // TODO: better way to get current time?
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
